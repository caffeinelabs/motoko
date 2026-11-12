//! The table: rows in a write buffer and sealed row groups, a primary index, the secondary indexes its schema
//! declares (kept current by every write), the integer leaves' totals kept exact and their extremes per group,
//! and the maintenance that compacts and drops sealed groups.
//!
//! A row type is flat: every leaf is a row-level scalar or `()` (records, tuples and options of scalars
//! flatten into such leaves).
//!
//! Hosts give rows and keys as cells through `RowSource`, and receive them through `RowSink`; a leaf is
//! named by its leaf id. A point operation (`get`, `upsert`, `remove`) builds the key's encoding E(k) on the
//! stack when it fits `KEY_STACK` bytes and in the host's scratch otherwise, so a key has no length cap. The
//! insert that fills the open group seals it in the same call; a sealed row's update writes its cells in place
//! or in patch columns. Maintenance runs in steps: `write_step`, which a host calls before every write, runs
//! exactly one when work is pending, and `maintain` runs steps until its budget is spent.

pub mod buffer;
pub mod cell;
pub mod clear;
pub mod cur;
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
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::host::{wake_from_state, Budget, Progress, Scratch, Wake};
use crate::index::secondary::{Secondary, Start};
use crate::key::{LenSink, Sink, SliceSink};
use crate::mem::{put, put_u16, rd_u16, PageHost, Region};
use crate::schema::SchemaView;
use crate::tree::{Dir, Path};

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
/// secondary index's key may be longer: it is written into the index's key log instead (`indexes`). Small, as
/// the stack buffer is zeroed at every call.
pub const KEY_STACK: usize = 64;

/// Bytes of the stack buffer a stored key value is copied into when it does not lie in one piece, and of one
/// piece a longer one is compared in: room for the longest value of a key of `KEY_STACK` bytes.
const PIECE: usize = KEY_STACK + KEY_STACK / 4;

/// Checks that a schema image's leaves are live scalars or `()`, and its index declarations, before anything
/// is allocated.
fn check_schema(image: &[u8]) -> Result<(), Fault> {
    let v = SchemaView::open(image)?;
    if image.len() as u64 > PAGE || v.leaves == 0 {
        return Err(rt(Corruption::Leaf, 0));
    }
    for o in 0..v.leaves {
        let l = v.leaf(o)?;
        if matches!(Ty::of(&l), None | Some(Ty::Dropped)) || l.dropped != 0 {
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
    piece: &mut [u8; PIECE],
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
                let k = n.wrapping_sub(at).min(PIECE as u64);
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
    host: &'h mut H,
    root: u64,
    shape: Shape,
    /// Bytes the host copied from the message's argument during the last write.
    pub filled: u64,
}

impl<'h, H: PageHost> Table<'h, H> {
    /// A new, empty table with table uid `uid` and the given schema image; returns it and its root block.
    pub fn create(host: &'h mut H, uid: u64, schema_image: &[u8]) -> Result<(Table<'h, H>, u64), Fault> {
        check_schema(schema_image)?;
        let tbl = crate::area::new_root(&mut *host)?;
        let sp = crate::area::image_page_alloc(&mut *host, tbl)?;
        put(host.get_mut(Region::Page(sp))?, 0, schema_image)?;
        let shape = Shape::open(&*host, sp)?;
        let mut t = Table { host, root: tbl, shape, filled: 0 };
        t.init(uid)?;
        Ok((t, tbl))
    }

    /// The root's fields, the shape and the declared indexes of an empty table with table uid `uid`, and its
    /// group 0: what `create` and `clear` write once the root holds its area records.
    fn init(&mut self, uid: u64) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        let kind = if shape.set { root::KIND_SET } else { root::KIND_TABLE };
        root::init(h, tbl, kind, uid, shape.hash, shape.version, shape.page)?;
        shape.store(h, tbl)?;
        indexes::declare(h, tbl, shape)?;
        self.open_first_group()?;
        root::set_u64(&mut *self.host, tbl, root::NEXT_GROUP, 1)
    }

    /// Removes every row, with its index entries, at a cost that does not grow with the table: the table gets
    /// the empty structures `create` gives it (its row type, declared indexes and table uid kept; sums, counts
    /// and extremes start again from no rows), and the old ones are parked (`clear`). Their memory goes back in
    /// the steps of the table's maintenance, after its compactions: each write runs one, `maintain` runs more.
    /// Scans and walks resumed after it hand out only rows written after it; row ids start again.
    pub fn clear(&mut self) -> Result<Wake, Fault> {
        let uid = root::u64_at(&*self.host, self.root, root::UID)?;
        let parked = clear::park(&mut *self.host, self.root)?;
        self.init(uid)?;
        clear::link(&mut *self.host, self.root, parked)?;
        self.wake()
    }

    /// Group 0's descriptor and first tail block, with a fresh hot state whose running-record leaf list
    /// follows the rule of room over the schema's leaves.
    fn open_first_group(&mut self) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        let g = 0u64;
        let hh = shape.hot_h;
        let nb = buffer::new_block_at(h, tbl, g, 0, 0, None, hh)?;
        let d = buffer::new_descriptor(h, tbl, g, nb)?;
        let ext = extremes::create(h, tbl, shape, g)?;
        crate::mem::put_u64(h.get_mut(Region::Block(d))?, buffer::D_EXT, ext)?;
        {
            let b = h.get_mut(Region::Block(nb))?;
            crate::mem::put_u64(b, buffer::B_DESC, d)?;
            buffer::zero_hot(b, shape, hh)?;
        }
        // The running-record leaf list: the integer leaves whose records fit, in ordinal order.
        let base = crate::stats::running::LIST_AT;
        let mut r = 0u64;
        let mut recs = 0u64;
        for o in 0..shape.leaves {
            let l = shape.leaf(&*h, o)?;
            let Some(k) = l.ty.running() else { continue };
            let bytes = k.bytes();
            let list = group::align8(r.wrapping_add(1).wrapping_mul(2));
            if base.wrapping_add(list).wrapping_add(recs).wrapping_add(bytes) <= crate::consts::HOT_MAX {
                let b = h.get_mut(Region::Block(nb))?;
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

    /// The table whose root block is `root`. Its shape is the one its root holds (`Shape::stored`), O(1). When
    /// the root holds none of its current schema (another table format wrote it, or its format, schema page,
    /// schema hash or schema version word changed), the shape is derived from the stored schema image again,
    /// its schema hash checked, and stored.
    pub fn open(host: &'h mut H, root: u64) -> Result<Table<'h, H>, Fault> {
        root::check(&*host, root)?;
        let shape = match Shape::stored(&*host, root)? {
            Some(s) => s,
            None => {
                let sp = root::u64_at(&*host, root, root::SCHEMA_PAGE)?;
                let s = Shape::open(&*host, sp)?;
                s.store(&mut *host, root)?;
                s
            }
        };
        Ok(Table { host, root, shape, filled: 0 })
    }

    pub fn root(&self) -> u64 {
        self.root
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// The host, shared, for checkers and tests.
    pub fn host(&self) -> &H {
        self.host
    }

    /// The host, writable, for test hosts whose logs and counters a test reads between calls.
    pub fn host_mut(&mut self) -> &mut H {
        self.host
    }

    /// Live rows: the hot state's count, O(1).
    pub fn len(&self) -> Result<u64, Fault> {
        let tail = root::u64_at(&*self.host, self.root, root::TAIL)?;
        buffer::u64_of(&*self.host, tail, buffer::H_LIVE)
    }

    pub fn is_empty(&self) -> Result<bool, Fault> {
        Ok(self.len()? == 0)
    }

    /// E(k) from a key's cells into `s`. The leaves after the last key leaf are not looked at.
    fn encode_key<K: RowSource + ?Sized, T: Sink>(&self, key: &mut K, s: &mut T) -> Result<(), Fault> {
        let mut left = self.shape.key_leaves;
        for o in 0..self.shape.leaves {
            if left == 0 {
                break;
            }
            let l = self.shape.leaf(&*self.host, o)?;
            if l.key {
                left = left.wrapping_sub(1);
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

    /// The primary index, `None` when not made yet, and the lookup of E(k) in it, for a write: a hit is the row
    /// the write finds, a miss the position its new entry takes. `path` is the caller's, for the lookup.
    #[inline(always)]
    fn look(&self, e: &[u8], path: &mut Path) -> Result<(Option<Primary>, primary::Lookup), Fault> {
        let prim = Primary::read(&*self.host, self.root)?;
        let look = match &prim {
            Some(p) => primary::lookup(&*self.host, p, e, path)?,
            None => primary::Lookup { found: None, vacant: None },
        };
        Ok((prim, look))
    }

    /// `f` of the live row of E(k), opened once for all its reads, and its rid; `None` when there is none. With
    /// `CHECK`, the row's key leaves are first checked to encode to E(k); a read leaves that out, since the lookup
    /// compared every byte of E(k). `path` is the caller's, for the lookup.
    fn read_key<const CHECK: bool, T, F>(&self, e: &[u8], path: &mut Path, f: F) -> Result<Option<T>, Fault>
    where
        F: FnOnce(u64, &read::Live) -> Result<T, Fault>,
    {
        let Some(p) = Primary::read(&*self.host, self.root)? else { return Ok(None) };
        let Some((rid, _, _)) = primary::lookup(&*self.host, &p, e, path)?.found else { return Ok(None) };
        f(rid, self.live_key::<CHECK>(&read::live(&*self.host, self.root, rid), rid, e)?).map(Some)
    }

    /// Where the live row `rid` that the primary index holds for E(k) lies, after checking that its key leaves
    /// encode to E(k).
    fn row_of(&self, rid: u64, e: &[u8]) -> Result<read::Row, Fault> {
        Ok(self.live_key::<true>(&read::live(&*self.host, self.root, rid), rid, e)?.row())
    }

    /// The live row `read::live` found for `rid`, borrowed where it lies (the opened row is large, and moving it
    /// out of the result copies it), after checking, with `CHECK`, that its key leaves encode to E(k) `e`.
    fn live_key<'f, const CHECK: bool>(
        &self,
        found: &'f Result<Option<read::Live>, Fault>,
        rid: u64,
        e: &[u8],
    ) -> Result<&'f read::Live, Fault> {
        let row = match found {
            Ok(Some(row)) => row,
            Ok(None) => return Err(rt(Corruption::Link, rid)),
            Err(f) => return Err(*f),
        };
        if CHECK && !self.key_matches(row, e)? {
            return Err(rt(Corruption::Encoding, rid));
        }
        Ok(row)
    }

    /// Whether a stored row's key leaves encode to `e`. A long value is read and compared in pieces of
    /// `PIECE` bytes, and a Nat's or Int's LEB128 bytes a window at a time, so no key is too long. The leaves
    /// after the last key leaf are not looked at.
    fn key_matches(&self, row: &read::Live, e: &[u8]) -> Result<bool, Fault> {
        let h = &*self.host;
        let mut cmp = KeyCompare { want: e, at: 0, same: true };
        let mut piece = [0u8; PIECE];
        let mut left = self.shape.key_leaves;
        for o in 0..self.shape.leaves {
            if !cmp.same || left == 0 {
                break;
            }
            let l = self.shape.leaf(h, o)?;
            if !l.key {
                continue;
            }
            left = left.wrapping_sub(1);
            let v = match *row {
                read::Live::Buffered(at) => {
                    let b = overlay::buffered(h, &at)?;
                    overlay::buffered_leaf(h, &self.shape, &b, &l)?
                }
                read::Live::Sealed(ref s, pos) => {
                    let gv = group::GroupView::from_meta(h, &s.meta);
                    match read::sealed_cell(&gv, s, &l, gv.slot(pos)?, pos, &mut piece) {
                        Ok(x) => {
                            cell::enc_key(&mut cmp, &l, &smallest(x, l.ty))?;
                            continue;
                        }
                        // A value longer than `piece`, compared in pieces below.
                        Err(Fault::Scratch { .. }) => overlay::sealed_value(h, s, &l, pos)?,
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
        Ok(self.read_key::<false, _, _>(e, &mut Path::new(), |rid, row| self.emit(row, rid, out))?.is_some())
    }

    fn emit<S: RowSink>(&self, row: &read::Live, rid: u64, out: &mut S) -> Result<(), Fault> {
        let h = &*self.host;
        match *row {
            read::Live::Buffered(at) => read::emit_buffered(h, &self.shape, &overlay::buffered(h, &at)?, out)?,
            read::Live::Sealed(ref s, pos) => {
                let gv = group::GroupView::from_meta(h, &s.meta);
                read::emit_sealed(&gv, &self.shape, s, pos, gv.slot(pos)?, out)?;
            }
        }
        out.row_end(rid)
    }

    /// The row with rid `rid`, handed to `out`; `false` when no live row has it.
    pub fn get_rid<S: RowSink>(&self, rid: u64, out: &mut S) -> Result<bool, Fault> {
        let found = read::live(&*self.host, self.root, rid);
        let Some(row) = found.as_ref().map_err(|e| *e)? else { return Ok(false) };
        self.emit(row, rid, out)?;
        Ok(true)
    }

    /// The row with key k, as `get` finds it, handed to `out` with only the leaves `leaves` names, in the order
    /// named, as `scan_cells` hands a row out; `false` when there is none. Only those leaves are read. `BadRef`
    /// for a leaf of `leaves` the table does not have live, when the row is there.
    pub fn get_leaves<S: Scratch, K: RowSource + ?Sized, O: RowSink>(
        &self,
        scratch: &mut S,
        key: &mut K,
        leaves: &[u32],
        out: &mut O,
    ) -> Result<bool, Fault> {
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(key, &mut kb, scratch)?;
        let read = |rid, row: &read::Live| filter::row_cells(&*self.host, &self.shape, leaves, row, rid, out);
        Ok(self.read_key::<false, _, _>(e, &mut Path::new(), read)?.is_some())
    }

    /// The row with rid `rid`, handed to `out` as `get_leaves` hands it; `false` when no live row has it.
    pub fn get_rid_leaves<S: RowSink>(&self, rid: u64, leaves: &[u32], out: &mut S) -> Result<bool, Fault> {
        let found = read::live(&*self.host, self.root, rid);
        let Some(row) = found.as_ref().map_err(|e| *e)? else { return Ok(false) };
        filter::row_cells(&*self.host, &self.shape, leaves, row, rid, out)?;
        Ok(true)
    }

    /// The rows whose cells of secondary index `index`'s parts encode to the key the cells of `key` give (by
    /// leaf id, as a row's): one descent of the index's tree. A key of any length: one longer than `KEY_STACK` is
    /// encoded into `scratch`. `BadRef` when the table has no secondary index `index`.
    pub fn count<S: Scratch, K: RowSource + ?Sized>(
        &self,
        scratch: &mut S,
        index: u8,
        key: &mut K,
    ) -> Result<i128, Fault> {
        indexes::count(&*self.host, self.root, &self.shape, index, key, scratch)
    }

    /// `count` of the rows whose first `parts` parts under index `index` (all of them when it has no more) have
    /// the cells of `key`: one descent, then a step per index entry of those rows.
    pub fn count_prefix<S: Scratch, K: RowSource + ?Sized>(
        &self,
        scratch: &mut S,
        index: u8,
        parts: u8,
        key: &mut K,
    ) -> Result<i128, Fault> {
        indexes::count_prefix(&*self.host, self.root, &self.shape, index, parts, key, scratch)
    }

    /// The key the cells of `cells` give secondary index `index` (by leaf id, as a row's), its full encoding, at
    /// the start of `out`, for a walk to start at; its length. `BadRef` when the table has no secondary index
    /// `index`.
    pub fn index_key<K: RowSource + ?Sized>(&self, index: u8, cells: &mut K, out: &mut [u8]) -> Result<usize, Fault> {
        indexes::key(&*self.host, &self.shape, index, cells, out)
    }

    /// Secondary index `index`'s rows in key order, from `start` in direction `dir` (going backward, the largest
    /// keys first; the rows of one key in rid order either way), at most `limit` of them: each handed to `row` as
    /// its key, the full encoding, and its rid. Returns how many were handed out; a walk stopped by its limit
    /// resumes with `Start::After` the last row. `buf` holds the walk's keys, two of them, so it needs twice the
    /// longest key it meets (and at least 2·W): a key that does not fit is `Scratch { need }`, the bytes `buf`
    /// needs, before any row of the key is handed out, and the walk resumes after the last row it handed out
    /// with a buffer of `need` bytes. `BadRef` when the table has no secondary index `index`.
    pub fn walk<F: FnMut(&[u8], u64) -> Result<(), Fault>>(
        &self,
        index: u8,
        dir: Dir,
        start: Start<'_>,
        limit: u64,
        buf: &mut [u8],
        row: F,
    ) -> Result<u64, Fault> {
        walks::walk(&*self.host, self.root, &self.shape, index, dir, start, limit, buf, row)
    }

    /// Secondary index `index`'s groups in key order: each distinct key its rows hold, the full encoding (a key
    /// whose first part is null first, as the index orders it), with how many live rows hold it. From the first
    /// key above `after` (the smallest when `None`), at most `limit` of them, each handed to `group`; returns how
    /// many were handed out, fewer than `limit` only when no group is left. A call stopped by its limit is
    /// resumed with `after` the last key it handed out; counts are read at each call, so writes between calls
    /// change the groups still to come as they change the index. A group takes two descents of the tree,
    /// whatever its rows. `buf` holds two keys (the last handed out and the next), so it needs twice the longest
    /// key it meets (and at least 2·W): a key that does not fit is `Scratch { need }`, the bytes `buf` needs,
    /// before it is handed out, and the walk resumes after the last key it handed out with a buffer of `need`
    /// bytes. `BadRef` when the table has no secondary index `index`.
    pub fn group_counts<F: FnMut(&[u8], i128) -> Result<(), Fault>>(
        &self,
        index: u8,
        after: Option<&[u8]>,
        limit: u64,
        buf: &mut [u8],
        group: F,
    ) -> Result<u64, Fault> {
        walks::groups(&*self.host, self.root, &self.shape, index, after, limit, buf, group)
    }

    /// Secondary index `index`'s smallest key whose first part is not null (the smallest value of its column,
    /// for an index of one column), its full encoding, at the start of `out`; its length, or `None` when no row
    /// holds one. A key that does not fit `out` is `Scratch { need }`, the bytes `out` needs. `BadRef` when the
    /// table has no secondary index `index`.
    pub fn min_key(&self, index: u8, out: &mut [u8]) -> Result<Option<usize>, Fault> {
        walks::end_key(&*self.host, self.root, &self.shape, index, false, out)
    }

    /// Secondary index `index`'s largest key whose first part is not null, as `min_key`.
    pub fn max_key(&self, index: u8, out: &mut [u8]) -> Result<Option<usize>, Fault> {
        walks::end_key(&*self.host, self.root, &self.shape, index, true, out)
    }

    /// Bytes [at, at + buf.len()) of a long value handed out as `CellOut::Arg`.
    pub fn read_large(&self, rid: u64, leaf: u32, at: u64, buf: &mut [u8]) -> Result<usize, Fault> {
        read::read_large(&*self.host, self.root, &self.shape, rid, leaf, at, buf)
    }

    /// Inserts the row, or updates the row with its key to the row's values (core Map's `add`). `Unique`, with
    /// nothing written, when a unique index holds the row's key under it for another row.
    pub fn upsert<S: Scratch, R: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        row: &mut R,
    ) -> Result<(Upsert, Wake), Fault> {
        if self.shape.unique != 0 {
            self.unique_row(&mut indexes::Dyn(row), scratch)?;
        }
        let (rid, inserted) = self.write_row(scratch, row, &mut Path::new(), None)?;
        let u = if inserted { Upsert::Inserted(rid) } else { Upsert::Replaced(rid) };
        Ok((u, self.settle()?))
    }

    /// (rid, inserted) of an upsert. E(k) is built on the stack when it fits, else in the scratch (its length
    /// counted by a plan that only counts it). A row image longer than an inline one takes the scratch too, after
    /// E(k) when E(k) is there, asked for before anything is written. `found` is the lookup of E(k) when the caller
    /// made it, into `path`. Inlined in both callers, where a call would make each insert dearer.
    #[inline(always)]
    fn write_row<S: Scratch, R: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        row: &mut R,
        path: &mut Path,
        found: Option<(Option<Primary>, primary::Lookup)>,
    ) -> Result<(u64, bool), Fault> {
        self.filled = 0;
        let mut kb = [0u8; KEY_STACK];
        let first = {
            let mut ks = SliceSink::new(&mut kb);
            match row::plan::<true, _, _, _>(&self.shape, &*self.host, row, &mut ks) {
                Ok(plan) => Some((plan, ks.len)),
                Err(Fault::Scratch { .. }) => None,
                Err(e) => return Err(e),
            }
        };
        // E(k)'s length on the stack, or its bytes in the scratch and the bytes a new row's image takes after them.
        let image = |plan: &row::Plan| if plan.len > row::STAGE as u64 { plan.len } else { 0 };
        let (plan, key) = match first {
            Some((plan, n)) => (plan, Ok(n)),
            None => {
                let mut c = LenSink(0);
                let plan = row::plan::<true, _, _, _>(&self.shape, &*self.host, row, &mut c)?;
                let need = c.0.checked_add(image(&plan)).ok_or(Fault::Scratch { need: u64::MAX })?;
                let buf = scratch.get(usize::try_from(need).map_err(|_| Fault::Scratch { need })?)?;
                let at = usize::try_from(c.0).unwrap_or(usize::MAX);
                let (k, big) = buf.split_at_mut_checked(at).ok_or(Fault::Scratch { need })?;
                self.encode_key(row, &mut SliceSink::new(k))?;
                (plan, Err((&*k, big)))
            }
        };
        let e = match &key {
            Ok(n) => kb.get(..*n).unwrap_or(&[]),
            Err((k, _)) => k,
        };
        // One lookup of E(k) in the primary index. A primary index not made yet is empty; it is made only when the
        // row goes in.
        let (prim, look) = match found {
            Some(found) => found,
            None => self.look(e, path)?,
        };
        if let Some((rid, _, _)) = look.found {
            let found = self.row_of(rid, e)?;
            // The indexes' entries move first, while the stored row still holds its old cells; the scratch a new
            // image past the stack takes is asked for before.
            indexes::check_source(&self.shape, &*self.host, row)?;
            if plan.len > row::STAGE as u64 && matches!(found, read::Row::Buffered(_)) {
                scratch.get(usize::try_from(plan.len).map_err(|_| Fault::Scratch { need: plan.len })?)?;
            }
            self.index_update(found, row, rid, &mut kb, scratch, path)?;
            let (h, tbl) = (&mut *self.host, self.root);
            match found {
                read::Row::Buffered(at) => {
                    self.filled = write::splice(h, tbl, &self.shape, row, &at, None, plan.len, scratch)?;
                }
                read::Row::Sealed { page, pos } => {
                    self.filled = update::update_sealed(h, tbl, &self.shape, row, page, pos, update::Every)?;
                }
            }
            return Ok((rid, false));
        }
        indexes::check_source(&self.shape, &*self.host, row)?;
        let (e, big): (_, &mut [u8]) = match key {
            Ok(n) => (
                kb.get(..n).unwrap_or(&[]),
                match image(&plan) {
                    0 => &mut [],
                    need => scratch.get(usize::try_from(need).map_err(|_| Fault::Scratch { need })?)?,
                },
            ),
            Err(k) => k,
        };
        let mut p = match prim {
            Some(p) => p,
            None => Primary::create(&mut *self.host, self.root, self.shape.key_w, self.shape.key_hash)?,
        };
        let (rid, filled) = write::insert(&mut *self.host, self.root, &self.shape, row, &plan, big)?;
        self.filled = filled;
        primary::insert(&mut *self.host, self.root, &mut p, e, rid, look.vacant, path)?;
        self.index_insert(row, rid, &mut kb, scratch, path)?;
        Ok((rid, true))
    }

    /// Sets the leaves `leaves` names (by leaf id) of the row with key k to the cells of `src`, keeping its other
    /// leaves: in a sealed group only those leaves are read and written, each changed one as an upsert writes it;
    /// a row still in the open group gets a new image, its stored one with those leaves' cells and items replaced
    /// (`write::splice`). Only the secondary indexes with a part among the changed leaves move. Returns the row's
    /// rid, or `None` when no row has key k: with nothing written, or with `insert`, after inserting `src`, which
    /// then has a cell for every leaf, as `upsert` does, in the same lookup. `BadRef` for a leaf the table does not
    /// have; `BadValue` for a cell its leaf's type refuses, or a key leaf given another value; `Unique` when a
    /// unique index holds the row's new key under it for another row. Every refusal comes before the first write.
    pub fn update<S: Scratch, K: RowSource + ?Sized, R: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        key: &mut K,
        leaves: &[u32],
        src: &mut R,
        insert: bool,
    ) -> Result<(Option<u64>, Wake), Fault> {
        self.filled = 0;
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(key, &mut kb, scratch)?;
        let mut path = Path::new();
        let found = self.look(e, &mut path)?;
        let Some((rid, _, _)) = found.1.found else {
            if !insert {
                return Ok((None, self.wake()?));
            }
            if self.shape.unique != 0 {
                self.unique_row(&mut indexes::Dyn(src), scratch)?;
            }
            self.write_row(scratch, src, &mut path, Some(found))?;
            return Ok((None, self.settle()?));
        };
        let row = self.live_key::<true>(&read::live(&*self.host, self.root, rid), rid, e)?.row();
        // The indexes whose key a changed leaf is part of; the cells checked before anything is written.
        let mut moved = 0u64;
        let (h, tbl, shape) = (&*self.host, self.root, &self.shape);
        for &id in leaves {
            let l = shape.leaf(h, shape.ordinal_of(h, id)?.ok_or(Fault::BadRef)?)?;
            let v = row::cell_of(src, &l)?;
            let mut parts = 0u64;
            for i in 0..shape.index_decls {
                if indexes::decl(shape, h, i)?.is_some_and(|d| d.ords().contains(&l.ordinal)) {
                    parts |= 1u64.wrapping_shl(u32::try_from(i).unwrap_or(0));
                }
            }
            // A leaf that is neither a key leaf nor part of an index is written as it comes, unread.
            if (l.key || parts != 0) && !cur::same(h, &l, &read::value_of(h, tbl, shape, rid, id)?, &v)? {
                if l.key {
                    return Err(Fault::BadValue { leaf: id });
                }
                moved |= parts;
            }
        }
        // The entries leave under the stored row's keys, and come back under its new ones.
        let live = read::Live::open(&*self.host, row)?;
        let unique = moved & self.shape.unique;
        if unique != 0 && self.held(&mut indexes::Dyn(src), Some((&live, leaves)), Some(rid), unique, scratch)? != 0 {
            return Err(Fault::Unique);
        }
        match row {
            read::Row::Sealed { page, pos } => {
                self.index_stored(&live, moved, false, rid, &mut kb, scratch, &mut path)?;
                self.filled = update::update_sealed(&mut *self.host, tbl, &self.shape, src, page, pos, leaves)?;
            }
            read::Row::Buffered(at) => {
                // The scratch a new image past the stack takes: a refusal comes before the first write.
                let len = write::spliced(&*self.host, &self.shape, src, &at, leaves)?;
                if len > row::STAGE as u64 {
                    scratch.get(usize::try_from(len).map_err(|_| Fault::Scratch { need: len })?)?;
                }
                self.index_stored(&live, moved, false, rid, &mut kb, scratch, &mut path)?;
                self.filled = write::splice(&mut *self.host, tbl, &self.shape, src, &at, Some(leaves), len, scratch)?;
            }
        }
        self.index_stored(&read::Live::open(&*self.host, row)?, moved, true, rid, &mut kb, scratch, &mut path)?;
        Ok((Some(rid), self.settle()?))
    }

    /// Removes the row with key k; returns its rid.
    pub fn remove<S: Scratch, K: RowSource + ?Sized>(
        &mut self,
        scratch: &mut S,
        key: &mut K,
    ) -> Result<(Option<u64>, Wake), Fault> {
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(key, &mut kb, scratch)?;
        let Some(mut p) = Primary::read(&*self.host, self.root)? else { return Ok((None, self.wake()?)) };
        let mut path = Path::new();
        let look = primary::lookup(&*self.host, &p, e, &mut path)?;
        let Some((rid, node, i)) = look.found else { return Ok((None, self.wake()?)) };
        let found = read::live(&*self.host, self.root, rid);
        let row = self.live_key::<true>(&found, rid, e)?;
        self.drop_row(row, rid, &mut p, (node, i), &mut path, &mut kb, scratch)?;
        Ok((Some(rid), self.settle()?))
    }

    /// Takes the live row `rid`, lying at `row`, out of the primary index `p` (its entry at `(node, i)`, which
    /// `path` reached), then out of every secondary index, read from the row's cells, then out of the table. Out
    /// of line: `remove` and `remove_range` share one copy.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn drop_row<S: Scratch>(
        &mut self,
        row: &read::Live,
        rid: u64,
        p: &mut Primary,
        (node, i): (u64, u64),
        path: &mut Path,
        kb: &mut [u8; KEY_STACK],
        scratch: &mut S,
    ) -> Result<(), Fault> {
        primary::remove(&mut *self.host, self.root, p, node, i, path)?;
        self.index_stored(row, u64::MAX, false, rid, kb, scratch, path)?;
        match *row {
            read::Live::Buffered(at) => write::delete_buffered(&mut *self.host, self.root, &self.shape, &at),
            read::Live::Sealed(ref s, pos) => update::delete_sealed(&mut *self.host, self.root, &self.shape, s, pos),
        }
    }

    /// Removes the rows of secondary index `index` from the first of key `from` (the index's first row when `None`)
    /// to the last of key `to` (its last when `None`), both full encodings, as `remove` removes a row: returns how
    /// many, with the maintenance state. Each row is found by its place in the index's counted tree (one descent),
    /// and its E(k) read from it. `BadRef` when the table has no secondary index `index`.
    pub fn remove_range<S: Scratch>(
        &mut self,
        scratch: &mut S,
        index: u8,
        from: Option<&[u8]>,
        to: Option<&[u8]>,
    ) -> Result<(u64, Wake), Fault> {
        indexes::find(&self.shape, &*self.host, index)?;
        let (lo, n) = Secondary::read(&*self.host, self.root, index)?.range(&*self.host, from, to)?;
        for _ in 0..n {
            let ix = Secondary::read(&*self.host, self.root, index)?;
            let rid = ix.rid_at(&*self.host, lo)?.ok_or(rt(Corruption::Count, lo))?;
            let found = read::live(&*self.host, self.root, rid);
            let row = self.live_key::<false>(&found, rid, &[])?;
            let mut kb = [0u8; KEY_STACK];
            let e = indexes::primary_key(&self.shape, &*self.host, row, &mut kb, scratch)?;
            let mut p = Primary::read(&*self.host, self.root)?.ok_or(rt(Corruption::Link, rid))?;
            let mut path = Path::new();
            match primary::lookup(&*self.host, &p, e, &mut path)?.found {
                Some((r, node, i)) if r == rid => {
                    self.drop_row(row, rid, &mut p, (node, i), &mut path, &mut kb, scratch)?
                }
                _ => return Err(rt(Corruption::Link, rid)),
            }
        }
        Ok((n, self.settle()?))
    }

    /// Changes the table's row type to the one schema image `image` gives (`evolve::change`): the same rows
    /// and indexes, new leaves null in every row stored before, dropped leaves no longer stored, and the
    /// secondary indexes the image no longer declares removed. The open group is sealed first when it has a
    /// position. A refused image is `Schema`, and changes nothing.
    pub fn evolve(&mut self, image: &[u8]) -> Result<(), Fault> {
        self.shape = evolve::change(&mut *self.host, self.root, &self.shape, image)?;
        Ok(())
    }

    /// The maintenance state, written when it changes.
    fn settle(&mut self) -> Result<Wake, Fault> {
        jobs::settle(&mut *self.host, self.root)
    }

    /// The maintenance state, the freeing byte included, O(1).
    pub fn wake(&self) -> Result<Wake, Fault> {
        Ok(wake_from_state(rd_u16(self.host.get(Region::Block(self.root))?, root::STATE)?))
    }

    /// One maintenance step; what it ran.
    pub fn step(&mut self) -> Result<jobs::Ran, Fault> {
        jobs::step(&mut *self.host, self.root, &self.shape)
    }

    /// The write-first step: exactly one step when work is pending; O(1) otherwise. Steps take no scratch: each
    /// job bounds its own step.
    pub fn write_step(&mut self) -> Result<Wake, Fault> {
        if self.wake()? == Wake::Idle {
            return Ok(Wake::Idle);
        }
        self.step()?;
        self.wake()
    }

    /// Runs steps until the budget is spent, checking `used() < limit()` before each; O(1) once no work is
    /// pending.
    pub fn maintain<B: Budget>(&mut self, b: &B) -> Result<Progress, Fault> {
        loop {
            if b.used() >= b.limit() {
                self.settle()?;
                return Ok(Progress::More);
            }
            if self.wake()? == Wake::Idle || self.step()? == jobs::Ran::Nothing {
                return Ok(Progress::Done);
            }
        }
    }

    /// The exact sum of the non-null values of integer leaf `leaf` (an integer of at most 64 bits, Nat, Int,
    /// u128 or i128) over every live row, and their count: one read of the leaf's total, at any table size.
    /// A sum past i128 is written to `buf` (`Total::Big`), or is `Scratch { need }` when `buf` is shorter.
    /// `BadRef` for a leaf of another type.
    pub fn sum(&self, leaf: u32, buf: &mut [u8]) -> Result<(totals::Total, u64), Fault> {
        let h = &*self.host;
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
        let h = &*self.host;
        let o = self.shape.ordinal_of(h, leaf)?.ok_or(Fault::BadRef)?;
        let l = self.shape.leaf(h, o)?;
        extremes::extreme(h, self.root, &self.shape, &l, largest, buf)
    }

    /// The live rows whose value of leaf `f.leaf` meets `f.cond` (every live row when `f` is `None`), from rid
    /// `next` on in rid order, at most `limit` of them, each handed to `out` in leaf order and ended with its rid;
    /// returns the rid to go on from, or `None` when no row is left. Rids never move, so a scan resumed after writes hands out every row
    /// that stays in the table and meets the filter then exactly once. A row's value of the leaf is read in the
    /// group the scan has open, and an integer leaf's sealed groups are skipped or passed whole by their
    /// minimum and maximum (`filter`). `BadRef` for a leaf the table does not have; `BadValue` for a filter
    /// value that is not of the leaf's type, a null end of a range, or a value still in the message's argument.
    pub fn scan_where<S: RowSink>(
        &self,
        f: Option<&filter::Filter<'_>>,
        next: u64,
        limit: u64,
        out: &mut S,
    ) -> Result<Option<u64>, Fault> {
        filter::scan(&*self.host, self.root, &self.shape, f, next, limit, out)
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
        filter::cells(&*self.host, self.root, &self.shape, leaves, f, next, limit, out)
    }

    /// The live rows below rid `before`, newest first, at most `limit` of them, each handed to `out` whole, as
    /// `scan_where` hands it, or with only the leaves `leaves` names, as `scan_cells` does. Returns the rid to go
    /// on from (the rows below it are left), or `None` when no row is left. Rids never move, so a scan resumed
    /// after writes hands out every row that stays in the table exactly once.
    pub fn scan_back<S: RowSink>(
        &self,
        leaves: Option<&[u32]>,
        before: u64,
        limit: u64,
        out: &mut S,
    ) -> Result<Option<u64>, Fault> {
        filter::back(&*self.host, self.root, &self.shape, leaves, before, limit, out)
    }

    /// How many live rows meet `f`, as `scan_where` finds them, reading no other cell of a row.
    pub fn count_where(&self, f: &filter::Filter<'_>) -> Result<u64, Fault> {
        filter::count(&*self.host, self.root, &self.shape, f)
    }

    /// The exact sum of the non-null values of integer leaf `leaf` (as `sum`) over the live rows that meet `f`,
    /// and their count, reading no other cell of a row. A sum past i128 is written to `buf` (`Total::Big`); a
    /// `buf` too short for it, or for a value past i128, is `Scratch { need }`. `BadRef` for a leaf of another
    /// type; the filter's faults are `scan_where`'s.
    pub fn sum_where(&self, f: &filter::Filter<'_>, leaf: u32, buf: &mut [u8]) -> Result<(totals::Total, u64), Fault> {
        filter::sum(&*self.host, self.root, &self.shape, f, leaf, buf)
    }
}
