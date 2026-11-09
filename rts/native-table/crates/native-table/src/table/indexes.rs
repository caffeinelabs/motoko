//! The table's secondary indexes: every index declaration of the schema image that is not a primary one, each
//! kept in one `index::secondary` tree, whose descriptor the table makes when it is created.
//!
//! A declaration gives an id (1 to 63), the tree's W, and 1 to 7 parts, each a leaf of the row, in key order.
//! A row's key under an index is E of its parts' values in that order, each with its null tag when the leaf is
//! nullable: what `cell::enc_key` writes for the leaf, as for the primary key. Every write keeps every index
//! current within itself: an insert takes the new row's entry in each index, a remove drops the row's, and an
//! update moves the row's entry in each index whose key it changes. A key's count is one descent of its
//! index's tree, plus the index's uploaded segments. An index's rows are walked in key order, from an end or a
//! key (`index::secondary::sorted`); its ends give the smallest and the largest value.
//!
//! A key is built in a stack buffer of `KEY_STACK` bytes. A longer one (a long Text, Blob or `[u8; N]`, a huge
//! Nat or Int) takes no buffer: a write first finds its length, then writes it into a new record of the index's
//! key log, a piece at a time (a stored row's parts a step at a time, as a build writes a key over units); its
//! descents compare the stored keys with that record (`LongKey`), each entry made under it takes a copy, and
//! the record is handed back. An update compares the new cells with it, piece by piece, to see whether the key
//! changes. A count encodes it into the caller's scratch. The walks (`walks`) take the caller's buffer for the
//! keys they hold and answer `Scratch` with the bytes it needs when a key is longer.
//!
//! A schema change may remove indexes: those its image no longer declares, and those holding a leaf it drops.
//! A removed index's tree and key log go to the drain (`remove`), which frees them in bounded units, and its
//! descriptor is cleared; writes keep only the indexes the new image declares.
//!
//! A schema change may add indexes (`add`). A new index of a table holding rows is built over them in
//! maintenance units (`build_unit`), in rid order: rids are never reused, so the rows the build takes are
//! those below the next rid at the change, and every row inserted later is taken by its insert. Until the
//! build reaches a stored row, a write of that row leaves the index alone (the build takes the row's current
//! cells when it reaches it, or nothing when it is gone); a row the build passed is kept by every write. The
//! build's position lies in the index's descriptor, and root job slot 1 holds the indexes being built ({ids
//! u64 @0}, bit i for id i). A count or a walk of an index being built is `Busy`. A row too long for a unit
//! has its key written into a record of the index's key log over units, a step at a time, from a saved
//! position (the part, the bytes of its value taken, the key's bytes written, and the facts of a Nat or Int
//! beyond i128 read from its LEB form) kept with the key's first W bytes in a state block of the log, which
//! the descriptor names; the row is taken once the key has ended.

use crate::consts::{GROUP_SLOTS, INDEXES_MAX, PATH_STACK, UNIT_BLOCKS_MAX, W_MAX};
use crate::fault::{rt, Corruption, Fault, ImageId, SchemaError};
use crate::index::area_nodes::AreaView;
use crate::index::secondary::{LongAt, LongKey, Secondary, AT_FLAGS, KIND_SECONDARY, MID, RAISED, SEEN};
use crate::key::probe::{ovf_field, row_field, StoredCmp};
use crate::key::{self, KeyKind, KeyPart, KeySpec, LenSink, Sink, SliceSink};
use crate::mem::{copy_bytes, rd_u16, rd_u32, rd_u8, PageHost, Region};
use crate::schema::{SchemaView, LEAF_NULLABLE};
use crate::tree::{IndexShape, Path};

use super::cur::{self, SVal};
use super::group::GroupView;
use super::jobs::{Clock, Spent};
use super::layout::{key_kind, LeafInfo, Shape, Ty};
use super::primary::Primary;
use super::{buffer, cell, drain, overlay, read, root, row, RowSource, Table, KEY_STACK, PIECE};

/// Declarations a schema image may hold.
const DECLS_MAX: u64 = 64;
/// Bytes of a declaration in the schema image.
pub(super) const DECL: u64 = crate::schema::DECL_BYTES;
/// Declaration flags bit 0: a primary index.
pub(super) const PRIMARY: u8 = 1;
/// The most parts a key has.
const PARTS_MAX: usize = 7;

/// A secondary index as its declaration gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decl {
    pub id: u8,
    pub w: u64,
    parts: u8,
    /// Its parts' leaf ordinals, in key order.
    ords: [u16; PARTS_MAX],
}

impl Decl {
    /// Its parts' leaf ordinals, in key order.
    pub fn ords(&self) -> &[u16] {
        self.ords.get(..usize::from(self.parts)).unwrap_or(&[])
    }

    /// Its key spec's parts, in key order (the first `ords().len()`): each part's leaf, key kind and
    /// nullability.
    pub fn parts<H: PageHost + ?Sized>(&self, shape: &Shape, h: &H) -> Result<[KeyPart; PARTS_MAX], Fault> {
        let mut parts = [KeyPart { leaf: 0, kind: KeyKind::Unit, nullable: false }; PARTS_MAX];
        for (p, &o) in parts.iter_mut().zip(self.ords()) {
            let l = shape.leaf(h, u64::from(o))?;
            *p = KeyPart { leaf: l.id, kind: key_kind(l.ty), nullable: l.nullable };
        }
        Ok(parts)
    }

    /// Its key spec, over the parts `Decl::parts` gave.
    pub fn spec<'a>(&self, parts: &'a [KeyPart; PARTS_MAX]) -> KeySpec<'a> {
        KeySpec { parts: parts.get(..self.ords().len()).unwrap_or(&[]) }
    }
}

/// The offset of declaration i in the schema page.
fn decl_at(shape: &Shape, i: u64) -> u64 {
    // i < 64 and the declarations lie in the schema page.
    shape.index_at.wrapping_add(i.wrapping_mul(DECL))
}

/// Declaration i of the table's schema image (i below `shape.index_decls`): `None` for a primary one, and for
/// one with a part whose leaf is dropped, which declares no index (a dropped column takes its indexes along).
pub fn decl<H: PageHost + ?Sized>(shape: &Shape, h: &H, i: u64) -> Result<Option<Decl>, Fault> {
    let b = h.get(Region::Page(shape.page))?;
    let o = decl_at(shape, i);
    if rd_u8(b, o.wrapping_add(1))? & PRIMARY != 0 {
        return Ok(None);
    }
    let parts = rd_u8(b, o.wrapping_add(4))?;
    if parts == 0 || usize::from(parts) > PARTS_MAX {
        return Err(rt(Corruption::Count, o));
    }
    let mut ords = [0u16; PARTS_MAX];
    for (k, slot) in ords.iter_mut().enumerate().take(usize::from(parts)) {
        let leaf = rd_u32(b, o.wrapping_add(8).wrapping_add((k as u64).wrapping_mul(4)))?;
        let Some(ord) = shape.ordinal_of(h, leaf)? else { return Ok(None) };
        *slot = u16::try_from(ord).map_err(|_| rt(Corruption::Leaf, o))?;
    }
    Ok(Some(Decl { id: rd_u8(b, o)?, w: u64::from(rd_u16(b, o.wrapping_add(2))?), parts, ords }))
}

/// Checks a schema image's index declarations before a table is made with it or changes to it: at most 64;
/// each secondary one with an id in 1..=63 no other has, a W its tree can take, and 1 to 7 parts, each naming a
/// leaf of the row, with the leaf's nullability.
pub fn check(v: &SchemaView<'_>) -> Result<(), Fault> {
    if u64::from(v.indexes) > DECLS_MAX {
        return Err(Fault::Schema(SchemaError::TooManyIndexes));
    }
    let b = v.bytes;
    let mut ids = 0u64;
    for i in 0..u64::from(v.indexes) {
        // i < 64.
        let o = u64::from(v.layout.indexes).wrapping_add(i.wrapping_mul(DECL));
        if rd_u8(b, o.wrapping_add(1))? & PRIMARY != 0 {
            continue;
        }
        let id = rd_u8(b, o)?;
        let bit = 1u64.wrapping_shl(u32::from(id));
        if id == 0 || u64::from(id) >= INDEXES_MAX || ids & bit != 0 {
            return Err(rt(Corruption::Range, o));
        }
        ids |= bit;
        if IndexShape::new(u64::from(rd_u16(b, o.wrapping_add(2))?), 0).is_none() {
            return Err(rt(Corruption::Layout, o));
        }
        let parts = rd_u8(b, o.wrapping_add(4))?;
        if parts == 0 || usize::from(parts) > PARTS_MAX {
            return Err(rt(Corruption::Count, o));
        }
        let nullable_parts = rd_u8(b, o.wrapping_add(5))?;
        for k in 0..u64::from(parts) {
            let po = o.wrapping_add(8).wrapping_add(k.wrapping_mul(4));
            let leaf = rd_u32(b, po)?;
            let mut nullable = None;
            for x in 0..v.leaves {
                let l = v.leaf(x)?;
                if l.id == leaf {
                    nullable = Some(l.flags & LEAF_NULLABLE != 0);
                    break;
                }
            }
            let nullable = nullable.ok_or(rt(Corruption::Leaf, po))?;
            if (nullable_parts >> k & 1 != 0) != nullable {
                return Err(rt(Corruption::Flags, po));
            }
        }
    }
    Ok(())
}

/// Makes the descriptor of every secondary index of a new table, its key spec hash over its parts; with the
/// primary index's descriptor first, which their directory block holds.
pub fn declare<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<(), Fault> {
    let mut primary = false;
    for i in 0..shape.index_decls {
        let Some(d) = decl(shape, &*h, i)? else { continue };
        if !primary {
            Primary::ensure(h, tbl, shape.key_w, shape.key_hash)?;
            primary = true;
        }
        let parts = d.parts(shape, &*h)?;
        Secondary::declare(h, tbl, d.id, d.w, d.spec(&parts).hash(KIND_SECONDARY))?;
    }
    Ok(())
}

/// Root job slot 1: the secondary indexes being built.
const BUILDING: u64 = root::JOB_SLOTS + 32;

/// The secondary indexes of table `tbl` being built, bit i for id i.
pub fn building<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<u64, Fault> {
    root::u64_at(h, tbl, BUILDING)
}

fn bit(id: u8) -> u64 {
    1u64.wrapping_shl(u32::from(id))
}

/// Whether index `id` holds the entry of the stored row `rid`, `building` the indexes being built: always,
/// unless its build has not reached the row yet.
fn holds<H: PageHost + ?Sized>(h: &H, tbl: u64, building: u64, id: u8, rid: u64) -> Result<bool, Fault> {
    if building & bit(id) == 0 {
        return Ok(true);
    }
    let (next, end) = Secondary::build_state(h, tbl, id)?;
    Ok(rid < next || rid >= end)
}

/// Index `index`'s declaration for a count or a walk: `BadRef` when the table has no secondary index of that
/// id, `Busy` while it is being built.
pub(super) fn find_built<H: PageHost + ?Sized>(shape: &Shape, h: &H, tbl: u64, index: u8) -> Result<Decl, Fault> {
    let d = find(shape, h, index)?;
    if building(h, tbl)? & bit(index) != 0 {
        return Err(Fault::Busy);
    }
    Ok(d)
}

/// Adds secondary index `d` of the table's new schema (`shape`) to table `tbl`: its descriptor made (the
/// primary index's first, which their directory block holds), and, when the table holds rows, its build
/// started over the rids below the next one.
pub fn add<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, d: &Decl) -> Result<(), Fault> {
    Primary::ensure(h, tbl, shape.key_w, shape.key_hash)?;
    let parts = d.parts(shape, &*h)?;
    Secondary::declare(h, tbl, d.id, d.w, d.spec(&parts).hash(KIND_SECONDARY))?;
    let (g, _, tail) = buffer::open_group(&*h, tbl)?;
    if buffer::u64_of(&*h, tail, buffer::H_LIVE)? == 0 {
        return Ok(());
    }
    let pos = buffer::u16_of(&*h, tail, buffer::H_OPEN_POS)?;
    let end = g.checked_mul(GROUP_SLOTS).and_then(|x| x.checked_add(pos)).ok_or(rt(Corruption::Range, g))?;
    Secondary::set_build_state(h, tbl, d.id, 0, end)?;
    let b = building(&*h, tbl)?;
    root::set_u64(h, tbl, BUILDING, b | bit(d.id))
}

/// Removes secondary index `id` from table `tbl`: its tree and key log go to the drain, which frees them in
/// bounded units, and its descriptor is cleared (a build under way ends with it).
pub fn remove<H: PageHost>(h: &mut H, tbl: u64, id: u8) -> Result<(), Fault> {
    let b = building(&*h, tbl)?;
    if b & bit(id) != 0 {
        root::set_u64(h, tbl, BUILDING, b & !bit(id))?;
    }
    let s = Secondary::read(&*h, tbl, id)?;
    if s.root != 0 {
        let mut r = drain::Rec::new(drain::K_INDEX_TREE, s.root, 0);
        r.digits[0] = u16::try_from(s.shape.w).map_err(|_| rt(Corruption::Layout, s.shape.w))?;
        drain::push(h, tbl, &r)?;
    }
    if s.log.radix.root != 0 {
        drain::push(h, tbl, &drain::Rec::new(drain::K_KEY_LOG, s.log.radix.root, s.log.radix.height))?;
    }
    Secondary::clear(h, tbl, id)
}

/// Index `index`'s declaration; `BadRef` when the table has no secondary index of that id.
pub fn find<H: PageHost + ?Sized>(shape: &Shape, h: &H, index: u8) -> Result<Decl, Fault> {
    for i in 0..shape.index_decls {
        if let Some(d) = decl(shape, h, i)? {
            if d.id == index {
                return Ok(d);
            }
        }
    }
    Err(Fault::BadRef)
}

/// Writes the key the cells of `src` give index `d` into `s`.
pub fn enc_source<H: PageHost + ?Sized, R: RowSource + ?Sized, S: Sink>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    src: &mut R,
    s: &mut S,
) -> Result<(), Fault> {
    for &o in d.ords() {
        let l = shape.leaf(h, u64::from(o))?;
        let v = row::cell_of(src, &l)?;
        cell::enc_key(s, &l, &v)?;
    }
    Ok(())
}

/// The key the cells of `src` give index `d`, at the start of `buf`; returns its length.
pub fn key_of_source<H: PageHost + ?Sized, R: RowSource + ?Sized>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    src: &mut R,
    buf: &mut [u8; KEY_STACK],
) -> Result<usize, Fault> {
    let mut s = SliceSink::new(buf);
    enc_source(shape, h, d, src, &mut s)?;
    Ok(s.len)
}

/// Checks, before a write changes anything, that the row `src` gives has a key under every secondary index:
/// each part's cell encodes (a cell given as an argument value does not). A key of any length is taken.
pub fn check_source<H: PageHost + ?Sized, R: RowSource + ?Sized>(
    shape: &Shape,
    h: &H,
    src: &mut R,
) -> Result<(), Fault> {
    for i in 0..shape.index_decls {
        let Some(d) = decl(shape, h, i)? else { continue };
        enc_source(shape, h, &d, src, &mut LenSink(0))?;
    }
    Ok(())
}

/// Whether a key part of leaf `l` is read from a stored row in pieces: a Text, Blob or `[u8; N]`.
fn in_pieces(l: &LeafInfo) -> bool {
    matches!(l.ty, Ty::Text | Ty::Blob | Ty::Fixed(_))
}

/// Writes the key the stored row `r` has under index `d` into `s`, from its current cells (the overlay's
/// values); `piece` takes a value that does not lie in one piece of table memory.
fn enc_live<H: PageHost + ?Sized, S: Sink>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: &read::Live,
    piece: &mut [u8],
    s: &mut S,
) -> Result<(), Fault> {
    match *r {
        read::Live::Buffered(at) => {
            let b = overlay::buffered(h, &at)?;
            let rr = row::RowRef::new(h, b.loc)?;
            // One walk while the parts' ordinals rise; a part before the walk's place starts it again.
            let mut walk = row::Walk::of(shape, &rr)?;
            let mut next = 0u64;
            for &o in d.ords() {
                let o = u64::from(o);
                if o < next {
                    walk = row::Walk::of(shape, &rr)?;
                    next = 0;
                }
                while next < o {
                    walk.skip(&shape.leaf(h, next)?);
                    next = next.wrapping_add(1);
                }
                let l = shape.leaf(h, o)?;
                let v = overlay::buffered_value(h, shape, &b, &rr, &mut walk, &l)?;
                next = o.wrapping_add(1);
                cell::enc_key(s, &l, &read::sval_val(h, &l, &v, piece)?)?;
            }
        }
        read::Live::Sealed(ref sg, pos) => {
            let gv = GroupView::from_meta(h, &sg.meta);
            let slot = gv.slot(pos)?;
            for &o in d.ords() {
                let l = shape.leaf(h, u64::from(o))?;
                cell::enc_key(s, &l, &read::sealed_cell(&gv, shape, sg, &l, slot, pos, piece)?)?;
            }
        }
    }
    Ok(())
}

/// Leaf `l`'s current value in the stored row `r` (the overlay's). O(L) for a buffered row's offsets.
fn current<H: PageHost + ?Sized>(shape: &Shape, h: &H, r: read::Row, l: &LeafInfo) -> Result<SVal, Fault> {
    match r {
        read::Row::Buffered(at) => overlay::buffered_leaf(h, shape, &overlay::buffered(h, &at)?, l),
        read::Row::Sealed { page, pos } => overlay::sealed_value(h, shape, &overlay::sealed(h, page)?, l, pos),
    }
}

/// A change of one index entry: the row `rid` taken under `key`, or dropped from it. The entry must be new to
/// the tree, or held by it: anything else is a corrupt index.
pub fn change<H: PageHost>(
    h: &mut H,
    tbl: u64,
    index: u8,
    key: &[u8],
    rid: u64,
    take: bool,
    path: &mut Path,
) -> Result<(), Fault> {
    // `for_write`, its usual case read in place: returning the descriptor through a call copies it.
    let mut ix = Secondary::read(&*h, tbl, index)?;
    if ix.pending {
        ix = for_write(h, tbl, index, path)?;
    }
    let before = ix;
    let done = if take { ix.insert(h, tbl, key, rid, path)? } else { ix.delete(h, tbl, key, rid, path)? };
    if !done {
        return Err(rt(Corruption::Link, rid));
    }
    if ix != before {
        ix.write_tree(h, tbl)?;
    }
    Ok(())
}

/// A build's row `rid` taken under `key` through the build's descriptor `ix`, kept current, unless its insert
/// splits a run of two or more full nodes that `fits` says does not fit (`Secondary::insert_fitting`): the run,
/// or `None` when nothing changed.
fn take_fitting<H: PageHost, F: FnOnce(u64) -> bool>(
    h: &mut H,
    tbl: u64,
    ix: &mut Secondary,
    key: &[u8],
    rid: u64,
    path: &mut Path,
    fits: F,
) -> Result<Option<u64>, Fault> {
    let before = *ix;
    let taken = ix.insert_fitting(h, tbl, key, rid, path, fits)?;
    if *ix != before {
        ix.write_tree(h, tbl)?;
    }
    Ok(taken)
}

/// The key the stored row `r` has under index `d`, at the start of `buf`: its length, or `None` when it is
/// longer than `KEY_STACK`.
#[inline(always)]
fn key_of_stored<H: PageHost + ?Sized>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: &read::Live,
    piece: &mut [u8; PIECE],
    buf: &mut [u8; KEY_STACK],
) -> Result<Option<usize>, Fault> {
    let mut s = SliceSink::new(buf);
    match enc_live(shape, h, d, r, piece, &mut s) {
        Ok(()) => Ok(Some(s.len)),
        Err(Fault::Scratch { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Takes the row `rid` in index `d` under the key the cells of `src` give.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn take_source<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    src: &mut R,
    rid: u64,
    buf: &mut [u8; KEY_STACK],
    path: &mut Path,
) -> Result<(), Fault> {
    match key_of_source(shape, &*h, d, src, buf) {
        Ok(n) => change(h, tbl, d.id, buf.get(..n).unwrap_or(&[]), rid, true, path),
        Err(Fault::Scratch { .. }) => {
            change_long(h, tbl, shape, d, &mut Cells::Source(src), rid, true, None::<&mut R>, path).map(|_| ())
        }
        Err(e) => Err(e),
    }
}

/// Where the cells of a key longer than `KEY_STACK` come from: a row source, or a stored row.
enum Cells<'a, R: ?Sized> {
    Source(&'a mut R),
    Stored(read::Row),
}

/// A sink that hands each piece to `put` with the host.
struct Fwd<'a, H, F> {
    h: &'a mut H,
    put: &'a mut F,
}

impl<H, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>> Sink for Fwd<'_, H, F> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        (self.put)(self.h, bytes)
    }
}

/// Hands the key `cells` give index `d` to `put` in order, with the host, which no read holds between the
/// pieces: a source's cells encoded whole, a stored row's parts a step at a time (`part_step`).
fn feed<H: PageHost, R: RowSource + ?Sized, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>>(
    h: &mut H,
    shape: &Shape,
    d: &Decl,
    cells: &mut Cells<'_, R>,
    mut put: F,
) -> Result<(), Fault> {
    for &o in d.ords() {
        let l = shape.leaf(&*h, u64::from(o))?;
        match cells {
            Cells::Source(src) => {
                let v = row::cell_of(&mut **src, &l)?;
                cell::enc_key(&mut Fwd { h: &mut *h, put: &mut put }, &l, &v)?;
            }
            Cells::Stored(r) => {
                let val = current(shape, &*h, *r, &l)?;
                let (mut facts, mut from) = (None, 0u64);
                while let Some(f) = part_step(h, (&l, o, &val), &mut facts, from, (&mut put, &mut || true))? {
                    from = f;
                }
            }
        }
    }
    Ok(())
}

/// Bytes [at, at + n) of the stored value `v` of the Text, Blob or `[u8; N]` leaf of ordinal `o` (of a Nat or
/// Int stored long, its LEB form), n as many as `out` takes, at its start: n and the value's length; `None`
/// for a null.
fn bytes_at<H: PageHost + ?Sized>(
    h: &H,
    v: &SVal,
    o: u16,
    at: u64,
    out: &mut [u8],
) -> Result<Option<(usize, u64)>, Fault> {
    match *v {
        SVal::Null => Ok(None),
        SVal::B { len: total, .. } | SVal::Huge { len: total, .. } => {
            let k = usize::try_from(total.saturating_sub(at)).unwrap_or(usize::MAX).min(out.len());
            cur::read(h, v, at, out.get_mut(..k).unwrap_or(&mut []))?;
            Ok(Some((k, total)))
        }
        _ => Err(rt(Corruption::Encoding, u64::from(o))),
    }
}

/// The widest key area, which holds a long key's first bytes.
// W_MAX = 1,016.
#[allow(clippy::cast_possible_truncation)]
const HEAD: usize = W_MAX as usize;

/// Writes the key `cells` give index `ix` (`d`), longer than `KEY_STACK`, into a new record of the index's key
/// log, and its first W bytes into `head`: its length found first, then its bytes written. Returns the
/// record's locator and the key's length.
#[allow(clippy::too_many_arguments)]
fn stream_key<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    ix: &mut Secondary,
    cells: &mut Cells<'_, R>,
    head: &mut [u8; HEAD],
) -> Result<(u64, u64), Fault> {
    let mut len = 0u64;
    feed(h, shape, d, cells, |_, b| {
        len = len.saturating_add(b.len() as u64);
        Ok(())
    })?;
    let loc = ix.new_key(h, tbl, len)?;
    let (w, ix) = (ix.shape.w, &*ix);
    let mut at = 0u64;
    feed(h, shape, d, cells, |h, b| {
        let mut b = b;
        if at < w {
            let from = usize::try_from(at).map_err(|_| rt(Corruption::Bounds, at))?;
            let n = usize::try_from(w.wrapping_sub(at)).unwrap_or(0).min(b.len());
            let (now, later) = b.split_at_checked(n).ok_or(rt(Corruption::Bounds, at))?;
            copy_bytes(head.get_mut(from..from.wrapping_add(n)).ok_or(rt(Corruption::Layout, w))?, now);
            (b, at) = (later, at.wrapping_add(n as u64));
        }
        if !b.is_empty() {
            ix.write_key(h, tbl, loc, len, at, b)?;
            at = at.wrapping_add(b.len() as u64);
        }
        Ok(())
    })?;
    if at != len {
        return Err(rt(Corruption::Encoding, at));
    }
    Ok((loc, len))
}

/// `change` under a key longer than `KEY_STACK`, which `cells` give: written into the index's key log as a
/// record first, the tree changed through it (a `LongKey`), then the record handed back. With `unless`, nothing
/// changes when that source's cells give the same key, compared piece by piece with the record. Returns
/// whether the tree changed.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn change_long<H: PageHost, R: RowSource + ?Sized, U: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    cells: &mut Cells<'_, R>,
    rid: u64,
    take: bool,
    unless: Option<&mut U>,
    path: &mut Path,
) -> Result<bool, Fault> {
    let mut ix = for_write(h, tbl, d.id, path)?;
    let before = ix;
    let mut head = [0u8; HEAD];
    let (loc, len) = stream_key(h, tbl, shape, d, &mut ix, cells, &mut head)?;
    let w = usize::try_from(ix.shape.w).map_err(|_| rt(Corruption::Layout, ix.shape.w))?;
    let key = LongKey { head: head.get(..w).ok_or(rt(Corruption::Layout, len))?, loc, len };
    let same = match unless {
        Some(src) => {
            let view = AreaView::new(&*h, ix.shape, ix.log);
            let mut c = StoredCmp::new(&view, key.head, ovf_field(row_field(0), loc))?;
            enc_source(shape, &*h, d, src, &mut c)?;
            c.same()
        }
        None => false,
    };
    let done =
        same || if take { ix.insert_long(h, tbl, key, rid, path)? } else { ix.delete_long(h, tbl, key, rid, path)? };
    ix.free_key(h, tbl, loc)?;
    if !done {
        return Err(rt(Corruption::Link, rid));
    }
    if ix != before {
        ix.write_tree(h, tbl)?;
    }
    Ok(!same)
}

impl<H: PageHost> Table<'_, H> {
    /// Takes the new row `rid` in every secondary index, under the key the cells of `src` give each; `buf` and
    /// `path` are the caller's.
    pub(super) fn index_insert<R: RowSource + ?Sized>(
        &mut self,
        src: &mut R,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut self.host, self.root, &self.shape);
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
            take_source(h, tbl, shape, &d, src, rid, buf, path)?;
        }
        Ok(())
    }

    /// Drops the stored row `rid`, lying at `r`, from every secondary index, under the key its cells give each.
    pub(super) fn index_remove(
        &mut self,
        r: &read::Live,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut self.host, self.root, &self.shape);
        let mut piece = [0u8; PIECE];
        let building = building(&*h, tbl)?;
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
            if !holds(&*h, tbl, building, d.id, rid)? {
                continue;
            }
            let Some(n) = key_of_stored(shape, &*h, &d, r, &mut piece, buf)? else {
                let cells = &mut Cells::<dyn RowSource>::Stored(r.row());
                change_long(h, tbl, shape, &d, cells, rid, false, None::<&mut dyn RowSource>, path)?;
                continue;
            };
            change(h, tbl, d.id, buf.get(..n).unwrap_or(&[]), rid, false, path)?;
        }
        Ok(())
    }

    /// Before the stored row `rid`, lying at `r`, takes the cells of `src`: moves its entry in every secondary
    /// index whose key the new cells change, from the key its stored cells give to theirs. A build writing or
    /// inserting the row's key over units, under a key the new cells change, takes the row again from its start.
    pub(super) fn index_update<R: RowSource + ?Sized>(
        &mut self,
        r: read::Row,
        src: &mut R,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut self.host, self.root, &self.shape);
        let mut piece = [0u8; PIECE];
        let building = building(&*h, tbl)?;
        // The row opened at its first key read, for all the indexes: their entries' moves leave its group as is.
        let mut live = None;
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
            if !holds(&*h, tbl, building, d.id, rid)? {
                let mut moved = false;
                for &o in d.ords() {
                    let l = shape.leaf(&*h, u64::from(o))?;
                    let v = row::cell_of(src, &l)?;
                    moved = moved || !cur::same(&*h, &l, &current(shape, &*h, r, &l)?, &v)?;
                }
                if moved {
                    restart_pending(h, tbl, d.id, rid, (&mut *buf, &mut *path))?;
                }
                continue;
            }
            let lv = match live {
                Some(ref x) => x,
                None => live.insert(read::Live::open(&*h, r)?),
            };
            let Some(n) = key_of_stored(shape, &*h, &d, lv, &mut piece, buf)? else {
                // A stored key longer than `KEY_STACK`: moved unless the new cells give it again.
                let cells = &mut Cells::<R>::Stored(r);
                if change_long(h, tbl, shape, &d, cells, rid, false, Some(&mut *src), path)? {
                    take_source(h, tbl, shape, &d, src, rid, buf, path)?;
                }
                continue;
            };
            let same = {
                let old = buf.get(..n).unwrap_or(&[]);
                let mut cmp = super::KeyCompare { want: old, at: 0, same: true };
                enc_source(shape, &*h, &d, src, &mut cmp)?;
                cmp.same && cmp.at == n
            };
            if same {
                continue;
            }
            change(h, tbl, d.id, buf.get(..n).unwrap_or(&[]), rid, false, path)?;
            take_source(h, tbl, shape, &d, src, rid, buf, path)?;
        }
        Ok(())
    }
}

/// The rows whose key under index `index` the cells of `key` give: one descent of the index's tree, plus the
/// key's count in each of its uploaded segments. A key longer than `KEY_STACK` is encoded into `scratch`.
pub fn count<H: PageHost + ?Sized, K: RowSource + ?Sized, S: crate::host::Scratch>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    key: &mut K,
    scratch: &mut S,
) -> Result<i128, Fault> {
    let d = find_built(shape, h, tbl, index)?;
    let mut buf = [0u8; KEY_STACK];
    let n = match key_of_source(shape, h, &d, key, &mut buf) {
        Ok(n) => n,
        Err(Fault::Scratch { .. }) => return count_long(h, tbl, shape, &d, key, scratch),
        Err(e) => return Err(e),
    };
    Secondary::read(h, tbl, index)?.count(h, buf.get(..n).unwrap_or(&[]))
}

/// `count` for a key longer than `KEY_STACK`: its length found first, then the key encoded into `scratch`.
#[inline(never)]
fn count_long<H: PageHost + ?Sized, K: RowSource + ?Sized, S: crate::host::Scratch>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    key: &mut K,
    scratch: &mut S,
) -> Result<i128, Fault> {
    let mut c = LenSink(0);
    enc_source(shape, h, d, key, &mut c)?;
    let need = usize::try_from(c.0).map_err(|_| Fault::Scratch { need: c.0 })?;
    let buf = scratch.get(need)?.get_mut(..need).ok_or(Fault::Scratch { need: c.0 })?;
    let mut s = SliceSink::new(buf);
    enc_source(shape, h, d, key, &mut s)?;
    let n = s.len;
    Secondary::read(h, tbl, d.id)?.count(h, buf.get(..n).unwrap_or(&[]))
}

/// The key the cells of `cells` give index `index`, at the start of `out`; its length.
pub fn key<H: PageHost + ?Sized, K: RowSource + ?Sized>(
    h: &H,
    shape: &Shape,
    index: u8,
    cells: &mut K,
    out: &mut [u8],
) -> Result<usize, Fault> {
    let d = find(shape, h, index)?;
    let mut s = SliceSink::new(out);
    enc_source(shape, h, &d, cells, &mut s)?;
    Ok(s.len)
}

/// Estimated instructions, as wasmtime counts them (fuel), of a build's state read and saved (the slot, the
/// declaration, the descriptor), of a rid whose row is gone or a group number with no group (looked up and
/// passed), of a live row taken, plus its key's bytes (its cells read and encoded, one descent and the insert,
/// the key-log record of a key longer than W written), of the descent that finds the full nodes on a row's
/// path, and of a node split ahead of a row's insert (that descent again included).
const BUILD_LOAD: u64 = 2_000;
const BUILD_SLOT_WORK: u64 = 600;
const BUILD_ROW_WORK: u64 = 12_000;
const BUILD_BYTE_WORK: u64 = 25;
const BUILD_RUN_WORK: u64 = 3_000;
const BUILD_SPLIT_WORK: u64 = 6_000;

/// The estimate of a live row taken under a key of `n` bytes.
fn build_row_work(n: u64) -> u64 {
    BUILD_ROW_WORK.saturating_add(n.saturating_mul(BUILD_BYTE_WORK))
}

/// The estimate a row's insert adds for the run of `k` full nodes on its path it splits, 0 or two or more
/// (`Secondary::insert_fitting`): `BUILD_SPLIT_WORK` for each. Such an insert takes the tree's general insert
/// rather than splitting its leaf in place.
fn run_work(k: u64) -> u64 {
    BUILD_SPLIT_WORK.saturating_mul(k)
}

/// The pages taking one more row may change, its insert and its count's splitting `splits` nodes, its key
/// longer than W (`long`) or not: the row's leaf, or each node split, the node it gives and the node taking
/// the last entry up (or a new root); the leaf of the key's count entry; with a split or a key-log record, the
/// key log's head block, two records' blocks and its radix node (a long key), the descriptor, and an
/// allocation's pages with the area's page list and radix node; the table root when the unit has not changed
/// it yet.
fn build_pages(clock: &Clock<'_>, tbl: u64, splits: u64, long: bool) -> u64 {
    if splits == 0 && !long {
        return clock.pages_for(2, 0, tbl);
    }
    let tree = if splits == 0 { 1 } else { splits.saturating_mul(2).saturating_add(1) };
    let log = if long { 4 } else { 0 };
    clock.pages_for(tree.saturating_add(4).saturating_add(log), 1, tbl)
}

/// The pages a node split ahead of a row's insert may change: the node, the node it gives, its parent or a new
/// root, the descriptor, an allocation's pages with the area's page list and radix node, and the table root.
fn split_pages(clock: &Clock<'_>, tbl: u64) -> u64 {
    clock.pages_for(6, 1, tbl)
}

/// Bytes of a stored Text, Blob or `[u8; N]` value one step of a key written over units reads. Without a `00`
/// they go into the key's record as they are; else the step escapes them `ESC_PIECE` at a time into a buffer of
/// its own while those that lie in words holding a `00` (escaped a byte at a time, at most 8 for each `00`
/// written) are fewer than `ESC_SLOW`, so a value of many `00` bytes takes as few as `ESC_SLOW` of them, and
/// writes the buffer into the record at once. Before the last piece the buffer holds at most 2 x `ESC_SLOW`
/// bytes, or, fewer than `ESC_SLOW` / 8 of them `00`, `FEED_CHUNK` + `ESC_SLOW` / 8 with the part's null tag:
/// so twice a piece always fits what is left of `ESC_BUF` (each piece escaped straight in, `SliceSink`), and a
/// step writes at most `ESC_BUF` bytes, which reach at most one fresh 4 KiB block of the record. One step, the
/// unit's start and its end stay within a unit.
const FEED_CHUNK: usize = 2_046;
const ESC_PIECE: usize = 256;
const ESC_SLOW: usize = 512;
const ESC_BUF: usize = FEED_CHUNK + ESC_SLOW / 8 + 2 * ESC_PIECE + 3;
const _: () = assert!(2 * ESC_SLOW <= FEED_CHUNK && ESC_BUF < 4_096);

/// Bytes of a Nat's or Int's LEB form beyond i128 one step of a key written over units reads.
const LEB_CHUNK: usize = 768;

/// Bytes a key part's encoding takes beyond its stored bytes (doubled for an escaped Text or Blob), at most: its
/// null tag and terminator, a long Nat's or Int's head, or a value of fixed width whole.
const PART_EXTRA: u64 = 64;

/// A stored row's key parts as located once: each part's leaf and current value, in key order.
type Located = [Option<(LeafInfo, SVal)>; PARTS_MAX];

/// The current values of the key parts of the stored row `r` under `d`, each located once, into `vals`, in
/// key order, with their leaves. Returns the stored bytes of its Text, Blob and `[u8; N]` parts and of the LEB
/// forms of its Nat and Int parts beyond i128, and the bytes its key takes at most: an escaped Text or Blob
/// doubles its stored bytes, a long Nat's or Int's magnitude takes at most its LEB form's, and a part at most
/// `PART_EXTRA` more.
fn located<H: PageHost + ?Sized>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: read::Row,
    vals: &mut Located,
) -> Result<(u64, u64), Fault> {
    let (mut stored, mut most) = (0u64, 0u64);
    let mut tally = |l: LeafInfo, v: SVal, slot: &mut Option<(LeafInfo, SVal)>| {
        let n = match v {
            SVal::B { len, .. } | SVal::Huge { len, .. } => len,
            _ => 0,
        };
        if in_pieces(&l) || matches!(l.ty, Ty::Nat | Ty::Int) {
            stored = stored.saturating_add(n);
        }
        let k = if matches!(l.ty, Ty::Text | Ty::Blob) { n.saturating_mul(2) } else { n };
        most = most.saturating_add(k).saturating_add(PART_EXTRA);
        *slot = Some((l, v));
    };
    match r {
        read::Row::Buffered(at) => {
            let b = overlay::buffered(h, &at)?;
            let rr = row::RowRef::new(h, b.loc)?;
            // One walk while the parts' ordinals rise, as `enc_live`.
            let mut walk = row::Walk::of(shape, &rr)?;
            let mut next = 0u64;
            for (&o, slot) in d.ords().iter().zip(vals.iter_mut()) {
                let o = u64::from(o);
                if o < next {
                    walk = row::Walk::of(shape, &rr)?;
                    next = 0;
                }
                while next < o {
                    walk.skip(&shape.leaf(h, next)?);
                    next = next.wrapping_add(1);
                }
                let l = shape.leaf(h, o)?;
                let v = overlay::buffered_value(h, shape, &b, &rr, &mut walk, &l)?;
                next = o.wrapping_add(1);
                tally(l, v, slot);
            }
        }
        read::Row::Sealed { page, pos } => {
            let sg = overlay::sealed(h, page)?;
            for (&o, slot) in d.ords().iter().zip(vals.iter_mut()) {
                let l = shape.leaf(h, u64::from(o))?;
                tally(l, overlay::sealed_value(h, shape, &sg, &l, pos)?, slot);
            }
        }
    }
    Ok((stored, most))
}

/// The key of the parts `vals` (`located`), at the start of `buf`: its length, or `None` when it is longer
/// than `KEY_STACK`.
fn key_of_located<H: PageHost + ?Sized>(
    h: &H,
    vals: &Located,
    piece: &mut [u8; PIECE],
    buf: &mut [u8; KEY_STACK],
) -> Result<Option<usize>, Fault> {
    let mut s = SliceSink::new(buf);
    for (l, v) in vals.iter().flatten() {
        match read::sval_val(h, l, v, &mut piece[..]).and_then(|c| cell::enc_key(&mut s, l, &c)) {
            Ok(()) => {}
            Err(Fault::Scratch { .. }) => return Ok(None),
            Err(e) => return Err(e),
        }
    }
    Ok(Some(s.len))
}

/// The caller's words of a build's key written over units (`AreaNodes::open_key`): the row's rid, the part
/// it reached, the bytes of that part's value taken, the key's bytes written.
const KW_RID: u64 = 0;
const KW_PART: u64 = 1;
const KW_FROM: u64 = 2;
const KW_OUT: u64 = 3;
/// The node the row's insert resumes from (0: the root), with its `RAISED` and `SEEN` flags (`insert_steps`).
/// A write never changes the tree under it: one that would completes the insert first (`finish_pending`).
const KW_AT: u64 = 4;
/// An insert under a key longer than `KEY_STACK` also keeps its node search's bounds and the parent frame's
/// index (20 bits each), the bytes its bounds share with the key, those found equal of the entry between them,
/// and the parent node (`LongAt`).
const KW_SPAN: u64 = 5;
const KW_LCP_LO: u64 = 6;
const KW_LCP_HI: u64 = 7;
const KW_OFF: u64 = 8;
const KW_PARENT: u64 = 9;
/// A Nat or Int part beyond i128 written over units (`leb_step`) keeps its facts once found (`LebFacts`): the
/// part they are of (its index + 1 in bits 56–59; 0: none), its sign (`LEB_NEG`) and its two's-complement
/// value's lowest non-zero byte (`LEB_LOW`), then its magnitude's length. Until they are found, `KW_FROM`
/// holds the bytes of its LEB form scanned for that lowest byte; after, the magnitude's bytes written.
const KW_LEB: u64 = 10;
const KW_LEB_MAG: u64 = 11;
/// The descent to the key's count entry under a key longer than `KEY_STACK` (`Secondary::count_step`): its
/// node with `MID`, its bounds (20 bits each), the bytes they share with the key, and those of the entry
/// between them found equal. It raises the count of a key whose row the insert passed (`SEEN`) before the row
/// is taken, and lowers back one the insert raised (`RAISED`) once the row is gone; 0 when none is under way.
const KW_CNT: u64 = 12;
const KW_CNT_SPAN: u64 = 13;
const KW_CNT_LCP_LO: u64 = 14;
const KW_CNT_LCP_HI: u64 = 15;
const KW_CNT_OFF: u64 = 16;
const LEB_PART: u32 = 56;
const LEB_NEG: u64 = 1 << 55;
const LEB_LOW: u64 = LEB_NEG - 1;

/// Where a build's key written over units stands: the part it reached, the bytes of that part's value taken,
/// and the key's bytes written (the caller's words `KW_PART`, `KW_FROM` and `KW_OUT`); and the facts of that
/// part's Nat or Int beyond i128 once a step of the unit knows them (`KW_LEB`).
#[derive(Clone, Copy)]
struct Fed {
    part: u64,
    from: u64,
    out: u64,
    facts: Option<key::LebFacts>,
}

/// One step of the key of the stored row `r` under `d`, written over units into state block `k` of `ix`'s key
/// log from where `st` stands (moved on): a step of its part (`part_step`). The part's value is taken from the
/// row's values located by the unit's caller (`vals`), else from `v`, the value the unit's last step located,
/// else located here and kept in `v`. Returns whether the key has ended.
#[allow(clippy::too_many_arguments)]
fn feed_step<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    ix: &mut Secondary,
    (k, r): (u64, read::Row),
    st: &mut Fed,
    (vals, v): (Option<&Located>, &mut Option<SVal>),
    more: &mut dyn FnMut() -> bool,
) -> Result<bool, Fault> {
    let Fed { part: p, from, out, mut facts } = *st;
    let Some((i, &o)) = usize::try_from(p).ok().and_then(|i| Some((i, d.ords().get(i)?))) else {
        return Ok(true);
    };
    let l = shape.leaf(&*h, u64::from(o))?;
    let val = match (vals.and_then(|a| a.get(i)?.as_ref()), *v) {
        (Some(&(_, x)), _) | (None, Some(x)) => x,
        (None, None) => current(shape, &*h, r, &l)?,
    };
    *v = Some(val);
    let leb = matches!(l.ty, Ty::Nat | Ty::Int) && matches!(val, SVal::B { .. } | SVal::Huge { .. });
    let tag = p.wrapping_add(1) << LEB_PART;
    if leb && facts.is_none() {
        let (word, mag) = ix.with_log(h, tbl, |m| Ok((m.key_word(k, KW_LEB)?, m.key_word(k, KW_LEB_MAG)?)))?;
        if word & !(LEB_NEG | LEB_LOW) == tag {
            let (negative, low) = (word & LEB_NEG != 0, word & LEB_LOW);
            facts = Some(key::LebFacts { len: val.len(), negative, low, mag_len: mag });
        }
    }
    let known = facts.is_some();
    let mut at = out;
    let mut put = |h: &mut H, b: &[u8]| {
        ix.with_log(h, tbl, |m| m.put_key(k, at, b))?;
        at = at.saturating_add(b.len() as u64);
        Ok(())
    };
    let (np, nf) = match part_step(h, (&l, o, &val), &mut facts, from, (&mut put, more))? {
        Some(f) => (p, f),
        None => (p.wrapping_add(1), 0),
    };
    if np != p {
        (*v, facts) = (None, None);
    } else if let Some(f) = facts.filter(|_| !known) {
        // Found by this step, the part going on: kept for the units after.
        let word = tag | if f.negative { LEB_NEG } else { 0 } | (f.low & LEB_LOW);
        ix.with_log(h, tbl, |m| {
            m.set_key_word(k, KW_LEB, word)?;
            m.set_key_word(k, KW_LEB_MAG, f.mag_len)
        })?;
    }
    *st = Fed { part: np, from: nf, out: at, facts };
    Ok(usize::try_from(np).is_ok_and(|i| i >= d.ords().len()))
}

/// One step of key part `l`, ordinal `o`, of a stored row whose current value is `val`, handed to `put` with
/// the host, from `from`: the next `FEED_CHUNK` bytes of a Text, Blob or `[u8; N]` value (escaped from where
/// the last step stopped; `from` of them taken), a step of a Nat or Int beyond i128 (`leb_step`, with its
/// facts `facts` and `more`), or any other value encoded whole. Returns where the part stands after the step,
/// or `None` once it has ended.
fn part_step<H: PageHost, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>>(
    h: &mut H,
    (l, o, val): (&LeafInfo, u16, &SVal),
    facts: &mut Option<key::LebFacts>,
    from: u64,
    (put, more): (&mut F, &mut dyn FnMut() -> bool),
) -> Result<Option<u64>, Fault> {
    if matches!(l.ty, Ty::Nat | Ty::Int) && matches!(val, SVal::B { .. } | SVal::Huge { .. }) {
        return leb_step(h, (l, o, val), facts, from, (put, more));
    }
    if !in_pieces(l) {
        // A Nat or Int up to i128, a value of another type (a Principal's 29 bytes at most), or a null, whole.
        let (mut b, mut small) = ([0u8; 40], [0u8; 32]);
        let mut s = SliceSink::new(&mut b);
        cell::enc_key(&mut s, l, &read::sval_val(&*h, l, val, &mut small)?)?;
        let n = s.len;
        put(h, b.get(..n).unwrap_or(&[]))?;
        return Ok(None);
    }
    let fixed = matches!(l.ty, Ty::Fixed(_));
    let mut chunk = [0u8; FEED_CHUNK];
    let Some((n, total)) = bytes_at(&*h, val, o, from, &mut chunk)? else {
        if !l.nullable {
            return Err(rt(Corruption::Encoding, u64::from(o)));
        }
        key::enc_opt_tag(&mut Fwd { h: &mut *h, put: &mut *put }, false)?;
        return Ok(None);
    };
    let bytes = chunk.get(..n).unwrap_or(&[]);
    let tag = from == 0 && l.nullable;
    let ends = |used: usize| from.saturating_add(used as u64) >= total;
    // Bytes of a fixed width, or with no `00`, handed on as they are; others escaped `ESC_PIECE` at a time into
    // a buffer of the step's own while those in words holding a `00` (at most 8 for each `00` written) are fewer
    // than `ESC_SLOW`, and handed on at once: a value of many `00` bytes goes on from the first byte not escaped.
    let used = if fixed || key::zero_free(bytes) {
        let f = &mut Fwd { h: &mut *h, put: &mut *put };
        if tag {
            key::enc_opt_tag(f, true)?;
        }
        f.put(bytes)?;
        if !fixed && ends(n) {
            key::enc_bytes_end(f)?;
        }
        n
    } else {
        let mut esc = [0u8; ESC_BUF];
        let f = &mut SliceSink::new(&mut esc);
        if tag {
            key::enc_opt_tag(f, true)?;
        }
        let (base, mut used) = (f.len, 0usize);
        for piece in bytes.chunks(ESC_PIECE) {
            let zeros = f.len.saturating_sub(base).saturating_sub(used);
            if used.min(zeros.saturating_mul(8)) >= ESC_SLOW {
                break;
            }
            key::enc_bytes_piece(f, piece)?;
            used = used.saturating_add(piece.len());
        }
        if ends(used) {
            key::enc_bytes_end(f)?;
        }
        put(h, f.written())?;
        used
    };
    let taken = from.saturating_add(used as u64);
    if taken < total {
        if used == 0 {
            return Err(rt(Corruption::Bounds, taken));
        }
        return Ok(Some(taken));
    }
    Ok(None)
}

/// Magnitude bytes one step of a Nat or Int beyond i128 writes: their 7-bit groups lie in at most
/// `LEB_CHUNK` + 2 bytes of its LEB form.
const LEB_STEP: usize = LEB_CHUNK / 8 * 7;

/// Estimated instructions of a step of a Nat or Int beyond i128 from its facts known to the next decision: its
/// `LEB_CHUNK` + 2 bytes read, its `LEB_STEP` key bytes made and written (a fresh block of its record at
/// most), the facts kept.
const LEB_STEP_WORK: u64 = 17_000;

/// Reads bytes [at, at + out.len()) of the stored value `v` of the leaf of ordinal `o` into `out`, all of them.
fn read_at<H: PageHost + ?Sized>(h: &H, v: &SVal, o: u16, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    match bytes_at(h, v, o, at, out)? {
        Some((n, _)) if n == out.len() => Ok(()),
        _ => Err(rt(Corruption::Bounds, at)),
    }
}

/// One step of the Nat or Int `val` of leaf `l`, ordinal `o`, beyond i128, a key part written a step at a
/// time: its canonical (signed) LEB form read `LEB_CHUNK` bytes at a time, from `from`. Until its facts are
/// known (`facts`), a step reads the form's last bytes and, for a negative, scans its next bytes for its lowest
/// non-zero byte; the step that finds them writes the part's null tag and head, and goes on when `more` finds
/// the unit's room for the rest (`LEB_STEP_WORK`). Each step then writes the next `LEB_STEP` bytes of its
/// magnitude, the most significant first (`from` of them written). Returns where the part stands after the
/// step, or `None` once it has ended.
fn leb_step<H: PageHost, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>>(
    h: &mut H,
    (l, o, val): (&LeafInfo, u16, &SVal),
    facts: &mut Option<key::LebFacts>,
    from: u64,
    (put, more): (&mut F, &mut dyn FnMut() -> bool),
) -> Result<Option<u64>, Fault> {
    let len = val.len();
    let mut win = [0u8; LEB_CHUNK + 8];
    let (f, done) = match *facts {
        Some(f) => (f, from),
        None => {
            let signed = matches!(l.ty, Ty::Int);
            let mut top = [0u8; key::LEB_TOP];
            let t = len.min(key::LEB_TOP as u64);
            let top = top.get_mut(..usize::try_from(t).unwrap_or(0)).unwrap_or(&mut []);
            read_at(&*h, val, o, len.wrapping_sub(t), top)?;
            let low = if key::leb_negative(top, signed, ImageId::Runtime)? {
                let n = len.saturating_sub(from).min(LEB_CHUNK as u64);
                let w = win.get_mut(..usize::try_from(n).unwrap_or(0)).unwrap_or(&mut []);
                read_at(&*h, val, o, from, w)?;
                match key::leb_low(w, from) {
                    Some(x) => x,
                    None if n > 0 && from.saturating_add(n) < len => return Ok(Some(from.saturating_add(n))),
                    None => return Err(rt(Corruption::Encoding, u64::from(o))),
                }
            } else {
                0
            };
            let f = key::LebFacts::new(len, signed, low, top, ImageId::Runtime)?;
            let fw = &mut Fwd { h: &mut *h, put: &mut *put };
            if l.nullable {
                key::enc_opt_tag(fw, true)?;
            }
            f.head(fw)?;
            *facts = Some(f);
            if !more() {
                return Ok(Some(0));
            }
            (f, 0)
        }
    };
    let hi = f.mag_len.saturating_sub(done);
    let lo = hi.saturating_sub(LEB_STEP as u64);
    let (a, b) = f.span(lo, hi);
    let w =
        win.get_mut(..usize::try_from(b.saturating_sub(a)).unwrap_or(usize::MAX)).ok_or(rt(Corruption::Bounds, a))?;
    read_at(&*h, val, o, a, w)?;
    let mut out = [0u8; LEB_STEP];
    let kb = out.get_mut(..usize::try_from(hi.wrapping_sub(lo)).unwrap_or(0)).unwrap_or(&mut []);
    f.key_bytes(w, a, lo, kb)?;
    put(h, kb)?;
    Ok(if lo == 0 { None } else { Some(f.mag_len.wrapping_sub(lo)) })
}

/// Estimated instructions of one step of a key written over units beyond its bytes (`BUILD_BYTE_WORK` each):
/// its value located (unless the unit located it already) and its record's next block made.
const BUILD_STEP_WORK: u64 = 25_000;

/// Estimated instructions of a unit's read-back of a key written over units once it has ended, beyond its
/// bytes (one each): the row located again, the key's state block checked, the key copied out of its record.
const BUILD_READ_WORK: u64 = 5_500;

/// The pages one step of a key written over units may change: its record's block and a fresh one with an
/// allocation's pages, the key log's radix block and a fresh one with an allocation's pages, and the
/// descriptor; for the unit's first step also the state block, the key log's head block and the table root
/// when the unit has not changed it yet.
fn feed_pages(clock: &Clock<'_>, tbl: u64, again: bool) -> u64 {
    let p = 5u64.saturating_add(clock.alloc_changes().saturating_mul(2));
    if again {
        p
    } else {
        p.saturating_add(2).saturating_add(u64::from(!clock.changed(tbl)))
    }
}

/// The build's row `rid` (`r`), its key under `d` too long for a unit: written into a record of the index's
/// key log over units, from state block `k` (0: started here, from the row's values `fresh` located by the
/// caller and the bytes its key takes at most), while the unit has room (its first step always when `lone`).
/// Where the key stands is read once a unit and kept once a unit; `ix` is the caller's, kept current. Returns
/// the state block's ordinal, the key's length once it has ended, and whether a step ran.
#[allow(clippy::too_many_arguments)]
fn feed_long<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    (rid, r, k): (u64, read::Row, u64),
    fresh: Option<(&Located, u64)>,
    ix: &mut Secondary,
) -> Result<(u64, Option<u64>, bool), Fault> {
    let opened = k == 0;
    let (k, mut st) = match (k, fresh) {
        (0, Some((_, most))) => {
            let k = ix.with_log(h, tbl, |m| {
                let k = m.open_key(most)?;
                m.set_key_word(k, KW_RID, rid)?;
                Ok(k)
            })?;
            Secondary::set_build_key(h, tbl, d.id, k)?;
            (k, Fed { part: 0, from: 0, out: 0, facts: None })
        }
        (0, None) => return Err(rt(Corruption::Link, rid)),
        (k, _) => {
            let (was, st) = ix.with_log(h, tbl, |m| {
                let st = Fed {
                    part: m.key_word(k, KW_PART)?,
                    from: m.key_word(k, KW_FROM)?,
                    out: m.key_word(k, KW_OUT)?,
                    facts: None,
                };
                Ok((m.key_word(k, KW_RID)?, st))
            })?;
            if was != rid {
                return Err(rt(Corruption::Link, rid));
            }
            (k, st)
        }
    };
    let parts = d.ords().len() as u64;
    let (mut lone, mut ran) = (lone, false);
    let est = BUILD_STEP_WORK.saturating_add(BUILD_BYTE_WORK.saturating_mul(FEED_CHUNK as u64));
    let (vals, mut v) = (fresh.map(|(a, _)| a), None);
    while st.part < parts {
        if !lone && !clock.room(*work, est, Some(feed_pages(clock, tbl, ran))) {
            break;
        }
        (lone, ran) = (false, true);
        let w = *work;
        let more = &mut || clock.room(w, LEB_STEP_WORK, Some(feed_pages(clock, tbl, true)));
        feed_step(h, tbl, shape, d, ix, (k, r), &mut st, (vals, &mut v), more)?;
        *work = work.saturating_add(est);
    }
    if ran {
        ix.with_log(h, tbl, |m| {
            m.set_key_word(k, KW_PART, st.part)?;
            m.set_key_word(k, KW_FROM, st.from)?;
            m.set_key_word(k, KW_OUT, st.out)
        })?;
    }
    if opened || ran {
        ix.write_tree(h, tbl)?;
    }
    let len = if st.part >= parts { Some(st.out) } else { None };
    Ok((k, len, ran))
}

/// Reads the key of `n` bytes the build wrote over units into state block `k` into `buf`.
fn read_long<H: PageHost>(h: &mut H, tbl: u64, id: u8, k: u64, n: u64, buf: &mut [u8]) -> Result<(), Fault> {
    let n = usize::try_from(n).map_err(|_| rt(Corruption::Bounds, n))?;
    let out = buf.get_mut(..n).ok_or(rt(Corruption::Bounds, n as u64))?;
    Secondary::read(&*h, tbl, id)?.with_log(h, tbl, |m| m.read_key(k, out))
}

/// Estimated instructions of one step of a key's second record (`AreaNodes::twin_step`): a block copied into
/// a fresh one, the state block read and written.
const BUILD_TWIN_WORK: u64 = 6_000;

/// The second record of the build's key of `len` bytes, longer than `KEY_STACK`, written over units into state
/// block `k`, for the key's count entry should the row be the key's first: made a block at a time
/// (`AreaNodes::twin_step`) while the unit has room (its first step always when `lone`), so no unit copies
/// the key; none when the key's record fits a block, which a count entry copies as one step would. A step
/// changes at most the pages of a step writing the key (`feed_pages`). `ix` is the caller's, kept current.
/// Returns whether it is made, and whether a step ran.
#[allow(clippy::too_many_arguments)]
fn twin_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    ix: &mut Secondary,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    (k, len): (u64, u64),
) -> Result<(bool, bool), Fault> {
    let mut left = ix.with_log(h, tbl, |m| m.twin_left(k, len))?;
    let (mut lone, mut ran) = (lone, false);
    while left > 0 {
        if !lone && !clock.room(*work, BUILD_TWIN_WORK, Some(feed_pages(clock, tbl, ran))) {
            break;
        }
        (lone, ran) = (false, true);
        ix.with_log(h, tbl, |m| m.twin_step(k, len))?;
        *work = work.saturating_add(BUILD_TWIN_WORK);
        left = left.wrapping_sub(1);
    }
    if ran {
        ix.write_tree(h, tbl)?;
    }
    Ok((left == 0, ran))
}

/// The insert under the build's key of `len` bytes, longer than `KEY_STACK`, written over units into state
/// block `k`, as the last unit left it: the key's first W bytes copied into `head`, its record's locator, and
/// where the insert stands (`LongAt`).
fn long_state<H: PageHost>(
    h: &mut H,
    tbl: u64,
    ix: &mut Secondary,
    (k, len): (u64, u64),
    head: &mut [u8; HEAD],
) -> Result<(u64, LongAt), Fault> {
    ix.with_log(h, tbl, |m| {
        let loc = m.key_open_record(k, len, &mut head[..])?;
        let (word, span) = (m.key_word(k, KW_AT)?, m.key_word(k, KW_SPAN)?);
        let st = LongAt {
            at: word & !AT_FLAGS,
            flags: word & AT_FLAGS,
            lo: span & SPAN_MASK,
            hi: (span >> 20) & SPAN_MASK,
            pidx: (span >> 40) & SPAN_MASK,
            lcp_lo: m.key_word(k, KW_LCP_LO)?,
            lcp_hi: m.key_word(k, KW_LCP_HI)?,
            off: m.key_word(k, KW_OFF)?,
            parent: m.key_word(k, KW_PARENT)?,
        };
        Ok((loc, st))
    })
}

/// A node's entry index in `KW_SPAN`.
const SPAN_MASK: u64 = (1 << 20) - 1;

/// Keeps where the insert under a long key stands in its state block `k` (`long_state`).
fn save_long<H: PageHost>(h: &mut H, tbl: u64, ix: &mut Secondary, k: u64, st: &LongAt) -> Result<(), Fault> {
    let span = (st.lo & SPAN_MASK) | (st.hi & SPAN_MASK) << 20 | (st.pidx & SPAN_MASK) << 40;
    ix.with_log(h, tbl, |m| {
        m.set_key_word(k, KW_AT, st.at | st.flags)?;
        m.set_key_word(k, KW_SPAN, span)?;
        m.set_key_word(k, KW_LCP_LO, st.lcp_lo)?;
        m.set_key_word(k, KW_LCP_HI, st.lcp_hi)?;
        m.set_key_word(k, KW_OFF, st.off)?;
        m.set_key_word(k, KW_PARENT, st.parent)
    })
}

/// The key `head` and `loc` give, of `len` bytes, with `ix`'s W.
fn key_of_long<'a>(ix: &Secondary, head: &'a [u8; HEAD], loc: u64, len: u64) -> Result<LongKey<'a>, Fault> {
    let w = usize::try_from(ix.shape.w).map_err(|_| rt(Corruption::Layout, ix.shape.w))?;
    Ok(LongKey { head: head.get(..w).ok_or(rt(Corruption::Layout, len))?, loc, len })
}

/// The pages one step of the descent to a long key's count entry may change (`count_steps`): the entry's node
/// or its key's record, and the key's state block.
const COUNT_PAGES: u64 = 2;

/// Changes by `d` the count of `key`, longer than `KEY_STACK`, the build's key written over units into state
/// block `k`, through a descent to its count entry made a node at a time (`Secondary::count_step`): from
/// where the last unit stopped, while the unit has room (its first step always when `lone`), each step reading
/// at most `LONG_CHUNK` bytes of the key; with no `clock` (a writer's call), to its end. Once changed, `RAISED`
/// is set (`d` = 1) or cleared (`d` = -1) in the key's word `KW_AT`. Returns whether the count is changed, and
/// whether a step ran.
#[allow(clippy::too_many_arguments)]
fn count_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    ix: &mut Secondary,
    clock: Option<&Clock<'_>>,
    work: &mut u64,
    lone: bool,
    (k, key): (u64, LongKey<'_>),
    d: i64,
) -> Result<(bool, bool), Fault> {
    let mut st = ix.with_log(h, tbl, |m| {
        let (word, span) = (m.key_word(k, KW_CNT)?, m.key_word(k, KW_CNT_SPAN)?);
        Ok(LongAt {
            at: word & !MID,
            flags: word & MID,
            lo: span & SPAN_MASK,
            hi: (span >> 20) & SPAN_MASK,
            lcp_lo: m.key_word(k, KW_CNT_LCP_LO)?,
            lcp_hi: m.key_word(k, KW_CNT_LCP_HI)?,
            off: m.key_word(k, KW_CNT_OFF)?,
            ..LongAt::default()
        })
    })?;
    let step = BUILD_LEVEL_WORK.saturating_add(LONG_BYTE_WORK.saturating_mul(LONG_CHUNK));
    let (mut lone, mut ran, mut done, mut steps) = (lone, false, false, 0u64);
    while !done {
        let mut budget = match clock {
            Some(c) if !lone && !c.room(*work, step, Some(COUNT_PAGES)) => break,
            Some(_) => LONG_CHUNK,
            // A node's search whole a step, down a tree of at most `PATH_STACK` levels.
            None if steps > PATH_STACK => return Err(rt(Corruption::Link, k)),
            None => u64::MAX,
        };
        let all = budget;
        (lone, ran, steps) = (false, true, steps.wrapping_add(1));
        done = ix.count_step(h, tbl, key, d, &mut st, &mut budget)?;
        let read = all.saturating_sub(budget);
        *work = work.saturating_add(BUILD_LEVEL_WORK.saturating_add(LONG_BYTE_WORK.saturating_mul(read)));
    }
    if ran {
        if done {
            st = LongAt::default();
        }
        let span = (st.lo & SPAN_MASK) | (st.hi & SPAN_MASK) << 20;
        ix.with_log(h, tbl, |m| {
            if done {
                let at = m.key_word(k, KW_AT)?;
                m.set_key_word(k, KW_AT, if d > 0 { at | RAISED } else { at & !RAISED })?;
            }
            m.set_key_word(k, KW_CNT, st.at | st.flags)?;
            m.set_key_word(k, KW_CNT_SPAN, span)?;
            m.set_key_word(k, KW_CNT_LCP_LO, st.lcp_lo)?;
            m.set_key_word(k, KW_CNT_LCP_HI, st.lcp_hi)?;
            m.set_key_word(k, KW_CNT_OFF, st.off)
        })?;
    }
    Ok((done, ran))
}

/// Before the build's key written over units into state block `k` is handed back with its row gone: a count
/// its insert raised under a key longer than `KEY_STACK` lowered back over units (`count_steps`). Returns
/// whether nothing is left to lower.
fn lower_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    k: u64,
) -> Result<bool, Fault> {
    let mut ix = Secondary::read(&*h, tbl, id)?;
    let (word, len) = ix.with_log(h, tbl, |m| Ok((m.key_word(k, KW_AT)?, m.key_word(k, KW_OUT)?)))?;
    if word & RAISED == 0 || len <= KEY_STACK as u64 {
        return Ok(true);
    }
    let mut head = [0u8; HEAD];
    let loc = ix.with_log(h, tbl, |m| m.key_open_record(k, len, &mut head[..]))?;
    let key = key_of_long(&ix, &head, loc, len)?;
    Ok(count_steps(h, tbl, &mut ix, Some(clock), work, lone, (k, key), -1)?.0)
}

/// Takes the build's row `rid` at the place its insert under the key of `len` bytes, longer than `KEY_STACK`,
/// written over units into state block `k`, found (`st`): the key's record made and taken by the row's entry
/// as its own, a new count entry taking the key's second record (`twin_steps`), so the key is not copied; the
/// state block is handed back with the second record when not taken (`leave_key`).
fn take_long<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    ix: &mut Secondary,
    (k, len, rid): (u64, u64, u64),
    (st, now): (&LongAt, bool),
    path: &mut Path,
) -> Result<(), Fault> {
    let mut head = [0u8; HEAD];
    let (loc, twin) = ix.with_log(h, tbl, |m| m.close_key(k, len, &mut head))?;
    Secondary::set_build_key(h, tbl, id, 0)?;
    let key = key_of_long(ix, &head, loc, len)?;
    if ix.place_own(h, tbl, key, twin, rid, st, path)? {
        ix.with_log(h, tbl, |m| m.take_twin(k))?;
    }
    leave_key(h, tbl, id, ix, k, now)?;
    ix.write_tree(h, tbl)
}

/// Bytes of the key one step of an insert under a key longer than `KEY_STACK` compares at most
/// (`Secondary::long_step`), each estimated at `LONG_BYTE_WORK`: the stored key's byte and the key's read
/// through their records.
const LONG_CHUNK: u64 = 4_096;
const LONG_BYTE_WORK: u64 = 2;

/// Estimated instructions of an insert's last step under a key longer than `KEY_STACK` beyond `BUILD_ROW_WORK`
/// (`take_long`): the row's entry, a new count entry, the row's leaf split.
const LONG_TAKE_WORK: u64 = 12_000;

/// Estimated instructions of the step before it that moves a key's record of up to 4,096 bytes into a chunk
/// (`AreaNodes::move_key`).
const BUILD_MOVE_WORK: u64 = 4_800;

/// Inserts the build's row `rid` under its key of `len` bytes, longer than `KEY_STACK`, written over units into
/// state block `k`, as `insert_steps` does, its comparisons read through the key's record and resumed inside a
/// key, so no unit reads more than `LONG_CHUNK` bytes of it a step (`Secondary::long_step`); when it passed a
/// row of the key after the build's row (`SEEN`), raises the key's count the same way (`count_steps`); then
/// moves a record that fits a chunk into it (`AreaNodes::move_key`) and takes the row (`take_long`), each a
/// step checked against the unit's room. `ix` is the caller's, kept current. Returns whether the row is in.
#[allow(clippy::too_many_arguments)]
fn long_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    (k, rid, len): (u64, u64, u64),
    (ix, path): (&mut Secondary, &mut Path),
) -> Result<bool, Fault> {
    let mut head = [0u8; HEAD];
    let (loc, mut st) = long_state(h, tbl, ix, (k, len), &mut head)?;
    let key = key_of_long(ix, &head, loc, len)?;
    let step = BUILD_LEVEL_WORK.saturating_add(LONG_BYTE_WORK.saturating_mul(LONG_CHUNK));
    let (mut lone, mut ran) = (lone, false);
    loop {
        if ix.long_ready(&*h, &st)? {
            // The insert passed a row of the key after the build's row: the key's count entry lies further on,
            // raised first through a descent of its own over units; the row's place stays where it is.
            if st.flags & (RAISED | SEEN) == SEEN {
                if ran {
                    save_long(h, tbl, ix, k, &st)?;
                    ix.write_tree(h, tbl)?;
                    ran = false;
                }
                let (raised, r) = count_steps(h, tbl, ix, Some(clock), work, lone, (k, key), 1)?;
                lone &= !r;
                if !raised {
                    return Ok(false);
                }
                st.flags |= RAISED;
                continue;
            }
            // The comparisons are done: the record moves into its chunk ahead of the take.
            if ix.with_log(h, tbl, |m| m.key_moves(k, len))? {
                if !lone && !clock.room(*work, BUILD_MOVE_WORK, Some(feed_pages(clock, tbl, false))) {
                    break;
                }
                (lone, ran) = (false, true);
                ix.with_log(h, tbl, |m| m.move_key(k, len))?;
                *work = work.saturating_add(BUILD_MOVE_WORK);
            }
            let est = BUILD_ROW_WORK.saturating_add(LONG_TAKE_WORK);
            let pages = build_pages(clock, tbl, 1, true);
            if !lone && !clock.room(*work, est, Some(pages)) {
                break;
            }
            take_long(h, tbl, id, ix, (k, len, rid), (&st, false), path)?;
            *work = work.saturating_add(est);
            return Ok(true);
        }
        if !lone && !clock.room(*work, step, Some(split_pages(clock, tbl).saturating_add(2))) {
            break;
        }
        (lone, ran) = (false, true);
        let mut budget = LONG_CHUNK;
        ix.long_step(h, tbl, key, rid, &mut st, &mut budget, path)?;
        let read = LONG_CHUNK.saturating_sub(budget);
        *work = work.saturating_add(BUILD_LEVEL_WORK.saturating_add(LONG_BYTE_WORK.saturating_mul(read)));
    }
    if ran {
        save_long(h, tbl, ix, k, &st)?;
        ix.write_tree(h, tbl)?;
    }
    Ok(false)
}

/// Estimated instructions of one step of a build's insert made over units (`Secondary::build_step`) beyond
/// two comparisons through its key's bytes (a node's search, whose later comparisons skip the prefix its
/// bounds share, and the check for the key's count entry): a node searched and its child split ahead.
const BUILD_LEVEL_WORK: u64 = 8_200;

/// Estimated instructions of a build's insert step that reaches a leaf (`Secondary::build_step`) under a key
/// of n bytes, with 1.25 a key byte: the leaf searched, the row's entry and its key's records written, a
/// split; the key's blocks marked for hand-back.
const BUILD_LEAF_WORK: u64 = 16_400;

/// Inserts the build's row `rid` under its key `key`, written over units into state block `k`, a node at a
/// time (`Secondary::build_step`): from where the last unit stopped, each step checked against the unit's
/// room (its first run whatever the check finds when `lone`). The step that reaches a leaf counts the row's
/// entries, its key's records and a split; any other, a split and the count raised on its path. Returns
/// whether the row is in.
#[allow(clippy::too_many_arguments)]
fn insert_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    (k, rid): (u64, u64),
    key: &[u8],
    path: &mut Path,
) -> Result<bool, Fault> {
    let mut ix = Secondary::read(&*h, tbl, id)?;
    let word = ix.with_log(h, tbl, |m| m.key_word(k, KW_AT))?;
    let mut flags = word & (RAISED | SEEN);
    let mut at = word & !(RAISED | SEEN);
    let n = key.len() as u64;
    let past_w = n > ix.shape.w;
    let level = BUILD_LEVEL_WORK.saturating_add(n.saturating_mul(2));
    let mut lone = lone;
    loop {
        let (est, pages) = if ix.last_step(&*h, at)? {
            (BUILD_LEAF_WORK.saturating_add(n).saturating_add(n / 4), build_pages(clock, tbl, 1, past_w))
        } else {
            (level, split_pages(clock, tbl).saturating_add(2))
        };
        // Checked even when `lone`, after the key's read-back: run then whatever the check finds.
        if !clock.room(*work, est, Some(pages)) && !lone {
            break;
        }
        lone = false;
        at = ix.build_step(h, tbl, key, rid, at, &mut flags, path)?;
        *work = work.saturating_add(est);
        if at == 0 {
            ix.write_tree(h, tbl)?;
            return Ok(true);
        }
    }
    ix.with_log(h, tbl, |m| m.set_key_word(k, KW_AT, at | flags))?;
    ix.write_tree(h, tbl)?;
    Ok(false)
}

/// Blocks besides its state block that a writer's call hands back at once from a key written over units, once
/// its row is in or gone: those of a key of at most `KEY_STACK` bytes. More go back over the build's units
/// (`give_back_steps`), as every block does when the build itself takes or drops the row.
const DROP_AT_ONCE: u64 = 2;

/// Hands back the blocks of the build's key written over units from state block `k` that no entry took, the
/// state block last: in a writer's call (`now`) at once when they are few (`DROP_AT_ONCE`); else leaves them
/// to the build's next units, which check each block's hand-back against the unit's room. Returns `k` when it
/// leaves them, else 0.
fn leave_key<H: PageHost>(h: &mut H, tbl: u64, id: u8, ix: &mut Secondary, k: u64, now: bool) -> Result<u64, Fault> {
    let held = if now { ix.with_log(h, tbl, |m| m.key_held(k))? } else { u64::MAX };
    if held > DROP_AT_ONCE {
        Secondary::set_build_back(h, tbl, id, k)?;
        return Ok(k);
    }
    ix.with_log(h, tbl, |m| {
        let mut last = false;
        for _ in 0..=held {
            last = m.give_back(k)?;
        }
        if last {
            Ok(0)
        } else {
            Err(rt(Corruption::Count, k))
        }
    })
}

/// Leaves the build's key written over units from state block `k`, once its row is in or gone, its blocks
/// handed back (`leave_key`); a row gone after its insert raised its key's count lowers it back (`buf` and
/// `path` the caller's), at once: the build lowers one under a key longer than `KEY_STACK` over units first
/// (`lower_steps`), so only a writer's call does it here. In a writer's call (`now`) a few blocks go back at
/// once. Returns `k` when its blocks are left to the build's next units, else 0.
fn drop_long<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    k: u64,
    (gone, now): (bool, bool),
    (buf, path): (&mut [u8; KEY_STACK], &mut Path),
) -> Result<u64, Fault> {
    let mut ix = Secondary::read(&*h, tbl, id)?;
    if gone && ix.with_log(h, tbl, |m| m.key_word(k, KW_AT))? & RAISED != 0 {
        let len = ix.with_log(h, tbl, |m| m.key_word(k, KW_OUT))?;
        if len > KEY_STACK as u64 {
            // A writer's call: the descent to the key's count entry run to its end (`count_steps`).
            let mut head = [0u8; HEAD];
            let loc = ix.with_log(h, tbl, |m| m.key_open_record(k, len, &mut head[..]))?;
            let key = key_of_long(&ix, &head, loc, len)?;
            count_steps(h, tbl, &mut ix, None, &mut 0, true, (k, key), -1)?;
        } else {
            read_long(h, tbl, id, k, len, &mut buf[..])?;
            let key = buf.get(..usize::try_from(len).map_err(|_| rt(Corruption::Bounds, len))?).unwrap_or(&[]);
            ix.lower_count(h, tbl, key, path)?;
        }
    }
    let left = leave_key(h, tbl, id, &mut ix, k, now)?;
    ix.write_tree(h, tbl)?;
    Secondary::set_build_key(h, tbl, id, 0)?;
    Ok(left)
}

/// Estimated instructions of one block handed back (`AreaNodes::give_back`): the block put on its area's free
/// stack, its key-log radix entry cleared and the radix blocks over it read for an entry left (512 words
/// each), the state block's counts written.
const BACK_WORK: u64 = 8_000;

/// The pages one block handed back may change: the block, the radix blocks over it (freed when emptied), the
/// log's head once the log is empty, the state block, the descriptor, and the table root (the area's record).
fn back_pages(clock: &Clock<'_>, tbl: u64) -> u64 {
    clock.pages_for(6, 0, tbl)
}

/// Hands back the blocks of the build's key written over units from state block `k` that no entry took and
/// `leave_key` left, a block a step while the unit has room (its first step always when `lone`), the state
/// block last. The build takes no other row's key before they are all back, so a removed row's records (up to
/// 48 blocks) or a second record not taken never go back in one unit. Returns whether every block is back,
/// and whether a step ran.
fn give_back_steps<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    clock: &Clock<'_>,
    work: &mut u64,
    lone: bool,
    k: u64,
) -> Result<(bool, bool), Fault> {
    let mut ix = Secondary::read(&*h, tbl, id)?;
    let (mut lone, mut ran, mut done) = (lone, false, false);
    while !done {
        if !lone && !clock.room(*work, BACK_WORK, Some(back_pages(clock, tbl))) {
            break;
        }
        (lone, ran) = (false, true);
        done = ix.with_log(h, tbl, |m| m.give_back(k))?;
        *work = work.saturating_add(BACK_WORK);
    }
    if ran {
        ix.write_tree(h, tbl)?;
    }
    if done {
        Secondary::set_build_back(h, tbl, id, 0)?;
    }
    Ok((done, ran))
}

/// Before the stored row `rid` takes new cells under index `id`: when the build of `id` is writing or
/// inserting that row's key over units, hands back its partial records (a count its insert raised lowered
/// back), so the build takes the row again from its start, under its new cells.
fn restart_pending<H: PageHost>(
    h: &mut H,
    tbl: u64,
    id: u8,
    rid: u64,
    (buf, path): (&mut [u8; KEY_STACK], &mut Path),
) -> Result<(), Fault> {
    if Secondary::build_state(&*h, tbl, id)?.0 != rid {
        return Ok(());
    }
    match Secondary::build_key(&*h, tbl, id)? {
        0 => Ok(()),
        k => drop_long(h, tbl, id, k, (true, true), (buf, path)).map(|_| ()),
    }
}

/// Before a write changes the tree of index `id` while its build has a key written over units: when the
/// build's insert of that row holds a saved position part way down the row's path (`insert_steps`), completes
/// the insert in the writer's call (a count it is raising or lowering through a descent of its own too), hands
/// the key back and moves the build past the row, so writes can never hold a build back; when the row is gone,
/// hands the key back instead, a count its insert raised lowered back. Each row's insert is completed so at
/// most once, and only during a build. `path` is the caller's scratch.
#[cold]
#[inline(never)]
fn finish_pending<H: PageHost>(h: &mut H, tbl: u64, id: u8, path: &mut Path) -> Result<(), Fault> {
    let k = Secondary::build_key(&*h, tbl, id)?;
    let mut ix = Secondary::read(&*h, tbl, id)?;
    let (word, len) = ix.with_log(h, tbl, |m| Ok((m.key_word(k, KW_AT)?, m.key_word(k, KW_OUT)?)))?;
    let (mut at, mut flags) = (word & !AT_FLAGS, word & (RAISED | SEEN));
    if at == 0 {
        return Ok(());
    }
    let (next, end) = Secondary::build_state(&*h, tbl, id)?;
    if read::find(&*h, tbl, next)?.is_none() {
        // The row is gone: its key handed back, a count its insert raised lowered back, as the build would.
        let mut buf = [0u8; KEY_STACK];
        drop_long(h, tbl, id, k, (true, true), (&mut buf, path))?;
        return Secondary::set_build_state(h, tbl, id, next.wrapping_add(1), end);
    }
    if len > KEY_STACK as u64 {
        let mut head = [0u8; HEAD];
        let (loc, mut st) = long_state(h, tbl, &mut ix, (k, len), &mut head)?;
        let key = key_of_long(&ix, &head, loc, len)?;
        // A node a step, its search whole, down a tree of at most `PATH_STACK` levels.
        let mut steps = 0u64;
        while !ix.long_ready(&*h, &st)? {
            if steps > PATH_STACK {
                return Err(rt(Corruption::Link, next));
            }
            let mut all = u64::MAX;
            ix.long_step(h, tbl, key, next, &mut st, &mut all, path)?;
            steps = steps.wrapping_add(1);
        }
        if st.flags & (RAISED | SEEN) == SEEN {
            count_steps(h, tbl, &mut ix, None, &mut 0, true, (k, key), 1)?;
            st.flags |= RAISED;
        }
        take_long(h, tbl, id, &mut ix, (k, len, next), (&st, true), path)?;
        return Secondary::set_build_state(h, tbl, id, next.wrapping_add(1), end);
    }
    let mut buf = [0u8; KEY_STACK];
    read_long(h, tbl, id, k, len, &mut buf[..])?;
    let key = buf.get(..usize::try_from(len).map_err(|_| rt(Corruption::Bounds, len))?).unwrap_or(&[]);
    // One node a step, down a tree of at most `PATH_STACK` levels.
    let mut steps = 0u64;
    while at != 0 {
        if steps >= PATH_STACK {
            return Err(rt(Corruption::Link, next));
        }
        at = ix.build_step(h, tbl, key, next, at, &mut flags, path)?;
        steps = steps.wrapping_add(1);
    }
    ix.write_tree(h, tbl)?;
    drop_long(h, tbl, id, k, (false, true), (&mut buf, path))?;
    Secondary::set_build_state(h, tbl, id, next.wrapping_add(1), end)
}

/// Index `id`'s tree, read for a change made by a write: a build's insert holding a saved position there is
/// completed first (`finish_pending`).
#[inline(always)]
fn for_write<H: PageHost>(h: &mut H, tbl: u64, id: u8, path: &mut Path) -> Result<Secondary, Fault> {
    let ix = Secondary::read(&*h, tbl, id)?;
    if !ix.pending {
        return Ok(ix);
    }
    finish_pending(h, tbl, id, path)?;
    Secondary::read(&*h, tbl, id)
}

/// A build unit's scratch, each buffer made (zeroed) when first needed: a unit that only writes a step of a
/// key over units needs none of them, and zeroing them all would cost it about 10,000 instructions.
#[derive(Default)]
struct Scratch {
    buf: Option<[u8; KEY_STACK]>,
    piece: Option<[u8; PIECE]>,
    path: Option<Path>,
}

impl Scratch {
    fn path(&mut self) -> &mut Path {
        self.path.get_or_insert_with(Path::new)
    }

    /// The key buffer and the buffer for a value that does not lie in one piece.
    fn buf_piece(&mut self) -> (&mut [u8; KEY_STACK], &mut [u8; PIECE]) {
        let Scratch { buf, piece, .. } = self;
        (buf.get_or_insert_with(|| [0u8; KEY_STACK]), piece.get_or_insert_with(|| [0u8; PIECE]))
    }

    fn buf_path(&mut self) -> (&mut [u8; KEY_STACK], &mut Path) {
        let Scratch { buf, path, .. } = self;
        (buf.get_or_insert_with(|| [0u8; KEY_STACK]), path.get_or_insert_with(Path::new))
    }
}

/// One unit's share of the build of the lowest index being built: the rows from its position on, in rid
/// order, each live one taken under the key its current cells give, while the unit has room for one more
/// (the first row always when the build began the unit, `lead`); a group number with no group is passed
/// whole. A row's pages count a split at every level of the tree when that fits, else the full nodes on its
/// path. A row whose insert's descent finds a run of two or more full nodes on its path checks the unit's room
/// for their splits too, and waits for the next unit when they do not fit. When even a unit's first row does
/// not fit, the tree is tall and that run of full nodes long: its highest node is split ahead, one at a time,
/// while the unit has room, and the row waits for a unit where it fits. A row too long for a unit by its stored bytes, or whose key passes `KEY_STACK`, waits for the unit's
/// first place, and its key is written over units (`feed_long`); once it has ended, its key is read back from
/// the record in each unit and the row inserted a node at a time over units (`insert_steps`), or, longer than
/// `KEY_STACK`, given a second record over units (`twin_steps`) and inserted through both a node at a time,
/// its comparisons resumed inside the key (`long_steps`). Once such a row is in or gone, the blocks of its key
/// that no entry took are handed back a block at a time over units, each checked against the unit's room,
/// before the build goes on (`give_back_steps`); the descriptor is read again after them. When the build passes its end and they are back, the index holds every row, and counts
/// and walks of it are answered from then on.
pub fn build_unit<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    spent: &mut Spent,
    clock: &Clock<'_>,
    lead: bool,
) -> Result<(), Fault> {
    let b = building(&*h, tbl)?;
    let id = u8::try_from(b.trailing_zeros()).map_err(|_| rt(Corruption::Range, b))?;
    let d = find(shape, &*h, id)?;
    let (mut next, end) = Secondary::build_state(&*h, tbl, id)?;
    let mut work = spent.work.saturating_add(BUILD_LOAD);
    let mut sc = Scratch::default();
    // Each row's key parts as located (`located` writes the same parts for every row).
    let mut vals: Located = [None; PARTS_MAX];
    let mut ix = Secondary::read(&*h, tbl, id)?;
    let mut first = lead;
    // The row whose key is being written over units, when there is one: the row `next`.
    let mut long = Secondary::build_key(&*h, tbl, id)?;
    // A key whose blocks are being handed back, when there is one (no key is being written then).
    let mut back = Secondary::build_back(&*h, tbl, id)?;
    loop {
        if back != 0 {
            let (done, ran) = give_back_steps(h, tbl, id, clock, &mut work, first, back)?;
            first &= !ran;
            if !done {
                break;
            }
            back = 0;
            ix = Secondary::read(&*h, tbl, id)?;
        }
        if next >= end {
            break;
        }
        if !first && !clock.room(work, BUILD_ROW_WORK, Some(build_pages(clock, tbl, 0, false))) {
            break;
        }
        let g = next / GROUP_SLOTS;
        let gone = if root::group(&*h, tbl, g)? == 0 { None } else { read::find(&*h, tbl, next)? };
        let Some(r) = gone else {
            if long != 0 {
                // A count the row's insert raised under a long key is lowered back over units first.
                if !lower_steps(h, tbl, id, clock, &mut work, first, long)? {
                    break;
                }
                (back, long) = (drop_long(h, tbl, id, long, (true, false), sc.buf_path())?, 0);
            }
            next = if root::group(&*h, tbl, g)? == 0 {
                g.saturating_add(1).saturating_mul(GROUP_SLOTS).min(end)
            } else {
                next.wrapping_add(1)
            };
            work = work.saturating_add(BUILD_SLOT_WORK);
            first = false;
            continue;
        };
        // A row too long for a unit by its stored bytes, or whose key passes the stack buffer: its key
        // written over units, then the row taken under it, as any other row when the key fits the buffer.
        // Its values are located once: their stored bytes route it, and its key or its key's first step is
        // taken from them.
        let (mut n, mut fresh) = (None, None);
        if long == 0 {
            let (stored, most) = located(shape, &*h, &d, r, &mut vals)?;
            if clock.room(work, build_row_work(stored), Some(0)) {
                let (buf, piece) = sc.buf_piece();
                let key = key_of_located(&*h, &vals, piece, buf)?;
                n = key.map(|n| n as u64).filter(|&n| clock.room(work, build_row_work(n), Some(0)));
            }
            if n.is_none() && !first {
                break;
            }
            fresh = Some((&vals, most));
        }
        if n.is_none() {
            // `ix` is kept current through the row's key and its insert under a key longer than `KEY_STACK`.
            let (k, len, ran) = feed_long(h, tbl, shape, &d, clock, &mut work, first, (next, r, long), fresh, &mut ix)?;
            long = k;
            first &= !ran;
            match len {
                None => break,
                // Read back each unit, and inserted a node at a time over units.
                Some(len) if len <= KEY_STACK as u64 => {
                    let est = BUILD_READ_WORK.saturating_add(len);
                    if !first && !clock.room(work, est, Some(0)) {
                        break;
                    }
                    let (buf, path) = sc.buf_path();
                    read_long(h, tbl, id, long, len, &mut buf[..])?;
                    work = work.saturating_add(est);
                    let key = buf.get(..usize::try_from(len).map_err(|_| rt(Corruption::Bounds, len))?).unwrap_or(&[]);
                    if !insert_steps(h, tbl, id, clock, &mut work, first, (long, next), key, path)? {
                        break;
                    }
                    (back, long) = (drop_long(h, tbl, id, long, (false, false), (buf, path))?, 0);
                }
                // Longer than the buffer: its second record made over units, then taken in a unit of its own.
                Some(len) => {
                    let (made, ran) = twin_steps(h, tbl, &mut ix, clock, &mut work, first, (long, len))?;
                    first &= !ran;
                    let at = (long, next, len);
                    if !made || !long_steps(h, tbl, id, clock, &mut work, first, at, (&mut ix, sc.path()))? {
                        break;
                    }
                    (back, long) = (Secondary::build_back(&*h, tbl, id)?, 0);
                }
            }
            ix = Secondary::read(&*h, tbl, id)?;
        }
        if let Some(n) = n {
            let n = usize::try_from(n).map_err(|_| rt(Corruption::Bounds, n))?;
            let (buf, path) = sc.buf_path();
            let key = buf.get(..n).unwrap_or(&[]);
            let est = build_row_work(n as u64);
            let past_w = n as u64 > ix.shape.w;
            let fits = |splits: u64| {
                let p = build_pages(clock, tbl, splits, past_w);
                clock.pages().saturating_add(p) <= UNIT_BLOCKS_MAX
            };
            // The tree's height, read only for a row taken whole.
            let mut splits = ix.height(&*h)?;
            if !fits(splits) {
                splits = ix.splits(&*h, key, next, path)?;
                work = work.saturating_add(BUILD_RUN_WORK);
            }
            // The unit's first split always runs, as its first row would.
            let mut lone = true;
            while first && splits > 0 && !fits(splits) {
                if !lone && !clock.room(work, BUILD_SPLIT_WORK, Some(split_pages(clock, tbl))) {
                    break;
                }
                lone = false;
                // The run loses its highest node: those below it on the row's path are unchanged, and a
                // split leaf's halves have room for two entries.
                if !ix.split_ahead(h, tbl, key, next, path)? {
                    break;
                }
                ix.write_tree(h, tbl)?;
                splits = splits.saturating_sub(1);
                work = work.saturating_add(BUILD_SPLIT_WORK);
            }
            let pages = build_pages(clock, tbl, splits, past_w);
            if if first { !fits(splits) } else { !clock.room(work, est, Some(pages)) } {
                break;
            }
            // A run of two or more full nodes on the row's path, which its insert's descent finds, takes the
            // tree's general insert: its splits are checked against the unit's room once the run is known, and
            // the row waits for the next unit when they do not fit (a unit's first row always goes in).
            let fit = move |k: u64| first || clock.room(work, est.saturating_add(run_work(k)), None);
            let Some(k) = take_fitting(h, tbl, &mut ix, key, next, path, fit)? else { break };
            work = work.saturating_add(est).saturating_add(run_work(k));
        }
        next = next.wrapping_add(1);
        first = false;
    }
    if next >= end && back == 0 {
        Secondary::set_build_state(h, tbl, id, 0, 0)?;
        root::set_u64(h, tbl, BUILDING, b & !bit(id))?;
    } else {
        Secondary::set_build_state(h, tbl, id, next, end)?;
    }
    spent.work = work;
    Ok(())
}
