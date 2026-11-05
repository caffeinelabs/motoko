//! The table: rows in a write buffer and sealed row groups, a primary index, the secondary indexes its schema
//! declares (kept current by every write), the integer leaves' totals kept exact and their extremes per group,
//! and the maintenance that seals frozen groups, compacts and drops sealed ones, releases dropped leaves and
//! frees what writes leave behind.
//!
//! A row type is flat: every leaf is a row-level scalar or `()` (records, tuples and options of scalars
//! flatten into such leaves).
//!
//! Hosts give rows and keys as cells through `RowSource`, and receive them through `RowSink`; a leaf is
//! named by its leaf id. A point operation (`get`, `upsert`, `remove`) builds the key's encoding E(k) on the
//! stack when it fits `KEY_STACK` bytes and in the host's scratch otherwise, so a key has no length cap. Writes
//! run no maintenance of their own: a sealed row's update writes its cells in place or in patch columns.
//! Maintenance runs in units of bounded work: `write_step`, which a host calls before every write, runs exactly
//! one when work is pending, and `maintain` runs units until its budget is spent.

pub mod buffer;
pub mod cell;
pub mod cur;
pub mod defaults;
pub mod drain;
pub mod evolve;
pub mod extremes;
pub mod filter;
pub mod group;
pub mod hot;
pub mod indexes;
pub mod jobs;
pub mod layout;
pub mod long;
mod long_key;
pub mod overlay;
pub mod page;
pub mod patch;
pub mod primary;
pub mod read;
pub mod root;
pub mod row;
pub mod seal;
pub mod totals;
pub mod update;
pub mod versions;
mod walks;
pub mod write;

use crate::consts::PAGE;
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::host::{wake_from_state, Budget, Progress, Scratch, Wake};
use crate::index::secondary::Start;
use crate::key::{LenSink, Sink, SliceSink};
use crate::mem::{put, put_u16, Owner, PageHost, Region};
use crate::schema::SchemaView;
use crate::tree::{Dir, Path};

use jobs::Tracked;
use layout::{Shape, Ty};
use primary::Primary;

/// A cell a host gives, or a cell the table hands out. `Arg { len }` in: a Text or Blob still in the message's
/// argument, read through `RowSource::fill`; out: a value that does not lie in one piece of table memory,
/// read through `Table::read_large`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellIn<'a> {
    Null,
    Bool(bool),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
    F32(f32),
    F64(f64),
    /// Text (UTF-8), Blob, Principal, `[u8; N]`.
    Bytes(&'a [u8]),
    /// LEB128 or signed LEB128, for values past 64 bits.
    Nat(&'a [u8]),
    Int(&'a [u8]),
    Arg {
        len: u64,
    },
    Unit,
}

pub type CellOut<'a> = CellIn<'a>;

/// Gives a row's (or a key's) leaves: any leaf, in any order, any number of times; every call for one leaf
/// returns the same cell. A write reads the cells once to check them before anything is written, and again
/// to write.
pub trait RowSource {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault>;
    /// For a cell given as `CellIn::Arg`: copies its bytes [at, at + dst.len()) into `dst`, a slice of the
    /// page that will hold them; called once per piece of at most one image block, in order.
    fn fill(&mut self, leaf: u32, at: u64, dst: &mut [u8]) -> Result<(), Fault>;
}

/// Receives a row's leaves in leaf order; values borrow table memory for the call only.
pub trait RowSink {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault>;
    fn row_end(&mut self, rid: u64) -> Result<(), Fault>;
}

/// What an upsert did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upsert {
    Inserted(u64),
    Replaced(u64),
}

/// The most bytes of E(k) a point operation builds on the stack; a longer E(k) is built in the scratch. A
/// secondary index's key may be longer: it is written into the index's key log instead (`indexes`).
pub const KEY_STACK: usize = 4_096;

/// Bytes of a stored key value one comparison piece reads.
const KEY_PIECE: usize = 4_096;

/// Bytes of the stack buffer a stored key value is copied into when it does not lie in one piece: room for
/// the longest value of a key of `KEY_STACK` bytes.
const PIECE: usize = 4_800;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// Checks that a schema image's leaves are live scalars or `()`, and its index declarations, before anything
/// is allocated.
fn check_schema(image: &[u8]) -> Result<(), Fault> {
    let v = SchemaView::open(image)?;
    if image.len() as u64 > PAGE || v.leaves == 0 {
        return Err(rt(Corruption::Leaf, 0));
    }
    for o in 0..v.leaves {
        let l = v.leaf(o)?;
        if Ty::of(&l).is_none() || l.dropped != 0 {
            return Err(rt(Corruption::Leaf, u64::from(o)));
        }
    }
    indexes::check(&v)
}

/// A Nat or Int held wide in its 64-bit cell when it fits, as a key encodes it.
fn smallest(x: cell::Val<'_>, ty: Ty) -> cell::Val<'_> {
    match (x, ty) {
        (cell::Val::Wide(w), Ty::Nat) => u64::try_from(w).map_or(cell::Val::Wide(w), cell::Val::U),
        (cell::Val::Wide(w), Ty::Int) => i64::try_from(w).map_or(cell::Val::Wide(w), cell::Val::I),
        (x, _) => x,
    }
}

/// The live row `read::live` found for `rid`, borrowed where it lies: the opened row is large, and moving it
/// out of the result copies it.
fn live_of(found: &Result<Option<read::Live>, Fault>, rid: u64) -> Result<&read::Live, Fault> {
    match found {
        Ok(Some(row)) => Ok(row),
        Ok(None) => Err(rt(Corruption::Link, rid)),
        Err(e) => Err(*e),
    }
}

/// A sink comparing the bytes it is given with an expected key.
struct KeyCompare<'a> {
    want: &'a [u8],
    at: usize,
    same: bool,
}

impl Sink for KeyCompare<'_> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        let end = self.at.saturating_add(bytes.len());
        if self.want.get(self.at..end) != Some(bytes) {
            self.same = false;
        }
        self.at = end;
        Ok(())
    }
}

/// Feeds key leaf `l`'s stored value `v`, longer than `piece`, to `cmp` as E encodes it, reading it in pieces:
/// Text and Blob escaped piece by piece, `[u8; N]` and Principal bytes as they are, and a Nat's or Int's
/// LEB128 bytes a window at a time.
fn long_key<H: PageHost + ?Sized>(
    h: &H,
    l: &layout::LeafInfo,
    v: &cur::SVal,
    cmp: &mut KeyCompare<'_>,
    piece: &mut [u8; KEY_PIECE],
) -> Result<(), Fault> {
    if l.nullable {
        crate::key::enc_opt_tag(cmp, true)?;
    }
    let n = v.len();
    match l.ty {
        Ty::Nat | Ty::Int => {
            crate::key::enc_int_leb_read(cmp, n, l.ty == Ty::Int, ImageId::Runtime, |at, out| cur::read(h, v, at, out))
        }
        Ty::Text | Ty::Blob | Ty::Fixed(_) | Ty::Principal => {
            let escaped = matches!(l.ty, Ty::Text | Ty::Blob);
            let mut at = 0u64;
            while at < n && cmp.same {
                let k = n.wrapping_sub(at).min(KEY_PIECE as u64);
                let d = piece.get_mut(..usize::try_from(k).unwrap_or(0)).ok_or(rt(Corruption::Range, k))?;
                cur::read(h, v, at, d)?;
                if escaped {
                    crate::key::enc_bytes_piece(cmp, d)?;
                } else {
                    cmp.put(d)?;
                }
                at = at.wrapping_add(k);
            }
            if escaped {
                crate::key::enc_bytes_end(cmp)?;
            }
            Ok(())
        }
        _ => Err(Fault::BadValue { leaf: l.id }),
    }
}

/// A table over a host, by its root block.
pub struct Table<'h, H: PageHost> {
    host: Tracked<'h, H>,
    root: u64,
    shape: Shape,
    /// Bytes the host copied from the message's argument during the last write.
    pub filled: u64,
    /// The schema image the table was opened with, while its schema hash differs from the table's (the
    /// table is stale): the next write or schema change adopts it first.
    open_image: Option<&'h [u8]>,
}

impl<'h, H: PageHost> Table<'h, H> {
    /// A new, empty table with table uid `uid` and the given schema image; returns it and its root block.
    pub fn create(host: &'h mut H, uid: u64, schema_image: &[u8]) -> Result<(Table<'h, H>, u64), Fault> {
        check_schema(schema_image)?;
        let mut host = Tracked::new(host);
        let tbl = crate::area::new_root(&mut host)?;
        let sp = crate::area::image_page_alloc(&mut host, tbl)?;
        put(host.get_mut(Region::Page(sp), Owner(crate::area::IMAGE_AREA))?, 0, schema_image)?;
        let shape = Shape::open(&host, sp)?;
        let kind = if shape.set { root::KIND_SET } else { root::KIND_TABLE };
        root::init(&mut host, tbl, kind, uid, shape.hash, shape.version, sp)?;
        shape.store(&mut host, tbl)?;
        indexes::declare(&mut host, tbl, &shape)?;
        let mut t = Table { host, root: tbl, shape, filled: 0, open_image: None };
        t.open_first_group()?;
        root::set_u64(&mut t.host, tbl, root::NEXT_GROUP, 1)?;
        Ok((t, tbl))
    }

    /// Group 0's descriptor and first tail block, with a fresh hot state whose running-record leaf list
    /// follows the rule of room over the schema's leaves.
    fn open_first_group(&mut self) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut self.host, self.root, &self.shape);
        let g = 0u64;
        let hh = shape.hot_h;
        let nb = buffer::new_block_at(h, tbl, g, 0, 0, None, hh)?;
        let d = buffer::new_descriptor(h, tbl, g, nb)?;
        let ext = extremes::create(h, tbl, shape, g)?;
        crate::mem::put_u64(h.get_mut(Region::Block(d), Owner(0))?, buffer::D_EXT, ext)?;
        {
            let b = h.get_mut(Region::Block(nb), Owner(0))?;
            crate::mem::put_u64(b, buffer::B_DESC, d)?;
            buffer::zero_hot(b, shape, hh)?;
        }
        // The running-record leaf list: the integer leaves whose records fit, in ordinal order.
        let base = crate::stats::running::LIST_AT;
        let mut r = 0u64;
        let mut recs = 0u64;
        for o in 0..shape.leaves {
            let l = shape.leaf(&*h, o)?;
            let Some(k) = l.ty.running().filter(|_| l.live()) else { continue };
            let bytes = k.bytes();
            let list = group::align8(r.wrapping_add(1).wrapping_mul(2));
            if base.wrapping_add(list).wrapping_add(recs).wrapping_add(bytes) <= crate::consts::HOT_MAX {
                let b = h.get_mut(Region::Block(nb), Owner(0))?;
                put_u16(b, crate::stats::running::LIST_AT.wrapping_add(r.wrapping_mul(2)), l.ordinal)?;
                r = r.wrapping_add(1);
                recs = recs.wrapping_add(bytes);
            }
        }
        if r != shape.run_r {
            return Err(rt(Corruption::Count, r));
        }
        // The totals the hot state has no room for, zero, in a totals space of their own.
        if shape.tot_bytes > 0 {
            totals::Totals::create(h, tbl, nb, shape.tot_bytes)?;
        }
        root::set_u64(h, tbl, root::OPEN_GROUP, g)?;
        root::set_u64(h, tbl, root::TAIL, nb)?;
        root::set_group(h, tbl, g, buffer::DESC_BIT | d)
    }

    /// The table whose root block is `root`. A schema image whose schema hash (the canonical one, which
    /// depends on the row type alone) differs from the table's makes the table *stale*: reads and maintenance
    /// use its stored schema, and the first write or schema change runs `change_schema` with the given image
    /// first, adopting its schema and nothing else; a refusal of it is that call's, before anything
    /// is written. Maintenance units never adopt it: an adoption is O(columns + leaves), which no unit's bound
    /// holds. An image of the table's row type leaves the table as it is.
    ///
    /// The table's shape is the one its root holds (`Shape::stored`), O(1), taken without hashing it again.
    /// When the root holds none of its current schema (another table format wrote it, or its format, schema
    /// page, schema hash or schema version word changed), the shape is derived from the stored schema image
    /// again, its schema hash checked, and stored. The image given may be the table's own
    /// schema page, read in place: its header is then read without checking its schema hash again, which was
    /// checked when the shape was derived from it.
    pub fn open(host: &'h mut H, root: u64, schema_image: &'h [u8]) -> Result<Table<'h, H>, Fault> {
        let mut host = Tracked::new(host);
        root::check(&host, root)?;
        let sp = root::u64_at(&host, root, root::SCHEMA_PAGE)?;
        let shape = match Shape::stored(&host, root)? {
            Some(s) => s,
            None => {
                let s = Shape::open(&host, sp)?;
                s.store(&mut host, root)?;
                s
            }
        };
        let v = if core::ptr::eq(schema_image.as_ptr(), host.get(Region::Page(sp))?.as_ptr()) {
            SchemaView::open_stored(schema_image)?
        } else {
            SchemaView::open(schema_image)?
        };
        let stored = root::u64_at(&host, root, root::SCHEMA_HASH)?;
        let open_image = (v.hash != stored).then_some(schema_image);
        Ok(Table { host, root, shape, filled: 0, open_image })
    }

    /// Whether the table is stale: opened with an image of another row type, not adopted yet.
    pub fn stale(&self) -> bool {
        self.open_image.is_some()
    }

    /// Before a mutating call on a stale table: the image it was opened with adopted (`change_schema`).
    fn adopt(&mut self) -> Result<(), Fault> {
        if let Some(img) = self.open_image {
            self.shape = evolve::change_schema(&mut self.host, self.root, &self.shape, img)?;
            self.open_image = None;
        }
        Ok(())
    }

    pub fn root(&self) -> u64 {
        self.root
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// The host, shared, for checkers and tests.
    pub fn host(&self) -> &H {
        self.host.h
    }

    /// The host, writable, for test hosts whose logs and counters a test reads between calls.
    pub fn host_mut(&mut self) -> &mut H {
        self.host.h
    }

    /// The host, given back: a host that opens the table again (`open`, with a new schema image after an
    /// upgrade) takes it from the table it held.
    pub fn into_host(self) -> &'h mut H {
        self.host.h
    }

    /// Live rows: the hot state's count, O(1).
    pub fn len(&self) -> Result<u64, Fault> {
        let tail = root::u64_at(&self.host, self.root, root::TAIL)?;
        buffer::u64_of(&self.host, tail, buffer::H_LIVE)
    }

    pub fn is_empty(&self) -> Result<bool, Fault> {
        Ok(self.len()? == 0)
    }

    /// E(k) from a key's cells into `s`.
    fn encode_key<K: RowSource + ?Sized, T: Sink>(&self, key: &mut K, s: &mut T) -> Result<(), Fault> {
        for o in 0..self.shape.leaves {
            let l = self.shape.leaf(&self.host, o)?;
            if l.key && l.live() {
                let v = row::cell_of(key, &l)?;
                cell::enc_key(s, &l, &v)?;
            }
        }
        Ok(())
    }

    /// E(k) from a key's cells: on the stack when it fits `KEY_STACK` bytes, else in the scratch, sized by a
    /// first encoding that only counts.
    fn key_in<'a, K: RowSource + ?Sized, S: Scratch>(
        &self,
        key: &mut K,
        stack: &'a mut [u8; KEY_STACK],
        scratch: &'a mut S,
    ) -> Result<&'a [u8], Fault> {
        let mut s = SliceSink::new(stack);
        match self.encode_key(key, &mut s) {
            Ok(()) => {
                let n = s.len;
                return Ok(stack.get(..n).unwrap_or(&[]));
            }
            Err(Fault::Scratch { .. }) => {}
            Err(e) => return Err(e),
        }
        let mut c = LenSink(0);
        self.encode_key(key, &mut c)?;
        let need = usize::try_from(c.0).map_err(|_| Fault::Scratch { need: c.0 })?;
        let buf = scratch.get(need)?;
        let buf = buf.get_mut(..need).ok_or(Fault::Scratch { need: c.0 })?;
        let mut s = SliceSink::new(buf);
        self.encode_key(key, &mut s)?;
        let n = s.len;
        Ok(buf.get(..n).unwrap_or(&[]))
    }

    /// The live row of E(k): its rid and where it lies, after checking that the row's key leaves encode to E(k).
    /// `path` is the caller's, for the lookup.
    fn find_key(&self, e: &[u8], path: &mut Path) -> Result<Option<(u64, read::Row)>, Fault> {
        let Some(p) = Primary::read(&self.host, self.root)? else { return Ok(None) };
        let look = primary::lookup(&self.host, &p, e, path)?;
        let Some((rid, _, _)) = look.found else { return Ok(None) };
        Ok(Some((rid, self.row_of(rid, e)?)))
    }

    /// Where the live row `rid` that the primary index holds for E(k) lies, after checking that its key leaves
    /// encode to E(k).
    fn row_of(&self, rid: u64, e: &[u8]) -> Result<read::Row, Fault> {
        let found = read::live(&self.host, self.root, rid);
        let row = live_of(&found, rid)?;
        if !self.key_matches(row, e)? {
            return Err(rt(Corruption::Encoding, rid));
        }
        Ok(row.row())
    }

    /// Whether a stored row's key leaves encode to `e`. A long value is read and compared in pieces of
    /// `KEY_PIECE` bytes, and a Nat's or Int's LEB128 bytes a window at a time, so no key is too long.
    fn key_matches(&self, row: &read::Live, e: &[u8]) -> Result<bool, Fault> {
        let h = &self.host;
        let mut cmp = KeyCompare { want: e, at: 0, same: true };
        let mut piece = [0u8; KEY_PIECE];
        for o in 0..self.shape.leaves {
            if !cmp.same {
                return Ok(false);
            }
            let l = self.shape.leaf(h, o)?;
            if !l.key || !l.live() {
                continue;
            }
            let v = match *row {
                read::Live::Buffered(at) => {
                    let b = overlay::buffered(h, &at)?;
                    overlay::buffered_leaf(h, &self.shape, &b, &l)?
                }
                read::Live::Sealed(ref s, pos) => {
                    let gv = group::GroupView::from_meta(h, &s.meta);
                    match read::sealed_cell(&gv, &self.shape, s, &l, gv.slot(pos)?, pos, &mut piece) {
                        Ok(x) => {
                            cell::enc_key(&mut cmp, &l, &smallest(x, l.ty))?;
                            continue;
                        }
                        // A value longer than `piece`, compared in pieces below.
                        Err(Fault::Scratch { .. }) => overlay::sealed_value(h, &self.shape, s, &l, pos)?,
                        Err(e) => return Err(e),
                    }
                }
            };
            let val = match v {
                cur::SVal::Null => cell::Val::Null,
                cur::SVal::Unit => cell::Val::Unit,
                cur::SVal::V(x) => smallest(x, l.ty),
                v => {
                    let n = v.len();
                    match usize::try_from(n).ok().and_then(|k| piece.get_mut(..k)) {
                        Some(d) => {
                            cur::read(h, &v, 0, d)?;
                            if matches!(l.ty, Ty::Nat | Ty::Int) {
                                cell::Val::Big(d)
                            } else {
                                cell::Val::Bytes(d)
                            }
                        }
                        None => {
                            long_key(h, &l, &v, &mut cmp, &mut piece)?;
                            continue;
                        }
                    }
                }
            };
            cell::enc_key(&mut cmp, &l, &val)?;
        }
        Ok(cmp.same && cmp.at == e.len())
    }

    /// The row with key k, handed to `out` in leaf order; `false` when there is none.
    pub fn get<S: Scratch, K: RowSource + ?Sized, O: RowSink>(
        &self,
        scratch: &mut S,
        key: &mut K,
        out: &mut O,
    ) -> Result<bool, Fault> {
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(key, &mut kb, scratch)?;
        let Some((rid, row)) = self.find_key(e, &mut Path::new())? else { return Ok(false) };
        self.emit(row, out)?;
        out.row_end(rid)?;
        Ok(true)
    }

    fn emit<S: RowSink>(&self, row: read::Row, out: &mut S) -> Result<(), Fault> {
        let h = &self.host;
        match row {
            read::Row::Buffered(at) => {
                let b = overlay::buffered(h, &at)?;
                read::emit_buffered(h, &self.shape, &b, out)
            }
            read::Row::Sealed { page, pos } => {
                let s = overlay::sealed(h, page)?;
                read::emit_sealed(h, &self.shape, &s, pos, out)
            }
        }
    }

    /// The row with rid `rid`, handed to `out`; `false` when no live row has it.
    pub fn get_rid<S: RowSink>(&self, rid: u64, out: &mut S) -> Result<bool, Fault> {
        let Some(row) = read::find(&self.host, self.root, rid)? else { return Ok(false) };
        self.emit(row, out)?;
        out.row_end(rid)?;
        Ok(true)
    }

    /// The rows whose cells of secondary index `index`'s parts encode to the key the cells of `key` give (by
    /// leaf id, as a row's): one descent of the index's tree, plus the key's count in each of its uploaded
    /// segments. A key of any length: one longer than `KEY_STACK` is compared with the stored keys of the tree
    /// and of each segment piece by piece as its cells are encoded again. `BadRef` when the table has no
    /// secondary index `index`, `Busy` while a schema change's build of it runs.
    pub fn count<K: RowSource + ?Sized>(&self, index: u8, key: &mut K) -> Result<i128, Fault> {
        indexes::count(&self.host, self.root, &self.shape, index, key)
    }

    /// The key the cells of `cells` give secondary index `index` (by leaf id, as a row's), its full encoding, at
    /// the start of `out`, for a walk to start at; its length. `BadRef` when the table has no secondary index
    /// `index`.
    pub fn index_key<K: RowSource + ?Sized>(&self, index: u8, cells: &mut K, out: &mut [u8]) -> Result<usize, Fault> {
        indexes::key(&self.host, &self.shape, index, cells, out)
    }

    /// Secondary index `index`'s rows in key order, from `start` in direction `dir` (going backward, the largest
    /// keys first; the rows of one key in rid order either way), at most `limit` of them: each handed to `row` as
    /// its key, the full encoding, and its rid. Returns how many were handed out; a walk stopped by its limit
    /// resumes with `Start::After` the last row. `buf` holds the walk's keys, three of them, so it needs three
    /// times the longest key it meets (and at least 3·W, and each segment's shared prefix more): a key that does
    /// not fit is `Scratch { need }`, the bytes `buf` needs, before any row of the key is handed out, and the walk
    /// resumes after the last row it handed out with a buffer of `need` bytes. `BadRef` when the table has no
    /// secondary index `index`, `Busy` while a schema change's build of it runs.
    pub fn walk<F: FnMut(&[u8], u64) -> Result<(), Fault>>(
        &self,
        index: u8,
        dir: Dir,
        start: Start<'_>,
        limit: u64,
        buf: &mut [u8],
        row: F,
    ) -> Result<u64, Fault> {
        walks::walk(&self.host, self.root, &self.shape, index, dir, start, limit, buf, row)
    }

    /// Secondary index `index`'s groups in key order: each distinct key its rows hold, the full encoding (a key
    /// whose first part is null first, as the index orders it), with how many live rows hold it, over the
    /// index's tree and its uploaded segments. From the first key above `after` (the smallest when `None`), at
    /// most `limit` of them, each handed to `group`; returns how many were handed out, fewer than `limit` only
    /// when no group is left. A call stopped by its limit is resumed with `after` the last key it handed out;
    /// counts are read at each call, so writes between calls change the groups still to come as they change
    /// the index. A group takes two descents of the tree and one key step in each uploaded segment, whatever
    /// its rows. `buf` holds three keys (the last handed out, the next, and each segment's), so it needs three
    /// times the longest key it meets (and at least 3·W): a key that does not fit is `Scratch { need }`, the
    /// bytes `buf` needs, before it is handed out, and the walk resumes after the last key it handed out with
    /// a buffer of `need` bytes. `BadRef` when the table has no secondary index `index`, `Busy` while a schema
    /// change's build of it runs.
    pub fn group_counts<F: FnMut(&[u8], i128) -> Result<(), Fault>>(
        &self,
        index: u8,
        after: Option<&[u8]>,
        limit: u64,
        buf: &mut [u8],
        group: F,
    ) -> Result<u64, Fault> {
        walks::groups(&self.host, self.root, &self.shape, index, after, limit, buf, group)
    }

    /// Secondary index `index`'s smallest key whose first part is not null (the smallest value of its column,
    /// for an index of one column), its full encoding, at the start of `out`; its length, or `None` when no row
    /// holds one. `out` also holds each segment's key, in its second half, so it needs twice the key: a key that
    /// does not fit is `Scratch { need }`, the bytes `out` needs. `BadRef` when the table has no secondary index
    /// `index`, `Busy` while a schema change's build of it runs.
    pub fn min_key(&self, index: u8, out: &mut [u8]) -> Result<Option<usize>, Fault> {
        walks::end_key(&self.host, self.root, &self.shape, index, false, out)
    }

    /// Secondary index `index`'s largest key whose first part is not null, as `min_key`.
    pub fn max_key(&self, index: u8, out: &mut [u8]) -> Result<Option<usize>, Fault> {
        walks::end_key(&self.host, self.root, &self.shape, index, true, out)
    }

    /// Bytes [at, at + buf.len()) of a long value handed out as `CellOut::Arg`.
    pub fn read_large(&self, rid: u64, leaf: u32, at: u64, buf: &mut [u8]) -> Result<usize, Fault> {
        read::read_large(&self.host, self.root, &self.shape, rid, leaf, at, buf)
    }

    /// Inserts the row, or updates the row with its key to the row's values (core Map's `add`).
    pub fn upsert<S: Scratch, R: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        row: &mut R,
    ) -> Result<(Upsert, Wake), Fault> {
        self.adopt()?;
        let (rid, inserted) = self.write_row(scratch, row)?;
        let u = if inserted { Upsert::Inserted(rid) } else { Upsert::Replaced(rid) };
        Ok((u, self.settle()?))
    }

    /// (rid, inserted) of an upsert. E(k) is built on the stack when it fits, else in the scratch (the plan
    /// made again once its length is known).
    fn write_row<S: Scratch, R: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        row: &mut R,
    ) -> Result<(u64, bool), Fault> {
        self.filled = 0;
        let mut kb = [0u8; KEY_STACK];
        let first = {
            let mut ks = SliceSink::new(&mut kb);
            match row::plan(&self.shape, &self.host, row, false, &mut ks) {
                Ok(plan) => Some((plan, ks.len)),
                Err(Fault::Scratch { .. }) => None,
                Err(e) => return Err(e),
            }
        };
        let (plan, e) = match first {
            Some((plan, n)) => (plan, kb.get(..n).unwrap_or(&[])),
            None => {
                let mut c = LenSink(0);
                row::plan(&self.shape, &self.host, row, false, &mut c)?;
                let need = usize::try_from(c.0).map_err(|_| Fault::Scratch { need: c.0 })?;
                let buf = scratch.get(need)?.get_mut(..need).ok_or(Fault::Scratch { need: c.0 })?;
                let mut ks = SliceSink::new(buf);
                let plan = row::plan(&self.shape, &self.host, row, false, &mut ks)?;
                let n = ks.len;
                (plan, buf.get(..n).unwrap_or(&[]))
            }
        };
        // One lookup of E(k) in the primary index: a hit is the row the write finds, a miss the position its
        // new entry takes. A primary index not made yet is empty; it is made only when the row goes in.
        let mut path = Path::new();
        let prim = Primary::read(&self.host, self.root)?;
        let look = match &prim {
            Some(p) => primary::lookup(&self.host, p, e, &mut path)?,
            None => primary::Lookup { found: None, vacant: None },
        };
        if let Some((rid, _, _)) = look.found {
            let found = self.row_of(rid, e)?;
            // The indexes' entries move first, while the stored row still holds its old cells.
            indexes::check_source(&self.shape, &self.host, row)?;
            self.index_update(found, row, rid, &mut kb, &mut path)?;
            let (h, tbl) = (&mut self.host, self.root);
            match found {
                read::Row::Buffered(at) => {
                    let b = overlay::buffered(&*h, &at)?;
                    let (_, filled) = if b.rw.is_some() {
                        update::update_snapshot(h, tbl, &self.shape, row, &b)?
                    } else {
                        write::update(h, tbl, &self.shape, row, &at)?
                    };
                    self.filled = filled;
                }
                read::Row::Sealed { page, pos } => {
                    let (_, filled) = update::update_sealed(h, tbl, &self.shape, row, page, pos)?;
                    self.filled = filled;
                }
            }
            return Ok((rid, false));
        }
        indexes::check_source(&self.shape, &self.host, row)?;
        let mut p = match prim {
            Some(p) => p,
            None => Primary::ensure(&mut self.host, self.root, self.shape.key_w, self.shape.key_hash)?,
        };
        let (rid, filled) = write::insert(&mut self.host, self.root, &self.shape, row, &plan)?;
        self.filled = filled;
        primary::insert(&mut self.host, self.root, &mut p, e, rid, look.vacant, &mut path)?;
        self.index_insert(row, rid, &mut kb, &mut path)?;
        Ok((rid, true))
    }

    /// Removes the row with key k; returns its rid.
    pub fn remove<S: Scratch, K: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        key: &mut K,
    ) -> Result<(Option<u64>, Wake), Fault> {
        self.adopt()?;
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(key, &mut kb, scratch)?;
        let Some(mut p) = Primary::read(&self.host, self.root)? else { return Ok((None, self.wake()?)) };
        let mut path = Path::new();
        let look = primary::lookup(&self.host, &p, e, &mut path)?;
        let Some((rid, node, i)) = look.found else { return Ok((None, self.wake()?)) };
        let found = read::live(&self.host, self.root, rid);
        let row = live_of(&found, rid)?;
        if !self.key_matches(row, e)? {
            return Err(rt(Corruption::Encoding, rid));
        }
        // The primary entry, then the indexes' entries, read from the row's cells, then the row.
        primary::remove(&mut self.host, self.root, &mut p, node, i, &mut path)?;
        self.index_remove(row, rid, &mut kb, &mut path)?;
        match *row {
            read::Live::Buffered(at) => {
                let b = overlay::buffered(&self.host, &at)?;
                if b.rw.is_some() {
                    update::delete_snapshot(&mut self.host, self.root, &self.shape, &b)?
                } else {
                    write::delete_buffered(&mut self.host, self.root, &self.shape, &at)?
                }
            }
            read::Live::Sealed(ref s, pos) => update::delete_sealed(&mut self.host, self.root, &self.shape, s, pos)?,
        }
        Ok((Some(rid), self.settle()?))
    }

    /// The maintenance state, written when it changes.
    fn settle(&mut self) -> Result<Wake, Fault> {
        jobs::settle(&mut self.host, self.root)
    }

    /// The maintenance state, O(1).
    pub fn wake(&self) -> Result<Wake, Fault> {
        Ok(wake_from_state(root::u8_at(&self.host, self.root, root::STATE)?))
    }

    /// One unit of maintenance; what it ran. A stale table's units run on its stored schema.
    pub fn unit<B: Budget>(&mut self, meter: &B) -> Result<(jobs::Ran, jobs::Spent), Fault> {
        jobs::unit(&mut self.host, self.root, &self.shape, meter)
    }

    /// The write-first step: exactly one unit when work is pending, however much budget is left; O(1)
    /// otherwise. Units take no scratch: every rewrite keeps its working state in its own blocks.
    pub fn write_step<S: Scratch, B: Budget>(&mut self, _scratch: &mut S, b: &B) -> Result<Wake, Fault> {
        if self.wake()? == Wake::Idle {
            return Ok(Wake::Idle);
        }
        self.unit(b)?;
        self.wake()
    }

    /// Runs units until the budget is spent, checking `used() < limit()` before each; O(1) once no work is
    /// pending.
    pub fn maintain<S: Scratch, B: Budget>(&mut self, _scratch: &mut S, b: &B) -> Result<Progress, Fault> {
        loop {
            if b.used() >= b.limit() {
                self.settle()?;
                return Ok(Progress::More);
            }
            if self.wake()? == Wake::Idle || self.unit(b)?.0 == jobs::Ran::Nothing {
                return Ok(Progress::Done);
            }
        }
    }

    /// The exact sum of the non-null values of integer leaf `leaf` (an integer of at most 64 bits, Nat, Int,
    /// u128 or i128) over every live row, and their count: one read of the leaf's total, at any table size.
    /// A sum past i128 is written to `buf` (`Total::Big`), or is `Scratch { need }` when `buf` is shorter.
    /// `BadRef` for a leaf of another type.
    pub fn sum(&self, leaf: u32, buf: &mut [u8]) -> Result<(totals::Total, u64), Fault> {
        let h = &self.host;
        let o = self.shape.ordinal_of(h, leaf)?.ok_or(Fault::BadRef)?;
        let tail = root::u64_at(h, self.root, root::TAIL)?;
        let l = self.shape.leaf(h, o)?;
        let at = hot::place_of(h, &self.shape, tail, &l)?;
        totals::total(h, tail, at, &l, buf)
    }

    /// The smallest (`largest`: the largest) non-null value of integer leaf `leaf` (an integer of at most 64
    /// bits, Nat, Int, u128 or i128) over every live row, `None` when no live row holds one: one record per
    /// group, and the rows of a group whose record does not give it (`extremes`). A value past i128 is written
    /// to `buf` as a sum is (`Total::Big`), or is `Scratch { need }` when `buf` is shorter. `BadRef` for a leaf
    /// of another type.
    pub fn extreme(&self, leaf: u32, largest: bool, buf: &mut [u8]) -> Result<Option<totals::Total>, Fault> {
        let h = &self.host;
        let o = self.shape.ordinal_of(h, leaf)?.ok_or(Fault::BadRef)?;
        let l = self.shape.leaf(h, o)?;
        extremes::extreme(h, self.root, &self.shape, &l, largest, buf)
    }

    /// The live rows whose value of leaf `f.leaf` meets `f.cond`, from rid `next` on in rid order, at most
    /// `limit` of them, each handed to `out` in leaf order and ended with its rid; returns the rid to go on
    /// from, or `None` when no row is left. Rids never move, so a scan resumed after writes hands out every row
    /// that stays in the table and meets the filter then exactly once. A row's value of the leaf is read in the
    /// group the scan has open, and an integer leaf's sealed groups are skipped or passed whole by their
    /// minimum and maximum (`filter`). `BadRef` for a leaf the table does not have; `BadValue` for a filter
    /// value that is not of the leaf's type, a null end of a range, or a value still in the message's argument.
    pub fn scan_where<S: RowSink>(
        &self,
        f: &filter::Filter<'_>,
        next: u64,
        limit: u64,
        out: &mut S,
    ) -> Result<Option<u64>, Fault> {
        filter::scan(&self.host, self.root, &self.shape, f, next, limit, out)
    }

    /// The live rows that meet `f` (every live row when it is `None`), as `scan_where` finds them, handed out
    /// with only some of their leaves: each row's leaves that `leaves` names, in the order named (a key leaf
    /// only when named), then `row_end` with its rid. Only those leaves are read, and the filter's, each in the
    /// group the scan has open, under the overlay. Returns the rid to go on from, or `None` when no row is left.
    /// `BadRef` for a leaf of `leaves` the table does not have live; the filter's faults are `scan_where`'s.
    pub fn scan_cells<S: RowSink>(
        &self,
        leaves: &[u32],
        f: Option<&filter::Filter<'_>>,
        next: u64,
        limit: u64,
        out: &mut S,
    ) -> Result<Option<u64>, Fault> {
        filter::cells(&self.host, self.root, &self.shape, leaves, f, next, limit, out)
    }

    /// How many live rows meet `f`, as `scan_where` finds them, reading no other cell of a row.
    pub fn count_where(&self, f: &filter::Filter<'_>) -> Result<u64, Fault> {
        filter::count(&self.host, self.root, &self.shape, f)
    }

    /// The exact sum of the non-null values of integer leaf `leaf` (as `sum`) over the live rows that meet `f`,
    /// and their count, reading no other cell of a row. A sum past i128 is written to `buf` (`Total::Big`); a
    /// `buf` too short for it, or for a value past i128, is `Scratch { need }`. `BadRef` for a leaf of another
    /// type; the filter's faults are `scan_where`'s.
    pub fn sum_where(&self, f: &filter::Filter<'_>, leaf: u32, buf: &mut [u8]) -> Result<(totals::Total, u64), Fault> {
        filter::sum(&self.host, self.root, &self.shape, f, leaf, buf)
    }

    /// The average of the non-null values of integer leaf `leaf`, `None` when it has none: its sum, rounded
    /// to the nearest f64, over their count. `buf` and the faults are `sum`'s.
    pub fn average(&self, leaf: u32, buf: &mut [u8]) -> Result<Option<f64>, Fault> {
        let (t, n) = self.sum(leaf, buf)?;
        if n == 0 {
            return Ok(None);
        }
        let s = match t {
            totals::Total::Small(x) => x as f64,
            totals::Total::Big(k) => totals::to_f64(buf.get(..k).unwrap_or(&[])),
        };
        Ok(Some(s / n as f64))
    }

    /// Adopts a new schema image (`evolve`): columns and record fields added, columns and fields dropped,
    /// secondary indexes removed (a dropped column takes those holding it) and added (built
    /// over the rows in maintenance units, `Busy` to counts and walks until then), in any combination; every
    /// other change is refused before anything is written.
    pub fn change_schema(&mut self, image: &[u8]) -> Result<Wake, Fault> {
        self.adopt()?;
        let shape = evolve::change_schema(&mut self.host, self.root, &self.shape, image)?;
        self.shape = shape;
        self.settle()
    }
}
