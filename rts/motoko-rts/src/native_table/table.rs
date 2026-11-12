//! The table primitives compiled code calls.
//!
//! A table is named by its root block, the reference `Table::create` returns, and the store
//! records every root block it creates a table at. Bytes 3,968 to 4,031 of a root block are the
//! host's, and the crate never touches them: the first word holds the codec hash of the table's
//! layout. Every primitive checks that the reference is a root block the store recorded and that
//! the word matches before it opens the table. A reference to any other block traps, also one
//! whose bytes a program wrote, for example through a long Text in a row. Any fault of the crate
//! traps.
//!
//! Once `nt_evolve` has changed a table's row type, the first word holds the new codec hash with
//! `CHANGED` mixed in, and the second word the address of the table's map page (`Map`): the column
//! of each field, since a column keeps its id when fields are added or dropped. A call compares the
//! first word with its codec hash as before, and only when they differ looks for the mark and
//! reads the map; a call on a table whose row type never changed runs as it did before.

use core::cmp::Ordering;

use ::native_table::area;
use ::native_table::index::secondary::Start;
use ::native_table::mem::{put, put_u64, rd_u64};
use ::native_table::schema::{IndexDecl, SchemaView};
use ::native_table::table::filter::Filter;
use ::native_table::table::totals::Total;
use ::native_table::table::{CellIn, CellOut, RowSink, Table, Upsert, buffer, indexes, root};
use ::native_table::tree::Dir;
use ::native_table::value::ty;
use ::native_table::{Budget, Fault, PageHost, Region, Scratch, Wake};
use motoko_rts_macros::ic_mem_fn;

use super::layout::{
    Layout, Picks, SCHEMA_VERSION, array_type, evolved_image, merge, schema_image, scratch,
    view_layout,
};
use super::row::{
    CellRows, Cells, Map, Out, Places, RowOut, cells, column_type, compact, filter_cond,
    int_of_i128, int_of_twos, key, key_cell, key_cells, nat_of_u64, nat64_of, or_trap, pair,
    text_bytes, update_cells,
};
use super::store::StoreHost;
use crate::barriers::allocation_barrier;
use crate::memory::{Memory, alloc_array, alloc_blob};
use crate::persistence::native_store_location;
use crate::rts_trap_with;
use crate::text::text_compare;
use crate::types::{Bytes, NULL_POINTER, TAG_ARRAY_I, TAG_BLOB_B, TRUE_VALUE, Value};

/// The host word that holds the codec hash, with `CHANGED` mixed in (XOR) once the table's row
/// type has changed.
const CODEC_HASH: u64 = root::HOST_WORDS;
/// The host word that holds the address of the map page of a table whose row type has changed;
/// read only when the codec hash word says so.
const MAP_PAGE: u64 = root::HOST_WORDS + 8;
/// The mark mixed into the codec hash word of a table whose row type has changed.
const CHANGED: u64 = 0x6E74_5F65_766F_6C76;
/// The most rows one scan or walk call hands back.
const BATCH_LIMIT: u64 = 64;
/// The bytes of the buffers a walk's keys start in: three keys of up to 256 bytes.
const WALK_BUFFER: usize = 768;
/// The bytes of the buffer a sum, a minimum or a maximum past i128 starts in.
const SUM_BUFFER: usize = 32;
/// The W of every secondary index.
const INDEX_W: u16 = 16;
/// The most parts an index key has.
const INDEX_PARTS: usize = 7;
/// The highest secondary index id.
const INDEX_MAX: u8 = 63;

unsafe extern "C" {
    fn ic0_performance_counter(counter: u32) -> u64;
}

/// The instruction meter maintenance reads: performance counter 0. `maintain` starts no step once
/// the counter has reached `limit`.
struct Meter {
    limit: u64,
}

impl Budget for Meter {
    fn used(&self) -> u64 {
        unsafe { ic0_performance_counter(0) }
    }

    fn limit(&self) -> u64 {
        self.limit
    }
}

/// Scratch the crate asks for: each request a new Blob that lives until the next collection.
/// It shares the memory with the store host, and each allocates only inside its own calls.
struct BlobScratch<M: Memory> {
    mem: *mut M,
}

impl<M: Memory> Scratch for BlobScratch<M> {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault> {
        unsafe {
            let bytes = scratch::<u8, M>(&mut *self.mem, need);
            Ok(core::slice::from_raw_parts_mut(bytes, need))
        }
    }
}

/// The bytes of the Blob `v`, in place.
unsafe fn blob_bytes<'a>(v: Value) -> &'a [u8] {
    let blob = v.as_blob();
    core::slice::from_raw_parts(blob.payload_const(), blob.len().as_usize())
}

/// The layout of the codec `codec`, its header alone checked (`Layout::of`): a codec is a pair of
/// the layout, a Blob, and the hash list of the row type. Traps for a view's codec (`nt_view`).
unsafe fn layout_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::of(layout).unwrap_or_else(|| not_a_table(codec))
}

/// `layout_of`, every column entry checked (`Layout::parse`): for a layout a table or a view is
/// made from.
unsafe fn checked_layout_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::parse(layout).unwrap_or_else(|| not_a_table(codec))
}

/// Traps for a codec whose layout is not a table's: a view's layout names a table that a view
/// only reads, and any other is malformed.
#[cold]
#[inline(never)]
unsafe fn not_a_table(codec: Value) -> ! {
    if Layout::view(blob_bytes(codec.as_array().get(0))).is_some() {
        rts_trap_with("native table: not a table of this key and row type");
    }
    rts_trap_with("native table: malformed layout")
}

/// The layout of the table that the view's codec `codec` reads, its header alone checked, and the
/// fields of the view; traps when `codec` is not a view's.
unsafe fn view_of<'a>(codec: Value) -> (Layout<'a>, Picks<'a>) {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::view(layout).unwrap_or_else(|| rts_trap_with("native table: malformed layout"))
}

/// The layout of the codec `codec`, for a call that builds no row of its row type: for a view's
/// codec, the layout of the table it reads.
unsafe fn read_layout_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::of(layout).unwrap_or_else(|| viewed(codec))
}

/// The layout of the table that the view's codec `codec` reads, its header alone checked, as a
/// table's codec's is (`Layout::of`). Traps when `codec` is not a view's. Out of line, so a call
/// with a table's codec runs as before.
#[cold]
#[inline(never)]
unsafe fn viewed<'a>(codec: Value) -> Layout<'a> {
    view_of(codec).0
}

/// `viewed`, with the fields of the view.
#[cold]
#[inline(never)]
unsafe fn viewed_picks<'a>(codec: Value) -> (Layout<'a>, Option<Picks<'a>>) {
    let (layout, picks) = view_of(codec);
    (layout, Some(picks))
}

/// The layout of the codec `codec`, its header alone checked (`Layout::header`): for a view's
/// codec, the layout of the table it reads.
unsafe fn header_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::header(layout).unwrap_or_else(|| rts_trap_with("native table: malformed layout"))
}

/// The hash list of the row type in the codec `codec`.
unsafe fn labels_of(codec: Value) -> Value {
    codec.as_array().get(1)
}

/// Checks that the root block `id` is one the store recorded, of a table of `layout`. Returns the
/// map of the table's columns when its row type has changed, else `None`. Inlined, so a call
/// takes a table whose row type never changed on the path it took before tables could change.
#[inline(always)]
unsafe fn root_block<M: Memory>(host: &StoreHost<'_, M>, id: u64, layout: &Layout) -> Option<Map> {
    let Some(block) = host.root(id) else {
        rts_trap_with("native table: not a table of this key and row type");
    };
    if rd_u64(block, CODEC_HASH) != Ok(layout.hash) {
        return Some(changed(host, block, layout.hash));
    }
    None
}

/// The map of the table of root block `block`, whose codec hash word does not hold the codec hash
/// `hash`: it holds that hash with `CHANGED` mixed in when the table's row type has changed to the
/// one of `hash`. Traps otherwise.
#[cold]
#[inline(never)]
unsafe fn changed<M: Memory>(host: &StoreHost<'_, M>, block: &[u8], hash: u64) -> Map {
    if rd_u64(block, CODEC_HASH) != Ok(hash ^ CHANGED) {
        rts_trap_with("native table: not a table of this key and row type");
    }
    let page = or_trap(host.get(Region::Page(or_trap(rd_u64(block, MAP_PAGE)))));
    Map::of(page).unwrap_or_else(|| rts_trap_with(Fault::BadRef.text()))
}

/// The table whose root block is `id`, checked by `root_block`. Out of line and the only call of
/// `Table::open`, so the table is built in its caller's place, not copied there.
#[inline(never)]
unsafe fn table_at<'h, 'a, M: Memory>(
    host: &'h mut StoreHost<'a, M>,
    id: u64,
) -> Table<'h, StoreHost<'a, M>> {
    or_trap(Table::open(host, id))
}

/// The table whose root block is `id`, of `layout`, for a call that looks up no field.
#[inline(always)]
unsafe fn open<'h, 'a, M: Memory>(
    host: &'h mut StoreHost<'a, M>,
    id: u64,
    layout: &Layout,
) -> Table<'h, StoreHost<'a, M>> {
    root_block(host, id, layout);
    table_at(host, id)
}

/// Creates an empty table of the codec's key and row types with a secondary index on each list of
/// the array `indexes`, an array of pairs of an array of Texts that name fields of the row, in key
/// order, and whether the index is unique. Returns its root block and the id of each list's
/// index, from 1, in a pair; equal lists share one index, unique when one of them is.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_create<M: Memory>(mem: &mut M, codec: Value, indexes: Value) -> Value {
    let layout = checked_layout_of(codec);
    let lists = indexes.as_array();
    let ids = scratch::<Value, M>(mem, lists.len());
    let mut decls = [IndexDecl::default(); INDEX_MAX as usize];
    let mut n = 0;
    for i in 0..lists.len() {
        let list = lists.get(i).as_array();
        let (fields, unique) = (list.get(0).as_array(), list.get(1).get_raw() == TRUE_VALUE);
        let parts = fields.len();
        if parts == 0 || parts > INDEX_PARTS {
            rts_trap_with("native table: an index has 1 to 7 fields");
        }
        let mut decl = IndexDecl {
            id: 0,
            flags: if unique { indexes::UNIQUE } else { 0 },
            w: INDEX_W,
            parts: parts as u8,
            part: [(0, false); INDEX_PARTS],
        };
        // The row's fields are the named columns after the key, each one leaf.
        for (p, part) in decl.part.iter_mut().take(parts).enumerate() {
            let (c, codes) = field(mem, &layout, fields.get(p));
            *part = (c << 16, column_type(codes).1);
        }
        let k = match decls[..n]
            .iter()
            .position(|d| (d.parts, d.part) == (decl.parts, decl.part))
        {
            Some(k) => {
                decls[k].flags |= decl.flags;
                k
            }
            None if n < decls.len() => {
                decl.id = n as u8 + 1;
                decls[n] = decl;
                n += 1;
                n - 1
            }
            None => rts_trap_with("native table: a table has at most 63 indexes"),
        };
        ids.add(i).write(nat_of_u64(k as u64 + 1));
    }
    let image = or_trap(schema_image(mem, &layout, 0, SCHEMA_VERSION, &decls[..n]));
    let id = {
        let mut host = StoreHost::new(mem, native_store_location());
        let (_, id) = or_trap(Table::create(&mut host, 0, image));
        let block = or_trap(host.get_mut(Region::Block(id)));
        or_trap(put_u64(block, CODEC_HASH, layout.hash));
        or_trap(host.add_root(id));
        nat64_of(mem, id)
    };
    let ids = array_of(mem, ids, lists.len());
    pair(mem, id, ids)
}

/// The number of rows of the table whose root block is `id`: the count of its live rows the table
/// keeps, read without opening the table, which only its other reads and its writes need.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_size<M: Memory>(mem: &mut M, id: u64, codec: Value) -> u64 {
    let layout = header_of(codec);
    let host = StoreHost::new(mem, native_store_location());
    root_block(&host, id, &layout);
    or_trap(root::check(&host, id));
    let tail = or_trap(root::u64_at(&host, id, root::TAIL));
    or_trap(buffer::u64_of(&host, tail, buffer::H_LIVE))
}

/// Inserts row `r` under key `k`, or replaces the row `k` has; 1 when `k` is new, else 0. A step
/// of maintenance runs first when the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_put<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value, r: Value) -> u64 {
    put_cells::<M, false>(mem, id, codec, k, r)
}

/// `nt_put`, but 0, writing no row, when a unique index has the value of `r` for another row,
/// else 1.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_try_put<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value, r: Value) -> u64 {
    put_cells::<M, true>(mem, id, codec, k, r)
}

/// `nt_put`, and with `TRY`, `nt_try_put`.
#[inline(always)]
unsafe fn put_cells<M: Memory, const TRY: bool>(
    mem: &mut M,
    id: u64,
    codec: Value,
    k: Value,
    r: Value,
) -> u64 {
    let layout = layout_of(codec);
    let mut row = cells(mem, &layout, k, r);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        row = map.cells(&mut *mem, row);
    }
    let mut table = table_at(&mut host, id);
    let mut scratch = BlobScratch { mem };
    or_trap(table.write_step());
    if TRY {
        return match table.upsert(&mut scratch, &mut row) {
            Err(Fault::Unique) => 0,
            done => {
                or_trap(done);
                1
            }
        };
    }
    put_row(&mut table, &mut scratch, &mut row)
}

/// Sets the fields of `fields`, a value of the row type of the codec `named`, in the row key `k`
/// has, and keeps the row's other fields; 1 when `k` has a row, else 0, writing nothing. Each field
/// of `named` is a field of the table's row of the same name and type, or a Nat (an option of one)
/// where the table's is an Int (an option of one) (`update_cells`); traps otherwise, and for a
/// view's codec. Only the indexes with a part among the fields whose value changes move. A step of
/// maintenance runs first when the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_update<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    named: Value,
    k: Value,
    fields: Value,
) -> u64 {
    set_fields::<M, false>(mem, id, codec, named, k, fields, false)
}

/// Sets the fields of `fields` in the row key `k` has, as `nt_update` does; when `k` has no row,
/// inserts one from them, with null in the row's other fields. 1 when `k` is new, else 0. The
/// compiler found each field of `named` in the table's row and gave it the table's type: the
/// codec's second Blob holds, in place of a hash list, the u32 position of each in the table's
/// layout (`placed`).
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_upsert<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    named: Value,
    k: Value,
    fields: Value,
) -> u64 {
    set_fields::<M, false>(mem, id, codec, named, k, fields, true)
}

/// Adds the elements of each array of `fields`, a value of the row type of the codec `named`, at
/// the end of that array field of the row key `k` has, and keeps the row's other fields, as
/// `nt_update` sets them; 1 when `k` has a row, else 0, writing nothing. A null array in the row
/// counts as empty; a null in `fields` adds nothing. Traps as `nt_update` does, and for a field
/// that is not an array.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_append<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    named: Value,
    k: Value,
    fields: Value,
) -> u64 {
    set_fields::<M, true>(mem, id, codec, named, k, fields, false)
}

/// `nt_update`, with `insert` `nt_upsert`, and with `APPEND` `nt_append`. Out of line, as is
/// `put_row`, so that each of the table's write calls is made in one place, where it is inlined.
#[inline(never)]
unsafe fn set_fields<M: Memory, const APPEND: bool>(
    mem: &mut M,
    id: u64,
    codec: Value,
    named: Value,
    k: Value,
    fields: Value,
    insert: bool,
) -> u64 {
    let layout = layout_of(codec);
    let places = insert.then(|| blob_bytes(labels_of(named)));
    let named = layout_of(named);
    if APPEND && merge(&layout, &named, true).any(|(_, e, _)| array_type(e.codes).is_none()) {
        rts_trap_with("native table: append takes array fields only");
    }
    let n = named.columns() - 1;
    let leaves = scratch::<u32, M>(mem, n);
    let mut row = update_cells(mem, &layout, &named, places, k, fields, leaves);
    // The key's cells are the row's.
    let mut k = row;
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        row = map.cells(&mut *mem, row);
        for i in 0..n {
            leaves.add(i).write(map.leaf(*leaves.add(i)));
        }
    }
    let mut table = table_at(&mut host, id);
    let mut scratch = BlobScratch { mem };
    or_trap(table.write_step());
    let leaves = core::slice::from_raw_parts(leaves, n);
    let (rid, _) = or_trap(if APPEND {
        let give = |row: &mut Cells, leaf, b: &[u8]| *row = row.with(&mut *mem, leaf, b);
        table.append(&mut scratch, &mut k, leaves, &mut row, give)
    } else {
        table.update(&mut scratch, &mut k, leaves, &mut row, insert)
    });
    (rid.is_some() != insert) as u64
}

/// The table's upsert of `row`; 1 when its key is new.
#[inline(never)]
unsafe fn put_row<M: Memory>(
    table: &mut Table<StoreHost<M>>,
    scratch: &mut BlobScratch<M>,
    row: &mut Cells,
) -> u64 {
    let (done, _) = or_trap(table.upsert(scratch, row));
    matches!(done, Upsert::Inserted(_)) as u64
}

/// The codec of a view of the table of `codec`, a table's codec or a view's, with the row type of
/// the codec `viewed`, a record whose fields are fields of the table's row, of the same names and
/// types, or the table's scalar row: a pair, as every codec is, of the view's layout
/// (`view_layout`) and the hash list of the view's row type. A read with it reads only the view's
/// fields. Traps when the table's row has no field of a name of `viewed`'s, or has it with
/// another type.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_view<M: Memory>(mem: &mut M, codec: Value, viewed: Value) -> Value {
    let view = checked_layout_of(viewed);
    let layout = view_layout(mem, blob_bytes(codec.as_array().get(0)), &view);
    pair(mem, layout, labels_of(viewed))
}

/// The row key `k` has, built as a value of the row type, or null.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_get<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value) -> Value {
    let Some(layout) = Layout::of(blob_bytes(codec.as_array().get(0))) else {
        return get_view(mem, id, codec, k);
    };
    let one;
    let mut k = key!(mem, &layout, k, one);
    let mem: *mut M = mem;
    let mut row = RowOut::new(mem, &layout, labels_of(codec), Out::Row);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    match root_block(&host, id, &layout) {
        None => get(mem, &mut host, id, &mut k, &mut row),
        Some(map) => get(mem, &mut host, id, &mut k, &mut row.mapped(map)),
    }
}

/// The row key `k` has in the table whose root block is `id`, built by `row`, or null.
#[inline(always)]
unsafe fn get<'a, M: Memory, F: Places<'a>>(
    mem: *mut M,
    host: &mut StoreHost<'_, M>,
    id: u64,
    k: &mut Cells,
    row: &mut RowOut<'a, M, F>,
) -> Value {
    let table = table_at(host, id);
    let mut scratch = BlobScratch { mem };
    let found = match row.leaves() {
        None => table.get(&mut scratch, k, row),
        Some(leaves) => table.get_leaves(&mut scratch, k, leaves, row),
    };
    if or_trap(found) {
        row.finish(&table)
    } else {
        NULL_POINTER
    }
}

/// `nt_get` with a view's codec: the row key `k` has, with only the view's fields read, or null.
/// Out of line, so a call with a table's codec runs as before.
#[cold]
#[inline(never)]
unsafe fn get_view<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value) -> Value {
    let (layout, picks) = view_of(codec);
    let one;
    let mut k = key!(mem, &layout, k, one);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let map = root_block(&host, id, &layout);
    let mut row = RowOut::picked(mem, &layout, picks, map, labels_of(codec), Out::Row);
    get(mem, &mut host, id, &mut k, &mut row)
}

/// Removes the row key `k` has; 1 when `k` had one, else 0. A step of maintenance runs first when
/// the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_remove<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value) -> u64 {
    let layout = header_of(codec);
    let one;
    let mut k = key!(mem, &layout, k, one);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let mut table = open(&mut host, id, &layout);
    let mut scratch = BlobScratch { mem };
    or_trap(table.write_step());
    let (removed, _) = or_trap(table.remove(&mut scratch, &mut k));
    removed.is_some() as u64
}

/// Removes the rows of the table whose root block is `id` that the walk of the secondary index on
/// `fields`, found in `indexes` (`index_of`), hands out from the key the cells of the array `from`
/// give, or from the index's first key when `from` is null, to the last row of the key the cells of
/// `to` give, or to its last key when `to` is null; returns how many. A step of maintenance runs
/// first when the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_remove_range<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    indexes: Value,
    fields: Value,
    from: Value,
    to: Value,
) -> u64 {
    let mut layout = read_layout_of(codec);
    let index = index_of(indexes, fields);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        layout = map.layout(&mut *mem, &layout);
    }
    let mut table = table_at(&mut host, id);
    or_trap(table.write_step());
    let from = index_key_of(mem, &layout, &table, index, from);
    let to = index_key_of(mem, &layout, &table, index, to);
    or_trap(table.remove_range(&mut BlobScratch { mem }, index, from, to)).0
}

/// The secondary index id of the index on the field list `fields`, an array of Texts, as the
/// array `indexes` gives it: a table keeps its indexes as pairs of a field list and an index id.
/// The pair is the one whose list is `fields` itself, or else holds the same Texts in the same
/// order; traps when no pair's does. A caller that names an index by the very field list the
/// table keeps finds it without comparing a Text.
unsafe fn index_of(indexes: Value, fields: Value) -> u8 {
    let indexes = indexes.as_array();
    let fields = fields.as_array();
    let id = 'found: {
        for i in 0..indexes.len() {
            let pair = indexes.get(i).as_array();
            if pair.get(0).as_array() == fields {
                break 'found pair.get(1);
            }
        }
        let parts = fields.len();
        for i in 0..indexes.len() {
            let pair = indexes.get(i).as_array();
            let list = pair.get(0).as_array();
            let same = |p: usize| text_compare(list.get(p), fields.get(p)) == 0;
            if list.len() == parts && (0..parts).all(same) {
                break 'found pair.get(1);
            }
        }
        rts_trap_with("native table: no index on these fields")
    };
    match compact(id).map(u8::try_from) {
        Some(Ok(index)) => index,
        _ => rts_trap_with(Fault::BadRef.text()),
    }
}

/// The cells of the index key the array `key` gives secondary index `index` of `table`, one cell
/// per part of the index in key order (per one of its first parts, at least one, when `prefix`), in
/// a Blob that lives until the next collection: a cell for every column, as a row's, null for the
/// columns the key has no part of, which the crate does not read; and whether the key has fewer
/// parts than the index.
unsafe fn index_cells<'c, M: Memory, H: PageHost>(
    mem: *mut M,
    layout: &Layout,
    table: &Table<'_, H>,
    index: u8,
    key: Value,
    prefix: bool,
) -> (Cells<'c>, bool) {
    let key = key.as_array();
    let columns = layout.columns();
    let cells = scratch::<CellIn, M>(&mut *mem, columns);
    for i in 0..columns {
        cells.add(i).write(CellIn::Null);
    }
    let (shape, h) = (table.shape(), table.host());
    let d = or_trap(indexes::find(shape, h, index));
    let (n, short) = (key.len(), key.len() < d.ords().len());
    if n == 0 || n > d.ords().len() || short && !prefix {
        rts_trap_with("native table: index key does not match the index's fields");
    }
    for (i, &o) in d.ords().iter().take(n).enumerate() {
        let column = (or_trap(shape.leaf(h, u64::from(o))).id >> 16) as usize;
        let codes = match layout.entries().take(columns).nth(column) {
            Some(e) => e.codes,
            None => rts_trap_with("native table: malformed layout"),
        };
        cells
            .add(column)
            .write(key_cell(&mut *mem, codes, key.get(i)));
    }
    (
        Cells::new(core::slice::from_raw_parts(cells, columns)),
        short,
    )
}

/// The number of rows of the table whose root block is `id` whose key under the secondary index
/// on `fields`, found in `indexes` (`index_of`), starts with the one the cells of the array `key`
/// give, one per part of the index in key order, for all its parts or its first ones.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_count<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    indexes: Value,
    fields: Value,
    key: Value,
) -> u64 {
    let mut layout = read_layout_of(codec);
    let index = index_of(indexes, fields);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        layout = map.layout(&mut *mem, &layout);
    }
    let table = table_at(&mut host, id);
    let (mut cells, short) = index_cells(mem, &layout, &table, index, key, true);
    let mut scratch = BlobScratch { mem };
    or_trap(if short {
        table.count_prefix(&mut scratch, index, key.as_array().len() as u8, &mut cells)
    } else {
        table.count(&mut scratch, index, &mut cells)
    }) as u64
}

/// The key `f` writes at the start of the buffer it is given, of the length it returns, or `None`
/// when `f` returns none. The buffer is scratch of `WALK_BUFFER` bytes, and grows to the bytes each
/// `Scratch` fault of `f` names.
unsafe fn key_of<'k, M: Memory>(
    mem: *mut M,
    mut f: impl FnMut(&mut [u8]) -> Result<Option<usize>, Fault>,
) -> Result<Option<&'k [u8]>, Fault> {
    let mut size = WALK_BUFFER;
    loop {
        let buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
        match f(buf) {
            Ok(n) => return Ok(n.map(|n| &buf[..n])),
            Err(Fault::Scratch { need }) => size = (need as usize).max(2 * size),
            Err(fault) => return Err(fault),
        }
    }
}

/// The key the cells of the array `key` give secondary index `index` of `table`, its full encoding,
/// in a Blob that lives until the next collection; `None` when `key` is null.
unsafe fn index_key_of<'k, M: Memory, H: PageHost>(
    mem: *mut M,
    layout: &Layout,
    table: &Table<'_, H>,
    index: u8,
    key: Value,
) -> Option<&'k [u8]> {
    if key == NULL_POINTER {
        return None;
    }
    let (mut cells, _) = index_cells(mem, layout, table, index, key, false);
    or_trap(key_of(mem, |out| {
        table.index_key(index, &mut cells, out).map(Some)
    }))
}

/// A new array of the first `n` values at `values`.
#[inline(always)]
unsafe fn array_of<M: Memory>(mem: &mut M, values: *const Value, n: usize) -> Value {
    let array = alloc_array(mem, TAG_ARRAY_I, n);
    for i in 0..n {
        array.as_array().initialize(i, *values.add(i), mem);
    }
    allocation_barrier(array)
}

/// A new Blob that holds `a` and then `b`.
unsafe fn blob_of_parts<M: Memory>(mem: &mut M, a: &[u8], b: &[u8]) -> Value {
    let v = alloc_blob(mem, TAG_BLOB_B, Bytes(a.len() + b.len()));
    let out = v.as_blob_mut().payload_addr();
    core::ptr::copy_nonoverlapping(a.as_ptr(), out, a.len());
    core::ptr::copy_nonoverlapping(b.as_ptr(), out.add(a.len()), b.len());
    allocation_barrier(v)
}

/// Up to `limit` rows, 1 to 64, of the table whose root block is `id`, in the order of the
/// secondary index on `fields`, found in `indexes` (`index_of`), as pairs of their key and their
/// row, and the token to go on from, empty when no row is left: an array and a Blob in a pair.
/// Keys rise, or fall when `reverse` is 1; the rows of one key come in row id order either way.
///
/// A walk resumes after the row its `token` names, the last row the call that gave the token
/// handed back. A walk with an empty token starts, by `start`: 0 at the index's first key in its
/// direction, 1 at its first key whose first part is not null, 2 at the key the cells of the array
/// `from` give, or at the first key past it when no row has it; `start` plus four times n starts it
/// at the n-th row from there instead, 0 the first, the rows before it not read. It stops before the first key past
/// the key the cells of `to` give, when `to` is not null. With a view's codec (`nt_view`), only
/// the view's fields of each row are read.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_walk<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    indexes: Value,
    fields: Value,
    reverse: u64,
    start: u64,
    from: Value,
    to: Value,
    token: Value,
    limit: u64,
) -> Value {
    let bytes = blob_bytes(codec.as_array().get(0));
    let (mut layout, view) = match Layout::of(bytes) {
        Some(layout) => (layout, None),
        None => viewed_picks(codec),
    };
    let index = index_of(indexes, fields);
    let dir = if reverse != 0 { Dir::Rev } else { Dir::Fwd };
    // A key past `to` in the walk's direction.
    let past = if reverse != 0 {
        Ordering::Less
    } else {
        Ordering::Greater
    };
    let limit = limit.clamp(1, BATCH_LIMIT) as usize;
    let rids = scratch::<u64, M>(mem, limit);
    let rows = scratch::<Value, M>(mem, limit);
    let token = blob_bytes(token);
    let mem: *mut M = mem;
    // With a view's codec, `row` builds only keys: the rows are built from the view's fields.
    let out = if view.is_none() { Out::Entry } else { Out::Key };
    let mut row = RowOut::new(mem, &layout, labels_of(codec), out);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let map = root_block(&host, id, &layout);
    if let Some(map) = map {
        layout = map.layout(&mut *mem, &layout);
    }
    let table = table_at(&mut host, id);
    let (start, nth) = (start & 3, start >> 2);
    // Where the walk starts; `None` when no row's first part is not null.
    let first = if !token.is_empty() {
        let Some(at) = token.len().checked_sub(8) else {
            rts_trap_with(Fault::BadToken.text())
        };
        let (key, rid) = token.split_at(at);
        Some(Start::After(
            key,
            u64::from_le_bytes(rid.try_into().unwrap()),
        ))
    } else {
        let key = match start {
            0 => Ok(None),
            1 => key_of(mem, |out| match dir {
                Dir::Fwd => table.min_key(index, out),
                Dir::Rev => table.max_key(index, out),
            }),
            2 => Ok(index_key_of(mem, &layout, &table, index, from)),
            _ => rts_trap_with("native table: unknown walk start"),
        };
        match (start, key) {
            (_, Ok(Some(key))) if nth > 0 => Some(Start::Nth(Some(key), nth)),
            (_, Ok(Some(key))) => Some(Start::Key(key)),
            (0, Ok(None)) if nth > 0 => Some(Start::Nth(None, nth)),
            (0, Ok(None)) => Some(Start::End),
            (_, Ok(None)) => None,
            (_, Err(fault)) => rts_trap_with(fault.text()),
        }
    };
    let to = index_key_of(mem, &layout, &table, index, to);
    // The row ids, from a walk that resumes after the last row it handed out each time a key does
    // not fit its buffer, with a buffer of the bytes the key needs.
    let (mut n, mut size) = (0, WALK_BUFFER);
    let (mut last, mut last_rid): (&[u8], u64) = (&[], 0);
    if let Some(first) = first {
        loop {
            let buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
            // The key handed out last, which fits a third of `buf`.
            let kept = scratch::<u8, M>(&mut *mem, size);
            let at = if n == 0 {
                first
            } else {
                Start::After(last, last_rid)
            };
            let (mut kept_len, mut stopped) = (None, false);
            let r = table.walk(index, dir, at, (limit - n) as u64, buf, |k, rid| {
                if to.is_some_and(|to| k.cmp(to) == past) {
                    // A fault of the callback's own, told apart from the crate's by `stopped`.
                    stopped = true;
                    return Err(Fault::BadToken);
                }
                if k.len() > size {
                    return Err(Fault::Scratch {
                        need: 3 * k.len() as u64,
                    });
                }
                core::ptr::copy_nonoverlapping(k.as_ptr(), kept, k.len());
                kept_len = Some(k.len());
                rids.add(n).write(rid);
                (n, last_rid) = (n + 1, rid);
                Ok(())
            });
            if let Some(len) = kept_len {
                last = core::slice::from_raw_parts(kept, len);
            }
            match r {
                Ok(_) => break,
                Err(_) if stopped => break,
                Err(Fault::Scratch { need }) => size = (need as usize).max(2 * size),
                Err(fault) => rts_trap_with(fault.text()),
            }
        }
    }
    match (view, map) {
        (None, None) => rows_of(&table, rids, n, &mut row, rows),
        (None, Some(map)) => rows_of(&table, rids, n, &mut row.mapped(map), rows),
        (Some(picks), _) => {
            let mut row = RowOut::picked(mem, &layout, picks, map, labels_of(codec), Out::Entry);
            rows_of(&table, rids, n, &mut row, rows)
        }
    }
    let mem = &mut *mem;
    let array = array_of(mem, rows, n);
    let token = if n == limit {
        blob_of_parts(mem, last, &last_rid.to_le_bytes())
    } else {
        blob_of_parts(mem, &[], &[])
    };
    pair(mem, array, token)
}

/// The rows of the `n` row ids at `rids` of `table`, built by `row` at the start of `rows`.
#[inline(always)]
unsafe fn rows_of<'a, M: Memory, F: Places<'a>>(
    table: &Table<'_, StoreHost<'_, M>>,
    rids: *const u64,
    n: usize,
    row: &mut RowOut<'a, M, F>,
    rows: *mut Value,
) {
    for i in 0..n {
        let found = match row.leaves() {
            None => table.get_rid(*rids.add(i), row),
            Some(leaves) => table.get_rid_leaves(*rids.add(i), leaves, row),
        };
        if !or_trap(found) {
            rts_trap_with("native table: an index names a row the table does not have");
        }
        rows.add(i).write(row.finish(table));
    }
}

/// Up to `limit` keys, 1 to 64, of the secondary index on `fields`, found in `indexes`
/// (`index_of`), of the table whose root block is `id`, in the index's order, each with the number of rows that have it, as pairs of the array of its
/// cells, one `NtCell` per part of the index, and a Nat; and the key to go on after, empty when no
/// key is left: an array and a Blob in a pair. From the index's first key, null first, when
/// `after` is empty, else from the first key past `after`, the last key the call that gave it
/// handed back. The counts are read at each call, so writes between calls show in the keys still
/// to come.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_group_counts<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    indexes: Value,
    fields: Value,
    after: Value,
    limit: u64,
) -> Value {
    let mut layout = read_layout_of(codec);
    let index = index_of(indexes, fields);
    let limit = limit.clamp(1, BATCH_LIMIT) as usize;
    let groups = scratch::<Value, M>(mem, limit);
    let after = blob_bytes(after);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        layout = map.layout(&mut *mem, &layout);
    }
    let table = table_at(&mut host, id);
    // The scalar type of each part of the index, and whether it can be null.
    let mut parts = [(0u8, false); INDEX_PARTS];
    let (shape, h) = (table.shape(), table.host());
    let d = or_trap(indexes::find(shape, h, index));
    for (part, &o) in parts.iter_mut().zip(d.ords()) {
        let leaf = or_trap(shape.leaf(h, u64::from(o)));
        let codes = match layout.entries().nth((leaf.id >> 16) as usize) {
            Some(e) => e.codes,
            None => rts_trap_with("native table: malformed layout"),
        };
        *part = (column_type(codes).0, leaf.nullable);
    }
    let parts = &parts[..d.ords().len()];
    // The keys handed out, from a walk that resumes after the last key it handed out each time a
    // key does not fit its buffer, with a buffer of the bytes the key needs.
    let (mut n, mut size) = (0, WALK_BUFFER);
    let mut last: &[u8] = after;
    loop {
        let buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
        // The key handed out last, and the bytes of a Text or Blob part: each fits `buf`.
        let kept = scratch::<u8, M>(&mut *mem, size);
        let bytes = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
        let from = if last.is_empty() { None } else { Some(last) };
        let mut kept_len = None;
        let r = table.group_counts(index, from, (limit - n) as u64, buf, |k, count| {
            core::ptr::copy_nonoverlapping(k.as_ptr(), kept, k.len());
            kept_len = Some(k.len());
            let mem = &mut *mem;
            let cells = key_cells(mem, parts, k, &mut *bytes);
            groups
                .add(n)
                .write(pair(mem, cells, nat_of_u64(count as u64)));
            n += 1;
            Ok(())
        });
        if let Some(len) = kept_len {
            last = core::slice::from_raw_parts(kept, len);
        }
        match r {
            Ok(_) => break,
            Err(Fault::Scratch { need }) => size = (need as usize).max(2 * size),
            Err(fault) => rts_trap_with(fault.text()),
        }
    }
    let mem = &mut *mem;
    let array = array_of(mem, groups, n);
    let token = blob_of_parts(mem, if n == limit { last } else { &[] }, &[]);
    pair(mem, array, token)
}

/// The Int the total `t` stands for, its bytes past i128 at the start of `buf`.
unsafe fn int_of_total<M: Memory>(mem: &mut M, t: Total, buf: &[u8]) -> Value {
    match t {
        Total::Small(x) => int_of_i128(mem, x),
        Total::Big(k) => int_of_twos(mem, &buf[..k]),
    }
}

/// What `f` gives, made a value by `done` with the buffer `f` was given, which `f` writes a value
/// past i128 to: `SUM_BUFFER` bytes on the stack, or scratch of the bytes each `Scratch` fault of
/// `f` names.
#[inline(always)]
unsafe fn with_buffer<M: Memory, T>(
    mem: *mut M,
    mut f: impl FnMut(&mut [u8]) -> Result<T, Fault>,
    done: impl FnOnce(&mut M, T, &[u8]) -> Value,
) -> Value {
    let mut first = [0u8; SUM_BUFFER];
    let mut buf: &mut [u8] = &mut first;
    loop {
        match f(buf) {
            Ok(t) => return done(&mut *mem, t, buf),
            Err(Fault::Scratch { need }) => {
                let size = (need as usize).max(2 * buf.len());
                buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
            }
            Err(fault) => rts_trap_with(fault.text()),
        }
    }
}

/// The sum and the number of the non-null values of the field of the row the Text `column` names,
/// as an Int and a Nat in a pair, over every row of the table whose root block is `id` when
/// `filter` is null, else over the rows whose field the Text `filter.0` names meets `filter.1`, a
/// value of type `NtFilter`; null when the summed field is not of an integer type. Over every row,
/// one read of the field's running total, at any table size.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_sum<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    column: Value,
    filter: Value,
) -> Value {
    let layout = header_of(codec);
    let (c, codes) = field(mem, &layout, column);
    let mut leaf = c << 16;
    let mut filter = filter_opt(mem, &layout, filter);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        leaf = map.leaf(leaf);
        if let Some(f) = &mut filter {
            f.leaf = map.leaf(f.leaf);
        }
    }
    let table = table_at(&mut host, id);
    if !matches!(column_type(codes).0, ty::U8..=ty::INT) {
        return NULL_POINTER;
    }
    let sum = |buf: &mut [u8]| match &filter {
        None => table.sum(leaf, buf),
        Some(f) => table.sum_where(f, leaf, buf),
    };
    with_buffer(mem, sum, |mem, (t, n), buf| {
        let sum = int_of_total(mem, t, buf);
        pair(mem, sum, nat_of_u64(n))
    })
}

/// The smallest non-null value of the field of the row the Text `column` names over every row of
/// the table whose root block is `id`, or the largest when `largest` is 1, as an Int; null when no
/// row holds one, or when the field is not of an integer type. Each row group keeps its own
/// smallest and largest value; a group whose value may have left with a removed or replaced row
/// is read from its rows.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_extreme<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    column: Value,
    largest: u64,
) -> Value {
    let layout = header_of(codec);
    let (c, codes) = field(mem, &layout, column);
    let mut leaf = c << 16;
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        leaf = map.leaf(leaf);
    }
    let table = table_at(&mut host, id);
    if !matches!(column_type(codes).0, ty::U8..=ty::INT) {
        return NULL_POINTER;
    }
    let extreme = |buf: &mut [u8]| table.extreme(leaf, largest != 0, buf);
    with_buffer(mem, extreme, |mem, t, buf| match t {
        Some(t) => int_of_total(mem, t, buf),
        None => NULL_POINTER,
    })
}

/// The position of the field of the row the Text `name` names in the layout, the key's 0, and
/// its type codes; traps when the row has no field of that name. In a table whose row type never
/// changed, the position is the field's column; in one whose row type has changed, the table's map
/// gives the column.
#[inline(always)]
unsafe fn field<'l, M: Memory>(mem: &mut M, layout: &Layout<'l>, name: Value) -> (u32, &'l [u8]) {
    let name = text_bytes(mem, name);
    match layout
        .entries()
        .enumerate()
        .find(|(_, e)| !e.name.is_empty() && e.name == name)
    {
        Some((c, e)) => (c as u32, e.codes),
        None => rts_trap_with("native table: the row type has no field of that name"),
    }
}

/// The filter `filter`, a value of type `NtFilter`, on the field of the row the Text `column`
/// names.
unsafe fn filter_of<'c, M: Memory>(
    mem: &mut M,
    layout: &Layout,
    column: Value,
    filter: Value,
) -> Filter<'c> {
    let (c, codes) = field(mem, layout, column);
    Filter {
        leaf: c << 16,
        cond: filter_cond(mem, codes, filter),
    }
}

/// The filter `filter.1` on the field of the row the Text `filter.0` names, or none when `filter`
/// is null.
unsafe fn filter_opt<'c, M: Memory>(
    mem: &mut M,
    layout: &Layout,
    filter: Value,
) -> Option<Filter<'c>> {
    if filter == NULL_POINTER {
        return None;
    }
    let pair = filter.as_array();
    Some(filter_of(mem, layout, pair.get(0), pair.get(1)))
}

/// The rows a filtered scan hands out, each built as `row` builds it when it ends, at the start
/// of `rows`.
struct Matches<'t, 'h, 'a, M: Memory, H: PageHost, F: Places<'a>> {
    row: RowOut<'a, M, F>,
    table: &'t Table<'h, H>,
    rows: *mut Value,
    n: usize,
}

impl<'a, M: Memory, H: PageHost, F: Places<'a>> RowSink for Matches<'_, '_, 'a, M, H, F> {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        self.row.cell(leaf, v)
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.row.row_end(rid)?;
        unsafe { self.rows.add(self.n).write(self.row.finish(self.table)) };
        self.n += 1;
        Ok(())
    }
}

/// Up to `limit` rows, at most 64, of the table whose root block is `id`, from row id `next` on
/// in row id order: every row when `filter` is null, else the rows whose field the Text
/// `filter.0` names meets `filter.1`, a value of type `NtFilter`; and the row id to go on from, or
/// null when no row is left: an array and an option in a pair. `what` says what is built of each
/// row: 0 the pair of its key and its row, 1 its key, 2 its row; with `BACK` added, every row
/// comes newest first, those below row id `next`, and `filter` is not read. Row ids never move, so
/// a scan resumed after writes hands back every row that stays in the table and meets the filter
/// exactly once.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_scan<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    what: u64,
    filter: Value,
    next: u64,
    limit: u64,
) -> Value {
    let Some(mut layout) = Layout::of(blob_bytes(codec.as_array().get(0))) else {
        return scan_view(mem, id, codec, what, filter, next, limit);
    };
    let out = scan_out(what);
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let row = RowOut::new(mem, &layout, labels_of(codec), out);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let map = root_block(&host, id, &layout);
    if let Some(map) = map {
        layout = map.layout(&mut *mem, &layout);
    }
    let filter = filter_opt(&mut *mem, &layout, filter);
    let (rest, n) = match map {
        None => scan(&mut host, id, filter.as_ref(), what, next, limit, rows, row),
        Some(map) => scan(
            &mut host,
            id,
            filter.as_ref(),
            what,
            next,
            limit,
            rows,
            row.mapped(map),
        ),
    };
    batch(&mut *mem, rows, n, rest)
}

/// `nt_scan` with a view's codec: the rows come with only the view's fields read. Out of line, so
/// a call with a table's codec runs as before.
#[cold]
#[inline(never)]
unsafe fn scan_view<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    what: u64,
    filter: Value,
    next: u64,
    limit: u64,
) -> Value {
    let (mut layout, picks) = view_of(codec);
    let out = scan_out(what);
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let map = root_block(&host, id, &layout);
    let row = RowOut::picked(mem, &layout, picks, map, labels_of(codec), out);
    if let Some(map) = map {
        layout = map.layout(&mut *mem, &layout);
    }
    let filter = filter_opt(&mut *mem, &layout, filter);
    let (rest, n) = scan(&mut host, id, filter.as_ref(), what, next, limit, rows, row);
    batch(&mut *mem, rows, n, rest)
}

/// Added to `nt_scan`'s `what`: the rows come newest first.
const BACK: u64 = 4;

/// What `nt_scan` builds of each row, by its `what` less `BACK`.
#[inline(always)]
fn scan_out(what: u64) -> Out {
    match what & !BACK {
        0 => Out::Entry,
        1 => Out::Key,
        2 => Out::Row,
        _ => rts_trap_with("native table: unknown scan"),
    }
}

/// The values a scan hands back, the first `n` at `rows`, in an array, and the row id to go on
/// from, or null: in a pair.
#[inline(always)]
unsafe fn batch<M: Memory>(mem: &mut M, rows: *const Value, n: usize, rest: Option<u64>) -> Value {
    let array = array_of(mem, rows, n);
    let rest = match rest {
        Some(rid) => nat64_of(mem, rid),
        None => NULL_POINTER,
    };
    pair(mem, array, rest)
}

/// `nt_scan`'s scan of the table whose root block is `id`, its rows built by `row` at the start
/// of `rows`: the row id to go on from, and the number of rows.
#[inline(always)]
unsafe fn scan<'a, M: Memory, F: Places<'a>>(
    host: &mut StoreHost<'_, M>,
    id: u64,
    filter: Option<&Filter>,
    what: u64,
    next: u64,
    limit: usize,
    rows: *mut Value,
    row: RowOut<'a, M, F>,
) -> (Option<u64>, usize) {
    let table = table_at(host, id);
    let mut out = Matches {
        row,
        table: &table,
        rows,
        n: 0,
    };
    let rest = match (what & BACK != 0, out.row.leaves()) {
        (true, leaves) => table.scan_back(leaves, next, limit as u64, &mut out),
        (false, None) => table.scan_where(filter, next, limit as u64, &mut out),
        (false, Some(leaves)) => table.scan_cells(leaves, filter, next, limit as u64, &mut out),
    };
    (or_trap(rest), out.n)
}

/// Up to `limit` rows, at most 64, of the table whose root block is `id`, from row id `next` on
/// in row id order: every row when `filter` is null, else the rows whose field the Text
/// `filter.0` names meets `filter.1`, a value of type `NtFilter`. Each row comes as an array of
/// `NtCell` values: its key's when `key` is 1, then those of the fields of the row the Texts of
/// the array `fields` name, in the order named; the table reads no other field, and builds no
/// row. With the row id to go on from, or null when no row is left: an array and an option in a
/// pair. Row ids never move, so a scan resumed after writes hands back every row that stays in
/// the table and meets the filter exactly once.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_scan_cells<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    key: u64,
    fields: Value,
    filter: Value,
    next: u64,
    limit: u64,
) -> Value {
    let mut layout = read_layout_of(codec);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        layout = map.layout(&mut *mem, &layout);
    }
    let mem = &mut *mem;
    let fields = fields.as_array();
    let first = usize::from(key != 0);
    let width = fields.len() + first;
    // Each cell's leaf, and its column's type: the key's first when asked for.
    let leaves = scratch::<u32, M>(mem, width);
    let types = scratch::<(u8, bool), M>(mem, width);
    if key != 0 {
        let Some(key) = layout.entries().next() else {
            rts_trap_with("native table: malformed layout")
        };
        leaves.write(0);
        types.write(column_type(key.codes));
    }
    for i in first..width {
        let (c, codes) = field(mem, &layout, fields.get(i - first));
        leaves.add(i).write(c << 16);
        types.add(i).write(column_type(codes));
    }
    let leaves = core::slice::from_raw_parts(leaves, width);
    let types = core::slice::from_raw_parts(types, width);
    let filter = filter_opt(mem, &layout, filter);
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let table = table_at(&mut host, id);
    let mut out = CellRows::new(mem, &table, leaves, types, rows);
    let rest = or_trap(table.scan_cells(leaves, filter.as_ref(), next, limit as u64, &mut out));
    batch(&mut *mem, rows, out.len(), rest)
}

/// The number of rows of the table whose root block is `id` whose field the Text `column` names
/// meets `filter`, a value of type `NtFilter`.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_count_where<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    column: Value,
    filter: Value,
) -> u64 {
    let mut layout = read_layout_of(codec);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    if let Some(map) = root_block(&host, id, &layout) {
        layout = map.layout(&mut *mem, &layout);
    }
    let filter = filter_of(&mut *mem, &layout, column, filter);
    or_trap(table_at(&mut host, id).count_where(&filter))
}

/// Removes every row of the table whose root block is `id`, with its index entries, at a cost
/// that does not grow with the table; 1 when work is then pending, as it is until the memory the
/// rows used is back, which the steps of maintenance give back.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_clear<M: Memory>(mem: &mut M, id: u64, codec: Value) -> u64 {
    let layout = read_layout_of(codec);
    let mut host = StoreHost::new(mem, native_store_location());
    let mut table = open(&mut host, id, &layout);
    (or_trap(table.clear()) != Wake::Idle) as u64
}

/// Runs steps of maintenance on the table whose root block is `id`, starting each only while
/// performance counter 0 is below its value at the start of the call plus `budget`, so a call can
/// go past its budget by the step it started last; 1 when work is still pending, else 0. A step
/// compacts or drops one whole row group, gives back part of the memory of the rows `nt_clear`
/// removed, or gives back pages that removed rows left empty; writes run one step each when work
/// is pending, so a table that receives no writes advances only through this call.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_maintain<M: Memory>(mem: &mut M, id: u64, codec: Value, budget: u64) -> u64 {
    let start = ic0_performance_counter(0);
    let layout = read_layout_of(codec);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let mut table = open(&mut host, id, &layout);
    let meter = Meter {
        limit: start.saturating_add(budget),
    };
    or_trap(table.maintain(&meter));
    (or_trap(table.wake()) != Wake::Idle) as u64
}

/// Changes the row type of the table whose root block is `id` from the one of `codec` to the one
/// of `next`, in place, for a migration: 1 when the two differ, else 0, changing nothing. The
/// fields of `next` are found by name among the table's columns (`evolved_image`): a field of the
/// same name and type keeps its column, a new field of an option type is a new column that the
/// rows stored before hold null in, and a column no field names is dropped, with the indexes that
/// have a part on it. Traps, changing nothing, when a field keeps its name but changes its type,
/// when a new field is not an option, or when either row type is a scalar. The rows written since
/// the last row group was packed are packed first; nothing else grows with the table. The table
/// then keeps the column of each field of `next` on a map page, and the codec hash of `next` with
/// `CHANGED` mixed in, so only a table value of `next` opens it from then on.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_evolve<M: Memory>(mem: &mut M, id: u64, codec: Value, next: Value) -> u64 {
    let layout = layout_of(codec);
    let new = checked_layout_of(next);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let had_map = root_block(&host, id, &layout).is_some();
    if new.hash == layout.hash {
        return 0;
    }
    if !layout.record_row() || !new.record_row() {
        rts_trap_with("native table: a row that is a scalar cannot change");
    }
    let mut table = table_at(&mut host, id);
    let (image, columns, m) = {
        let page = or_trap(table.host().get(Region::Page(table.shape().page)));
        let stored = or_trap(SchemaView::open(page));
        or_trap(evolved_image(&mut *mem, &stored, &new))
    };
    or_trap(table.evolve(image));
    let size = 8 + 2 * (columns.len() + m);
    let map = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
    Map::write(columns, m, map);
    let h = table.host_mut();
    let page = or_trap(area::image_page_alloc(h, id));
    or_trap(put(or_trap(h.get_mut(Region::Page(page))), 0, map));
    let block = or_trap(h.get_mut(Region::Block(id)));
    let old = or_trap(rd_u64(block, MAP_PAGE));
    or_trap(put_u64(block, MAP_PAGE, page));
    if had_map {
        or_trap(area::image_page_free(h, id, old));
    }
    let block = or_trap(h.get_mut(Region::Block(id)));
    or_trap(put_u64(block, CODEC_HASH, new.hash ^ CHANGED));
    1
}
