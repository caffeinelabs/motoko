//! The table primitives compiled code calls.
//!
//! A table is named by its root block, the reference `Table::create` returns, and the store
//! records every root block it creates a table at. Bytes 3,968 to 4,031 of a root block are the
//! host's, and the crate never touches them: the first word holds `TABLE_MAGIC`, the second the
//! codec hash of the table's layout. Every primitive checks that the reference is a root block the
//! store recorded and that both words match before it opens the table, and opens it with the
//! schema image stored in it. A reference to any other block traps, also one whose bytes a
//! program wrote, for example through a long Text in a row. Any fault of the crate traps.

use core::cmp::Ordering;

use ::native_table::consts::GROUP_SLOTS;
use ::native_table::index::secondary::Start;
use ::native_table::mem::{put_u64, rd_u64};
use ::native_table::schema::IndexDecl;
use ::native_table::table::filter::Filter;
use ::native_table::table::totals::Total;
use ::native_table::table::{CellIn, CellOut, RowSink, Table, Upsert, buffer, indexes, root};
use ::native_table::tree::Dir;
use ::native_table::value::ty;
use ::native_table::{Budget, Fault, PageHost, Region, Scratch, Wake};
use motoko_rts_macros::ic_mem_fn;

use super::layout::{Layout, SCHEMA_VERSION, schema_image, scratch};
use super::row::{
    CellRows, Cells, Out, RowOut, cells, column_type, compact, filter_cond, int_of_i128,
    int_of_twos, key, key_cell, key_cells, nat_of_u64, nat64_of, or_trap, pair, text_bytes,
};
use super::store::StoreHost;
use crate::barriers::allocation_barrier;
use crate::memory::{Memory, alloc_array, alloc_blob};
use crate::persistence::native_store_location;
use crate::rts_trap_with;
use crate::text::text_compare;
use crate::types::{Bytes, NULL_POINTER, TAG_ARRAY_I, TAG_BLOB_B, Value};

/// The first host word of a table's root block.
const TABLE_MAGIC: u64 = u64::from_le_bytes(*b"MOTOKONT");
/// The host word that holds the codec hash.
const CODEC_HASH: u64 = root::HOST_WORDS + 8;
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
/// the counter has reached `limit`; a write's step reads no limit.
struct Meter {
    limit: u64,
}

impl Meter {
    /// The meter of a write's step.
    const WRITE: Meter = Meter { limit: u64::MAX };
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

/// The layout of the codec `codec`: a codec is a pair of the layout, a Blob, and the hash list of
/// the row type.
unsafe fn layout_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::parse(layout).unwrap_or_else(|| rts_trap_with("native table: malformed layout"))
}

/// The layout of the codec `codec`, its header alone checked (`Layout::header`).
unsafe fn header_of<'a>(codec: Value) -> Layout<'a> {
    let layout = blob_bytes(codec.as_array().get(0));
    Layout::header(layout).unwrap_or_else(|| rts_trap_with("native table: malformed layout"))
}

/// The hash list of the row type in the codec `codec`.
unsafe fn labels_of(codec: Value) -> Value {
    codec.as_array().get(1)
}

/// The root block `id`, checked to be one the store recorded, of a table of the layout with codec
/// hash `hash`.
unsafe fn root_block<'h, M: Memory>(host: &'h StoreHost<'_, M>, id: u64, hash: u64) -> &'h [u8] {
    if !host.is_root(id) {
        rts_trap_with("native table: not a table of this key and row type");
    }
    let block = or_trap(host.get(Region::Block(id)));
    if rd_u64(block, root::HOST_WORDS) != Ok(TABLE_MAGIC) || rd_u64(block, CODEC_HASH) != Ok(hash) {
        rts_trap_with("native table: not a table of this key and row type");
    }
    block
}

/// The table whose root block is `id`, of the layout with codec hash `hash`.
unsafe fn open<'h, 'a, M: Memory>(
    host: &'h mut StoreHost<'a, M>,
    id: u64,
    hash: u64,
) -> Table<'h, StoreHost<'a, M>> {
    root_block(host, id, hash);
    or_trap(Table::open(host, id))
}

/// Creates an empty table of the codec's key and row types with a secondary index on each list of
/// the array `indexes`, an array of arrays of Texts that name fields of the row, in key order.
/// Returns its root block and the id of each list's index, from 1, in a pair; equal lists share
/// one index.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_create<M: Memory>(mem: &mut M, codec: Value, indexes: Value) -> Value {
    let layout = layout_of(codec);
    let lists = indexes.as_array();
    let ids = scratch::<Value, M>(mem, lists.len());
    let mut decls = [IndexDecl::default(); INDEX_MAX as usize];
    let mut n = 0;
    for i in 0..lists.len() {
        let fields = lists.get(i).as_array();
        let parts = fields.len();
        if parts == 0 || parts > INDEX_PARTS {
            rts_trap_with("native table: an index has 1 to 7 fields");
        }
        let mut decl = IndexDecl {
            id: 0,
            flags: 0,
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
            .position(|d| IndexDecl { id: d.id, ..decl } == *d)
        {
            Some(k) => k,
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
        or_trap(put_u64(block, root::HOST_WORDS, TABLE_MAGIC));
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
    root_block(&host, id, layout.hash);
    or_trap(root::check(&host, id));
    let tail = or_trap(root::u64_at(&host, id, root::TAIL));
    or_trap(buffer::u64_of(&host, tail, buffer::H_LIVE))
}

/// Inserts row `r` under key `k`, or replaces the row `k` has; 1 when `k` is new, else 0. A step
/// of maintenance runs first when the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_put<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value, r: Value) -> u64 {
    let layout = layout_of(codec);
    let mut row = cells(mem, &layout, k, r);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let mut table = open(&mut host, id, layout.hash);
    let mut scratch = BlobScratch { mem };
    or_trap(table.write_step(&Meter::WRITE));
    let (done, _) = or_trap(table.upsert(&mut scratch, &mut row));
    matches!(done, Upsert::Inserted(_)) as u64
}

/// The row key `k` has, built as a value of the row type, or null.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_get<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value) -> Value {
    let layout = layout_of(codec);
    let k = [key(mem, &layout, k)];
    let mut k = Cells::new(&k);
    let mem: *mut M = mem;
    let mut row = RowOut::new(mem, &layout, labels_of(codec), Out::Row);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    let mut scratch = BlobScratch { mem };
    if or_trap(table.get(&mut scratch, &mut k, &mut row)) {
        row.finish(&table)
    } else {
        NULL_POINTER
    }
}

/// Removes the row key `k` has; 1 when `k` had one, else 0. A step of maintenance runs first when
/// the table has work pending.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_remove<M: Memory>(mem: &mut M, id: u64, codec: Value, k: Value) -> u64 {
    let layout = layout_of(codec);
    let k = [key(mem, &layout, k)];
    let mut k = Cells::new(&k);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let mut table = open(&mut host, id, layout.hash);
    let mut scratch = BlobScratch { mem };
    or_trap(table.write_step(&Meter::WRITE));
    let (removed, _) = or_trap(table.remove(&mut scratch, &mut k));
    removed.is_some() as u64
}

/// Up to `limit` rows, at most 64, of the table whose root block is `id`, from row id `next` on in
/// row id order, and the row id to go on from, or null when no row is left: an array and an
/// option in a pair. `what` says what is built of each row: 0 the pair of its key and its row, 1
/// its key, 2 its row. Groups the directory has no entry for are skipped, and the open group is
/// read up to its next free position. Row ids never move, so a scan resumed after writes hands
/// back every row that stays in the table exactly once.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_scan<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    what: u64,
    next: u64,
    limit: u64,
) -> Value {
    let layout = layout_of(codec);
    let out = match what {
        0 => Out::Entry,
        1 => Out::Key,
        2 => Out::Row,
        _ => rts_trap_with("native table: unknown scan"),
    };
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let mut row = RowOut::new(mem, &layout, labels_of(codec), out);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    let h = table.host();
    let (open_group, _, tail) = or_trap(buffer::open_group(h, id));
    let end = open_group * GROUP_SLOTS + or_trap(buffer::u16_of(h, tail, buffer::H_OPEN_POS));
    let (mut rid, mut n) = (next, 0);
    while rid < end && n < limit {
        let group = rid / GROUP_SLOTS;
        let group_end = ((group + 1) * GROUP_SLOTS).min(end);
        if or_trap(root::group(h, id, group)) == 0 {
            rid = group_end;
            continue;
        }
        while rid < group_end && n < limit {
            if or_trap(table.get_rid(rid, &mut row)) {
                rows.add(n).write(row.finish(&table));
                n += 1;
            }
            rid += 1;
        }
    }
    let mem = &mut *mem;
    let array = array_of(mem, rows, n);
    let rest = if rid < end {
        nat64_of(mem, rid)
    } else {
        NULL_POINTER
    };
    pair(mem, array, rest)
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
/// per part of the index in key order, in a Blob that lives until the next collection: a cell for
/// every column, as a row's, null for the columns the index has no part of, which the crate does
/// not read.
unsafe fn index_cells<'c, M: Memory, H: PageHost>(
    mem: *mut M,
    layout: &Layout,
    table: &Table<'_, H>,
    index: u8,
    key: Value,
) -> Cells<'c> {
    let key = key.as_array();
    let columns = layout.columns();
    let cells = scratch::<CellIn, M>(&mut *mem, columns);
    for i in 0..columns {
        cells.add(i).write(CellIn::Null);
    }
    let (shape, h) = (table.shape(), table.host());
    let d = or_trap(indexes::find(shape, h, index));
    if key.len() != d.ords().len() {
        rts_trap_with("native table: index key does not match the index's fields");
    }
    for (i, &o) in d.ords().iter().enumerate() {
        let column = (or_trap(shape.leaf(h, u64::from(o))).id >> 16) as usize;
        let codes = match layout.entries().nth(column) {
            Some(e) => e.codes,
            None => rts_trap_with("native table: malformed layout"),
        };
        cells
            .add(column)
            .write(key_cell(&mut *mem, codes, key.get(i)));
    }
    Cells::new(core::slice::from_raw_parts(cells, columns))
}

/// The number of rows of the table whose root block is `id` whose key under the secondary index
/// on `fields`, found in `indexes` (`index_of`), is the one the cells of the array `key` give, one
/// per part of the index, in key order.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_count<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    indexes: Value,
    fields: Value,
    key: Value,
) -> u64 {
    let layout = layout_of(codec);
    let index = index_of(indexes, fields);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    let mut cells = index_cells(mem, &layout, &table, index, key);
    let mut scratch = BlobScratch { mem };
    or_trap(table.count(&mut scratch, index, &mut cells)) as u64
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
/// `from` give, or at the first key past it when no row has it. It stops before the first key past
/// the key the cells of `to` give, when `to` is not null.
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
    let layout = layout_of(codec);
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
    let mut row = RowOut::new(mem, &layout, labels_of(codec), Out::Entry);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
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
            2 => {
                let mut cells = index_cells(mem, &layout, &table, index, from);
                key_of(mem, |out| table.index_key(index, &mut cells, out).map(Some))
            }
            _ => rts_trap_with("native table: unknown walk start"),
        };
        match (start, key) {
            (_, Ok(Some(key))) => Some(Start::Key(key)),
            (0, Ok(None)) => Some(Start::End),
            (_, Ok(None)) => None,
            (_, Err(fault)) => rts_trap_with(fault.text()),
        }
    };
    let to = if to == NULL_POINTER {
        None
    } else {
        let mut cells = index_cells(mem, &layout, &table, index, to);
        or_trap(key_of(mem, |out| {
            table.index_key(index, &mut cells, out).map(Some)
        }))
    };
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
    for i in 0..n {
        if !or_trap(table.get_rid(*rids.add(i), &mut row)) {
            rts_trap_with("native table: an index names a row the table does not have");
        }
        rows.add(i).write(row.finish(&table));
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
    let layout = layout_of(codec);
    let index = index_of(indexes, fields);
    let limit = limit.clamp(1, BATCH_LIMIT) as usize;
    let groups = scratch::<Value, M>(mem, limit);
    let after = blob_bytes(after);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
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

/// The sum and the number of values `f` gives, as an Int and a Nat in a pair. `f` writes a sum past
/// i128 to the buffer it is given, scratch of `SUM_BUFFER` bytes that grows to the bytes each
/// `Scratch` fault of `f` names.
#[inline(always)]
unsafe fn sum_of<M: Memory>(
    mem: *mut M,
    mut f: impl FnMut(&mut [u8]) -> Result<(Total, u64), Fault>,
) -> Value {
    let mut size = SUM_BUFFER;
    loop {
        let buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
        match f(buf) {
            Ok((total, n)) => {
                let mem = &mut *mem;
                let sum = match total {
                    Total::Small(x) => int_of_i128(mem, x),
                    Total::Big(k) => int_of_twos(mem, &buf[..k]),
                };
                return pair(mem, sum, nat_of_u64(n));
            }
            Err(Fault::Scratch { need }) => size = (need as usize).max(2 * size),
            Err(fault) => rts_trap_with(fault.text()),
        }
    }
}

/// The sum and the number of the non-null values of the field of the row the Text `column` names,
/// over every row of the table whose root block is `id`, as an Int and a Nat in a pair; null when
/// the field is not of an integer type. One read of the field's running total, at any table size.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_sum<M: Memory>(mem: &mut M, id: u64, codec: Value, column: Value) -> Value {
    let layout = layout_of(codec);
    let (c, codes) = field(mem, &layout, column);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    if !matches!(column_type(codes).0, ty::U8..=ty::INT) {
        return NULL_POINTER;
    }
    sum_of(mem, |buf| table.sum(c << 16, buf))
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
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    if !matches!(column_type(codes).0, ty::U8..=ty::INT) {
        return NULL_POINTER;
    }
    // A value past i128 goes to this buffer, or to scratch when it needs more room.
    let mut first = [0u8; SUM_BUFFER];
    let mut buf: &mut [u8] = &mut first;
    loop {
        match table.extreme(c << 16, largest != 0, buf) {
            Ok(Some(Total::Small(x))) => return int_of_i128(&mut *mem, x),
            Ok(Some(Total::Big(k))) => return int_of_twos(&mut *mem, &buf[..k]),
            Ok(None) => return NULL_POINTER,
            Err(Fault::Scratch { need }) => {
                let size = (need as usize).max(2 * buf.len());
                buf = core::slice::from_raw_parts_mut(scratch::<u8, M>(&mut *mem, size), size);
            }
            Err(fault) => rts_trap_with(fault.text()),
        }
    }
}

/// The column of the field of the row the Text `name` names, and its type codes; traps when the
/// row has no field of that name.
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

/// The rows a filtered scan hands out, each built as `row` builds it when it ends, at the start
/// of `rows`.
struct Matches<'t, 'h, 'a, M: Memory, H: PageHost> {
    row: RowOut<'a, M>,
    table: &'t Table<'h, H>,
    rows: *mut Value,
    n: usize,
}

impl<M: Memory, H: PageHost> RowSink for Matches<'_, '_, '_, M, H> {
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

/// Up to `limit` rows, at most 64, of the table whose root block is `id` whose field the Text
/// `column` names meets `filter`, a value of type `NtFilter`, from row id `next` on in row id
/// order, as pairs of their key and their row, and the row id to go on from, or null when no row
/// is left: an array and an option in a pair. Row ids never move, so a scan resumed after writes
/// hands back every row that stays in the table and meets the filter exactly once.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_scan_where<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    column: Value,
    filter: Value,
    next: u64,
    limit: u64,
) -> Value {
    let layout = layout_of(codec);
    let filter = filter_of(mem, &layout, column, filter);
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let row = RowOut::new(mem, &layout, labels_of(codec), Out::Entry);
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    let mut out = Matches {
        row,
        table: &table,
        rows,
        n: 0,
    };
    let rest = or_trap(table.scan_where(&filter, next, limit as u64, &mut out));
    let n = out.n;
    let mem = &mut *mem;
    let array = array_of(mem, rows, n);
    let rest = match rest {
        Some(rid) => nat64_of(mem, rid),
        None => NULL_POINTER,
    };
    pair(mem, array, rest)
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
    let layout = layout_of(codec);
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
    let filter = if filter == NULL_POINTER {
        None
    } else {
        let pair = filter.as_array();
        Some(filter_of(mem, &layout, pair.get(0), pair.get(1)))
    };
    let limit = limit.min(BATCH_LIMIT) as usize;
    let rows = scratch::<Value, M>(mem, limit);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    let mut out = CellRows::new(mem, &table, leaves, types, rows);
    let rest = or_trap(table.scan_cells(leaves, filter.as_ref(), next, limit as u64, &mut out));
    let n = out.len();
    let mem = &mut *mem;
    let array = array_of(mem, rows, n);
    let rest = match rest {
        Some(rid) => nat64_of(mem, rid),
        None => NULL_POINTER,
    };
    pair(mem, array, rest)
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
    let layout = layout_of(codec);
    let filter = filter_of(mem, &layout, column, filter);
    let mut host = StoreHost::new(mem, native_store_location());
    or_trap(open(&mut host, id, layout.hash).count_where(&filter))
}

/// The sum and the number of the non-null values of the field of the row the Text `sum_column`
/// names, over the rows of the table whose root block is `id` whose field the Text `column` names
/// meets `filter`, a value of type `NtFilter`, as an Int and a Nat in a pair; null when the summed
/// field is not of an integer type.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_sum_where<M: Memory>(
    mem: &mut M,
    id: u64,
    codec: Value,
    sum_column: Value,
    column: Value,
    filter: Value,
) -> Value {
    let layout = layout_of(codec);
    let (c, codes) = field(mem, &layout, sum_column);
    let filter = filter_of(mem, &layout, column, filter);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let table = open(&mut host, id, layout.hash);
    if !matches!(column_type(codes).0, ty::U8..=ty::INT) {
        return NULL_POINTER;
    }
    sum_of(mem, |buf| table.sum_where(&filter, c << 16, buf))
}

/// Runs steps of maintenance on the table whose root block is `id`, starting each only while
/// performance counter 0 is below its value at the start of the call plus `budget`, so a call can
/// go past its budget by the step it started last; 1 when work is still pending, else 0. A step
/// compacts or drops one whole row group; writes run one step each when work is pending, so a
/// table that receives no writes advances only through this call.
#[ic_mem_fn(ic_only)]
pub unsafe fn nt_maintain<M: Memory>(mem: &mut M, id: u64, codec: Value, budget: u64) -> u64 {
    let start = ic0_performance_counter(0);
    let layout = layout_of(codec);
    let mem: *mut M = mem;
    let mut host = StoreHost::new(&mut *mem, native_store_location());
    let mut table = open(&mut host, id, layout.hash);
    let meter = Meter {
        limit: start.saturating_add(budget),
    };
    or_trap(table.maintain(&meter));
    (or_trap(table.wake()) != Wake::Idle) as u64
}
