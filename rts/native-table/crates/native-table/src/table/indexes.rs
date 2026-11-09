//! The table's secondary indexes: every index declaration of the schema image that is not a primary one, each
//! kept in one `index::secondary` tree, whose descriptor the table makes when it is created.
//!
//! A declaration gives an id (1 to 63), the tree's W, and 1 to 7 parts, each a leaf of the row, in key order.
//! A row's key under an index is E of its parts' values in that order, each with its null tag when the leaf is
//! nullable: what `cell::enc_key` writes for the leaf, as for the primary key. Every write keeps every index
//! current within itself: an insert takes the new row's entry in each index, a remove drops the row's, and an
//! update moves the row's entry in each index whose key it changes. A key's count is one descent of its
//! index's tree. An index's rows are walked in key order, from an end or a key (`index::secondary::sorted`); its
//! ends give the smallest and the largest value.
//!
//! A key is built in a stack buffer of `KEY_STACK` bytes. A longer one (a long Text, Blob or `[u8; N]`, a huge
//! Nat or Int) takes no buffer: a write first finds its length, then writes it into a new record of the index's
//! key log, a piece at a time (a stored row's parts a piece at a time, `part_step`); its descents compare the
//! stored keys with that record (`LongKey`), each entry made under it takes a copy, and the record is handed
//! back. An update compares the new cells with it, piece by piece, to see whether the key changes. A count
//! encodes it into the caller's scratch. The walks (`walks`) take the caller's buffer for the keys they hold
//! and answer `Scratch` with the bytes it needs when a key is longer.

use crate::consts::{INDEXES_MAX, W_MAX};
use crate::fault::{rt, Corruption, Fault, ImageId, SchemaError};
use crate::index::area_nodes::AreaView;
use crate::index::secondary::{LongKey, Secondary, KIND_SECONDARY};
use crate::key::probe::{ovf_field, row_field, StoredCmp};
use crate::key::{self, KeyKind, KeyPart, KeySpec, LenSink, Sink, SliceSink};
use crate::mem::{copy_bytes, rd_u16, rd_u32, rd_u8, PageHost, Region};
use crate::schema::{SchemaView, LEAF_NULLABLE};
use crate::tree::{IndexShape, Path};

use super::cur::{self, SVal};
use super::group::GroupView;
use super::layout::{key_kind, LeafInfo, Shape, Ty};
use super::primary::Primary;
use super::{cell, overlay, read, row, RowSource, Table, KEY_STACK, PIECE};

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

/// Checks a schema image's index declarations before a table is made with it: at most 64;
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
            Primary::create(h, tbl, shape.key_w, shape.key_hash)?;
            primary = true;
        }
        let parts = d.parts(shape, &*h)?;
        Secondary::declare(h, tbl, d.id, d.w, d.spec(&parts).hash(KIND_SECONDARY))?;
    }
    Ok(())
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
                let v = overlay::buffered_value(shape, &rr, &mut walk, &l)?;
                next = o.wrapping_add(1);
                cell::enc_key(s, &l, &read::sval_val(h, &l, &v, piece)?)?;
            }
        }
        read::Live::Sealed(ref sg, pos) => {
            let gv = GroupView::from_meta(h, &sg.meta);
            let slot = gv.slot(pos)?;
            for &o in d.ords() {
                let l = shape.leaf(h, u64::from(o))?;
                cell::enc_key(s, &l, &read::sealed_cell(&gv, sg, &l, slot, pos, piece)?)?;
            }
        }
    }
    Ok(())
}

/// Leaf `l`'s current value in the stored row `r` (the overlay's). O(L) for a buffered row's offsets.
fn current<H: PageHost + ?Sized>(shape: &Shape, h: &H, r: read::Row, l: &LeafInfo) -> Result<SVal, Fault> {
    match r {
        read::Row::Buffered(at) => overlay::buffered_leaf(h, shape, &overlay::buffered(h, &at)?, l),
        read::Row::Sealed { page, pos } => overlay::sealed_value(h, &overlay::sealed(h, page)?, l, pos),
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
    let mut ix = Secondary::read(&*h, tbl, index)?;
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
/// pieces: a source's cells encoded whole, a stored row's parts a piece at a time (`part_step`).
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
                while let Some(f) = part_step(h, (&l, o, &val), &mut facts, from, &mut put)? {
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
    let mut ix = Secondary::read(&*h, tbl, d.id)?;
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
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
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
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        let mut piece = [0u8; PIECE];
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
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
    /// index whose key the new cells change, from the key its stored cells give to theirs.
    pub(super) fn index_update<R: RowSource + ?Sized>(
        &mut self,
        r: read::Row,
        src: &mut R,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        let mut piece = [0u8; PIECE];
        // The row opened at its first key read, for all the indexes: their entries' moves leave its group as is.
        let mut live = None;
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
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

/// The rows whose key under index `index` the cells of `key` give: one descent of the index's tree. A key longer
/// than `KEY_STACK` is encoded into `scratch`.
pub fn count<H: PageHost + ?Sized, K: RowSource + ?Sized, S: crate::host::Scratch>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    key: &mut K,
    scratch: &mut S,
) -> Result<i128, Fault> {
    let d = find(shape, h, index)?;
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

/// Bytes of a stored Text, Blob or `[u8; N]` value one piece of a stored row's key part takes: escaped, they
/// fit `ESC_BUF` with the part's null tag and terminator.
const FEED_CHUNK: usize = 2_046;
const ESC_BUF: usize = 2 * FEED_CHUNK + 3;

/// Bytes of a Nat's or Int's LEB form beyond i128 one piece of a stored row's key part reads.
const LEB_CHUNK: usize = 768;

/// One piece of key part `l`, ordinal `o`, of a stored row whose current value is `val`, handed to `put` with
/// the host, from `from`: the next `FEED_CHUNK` bytes of a Text, Blob or `[u8; N]` value (`from` of them
/// taken), a piece of a Nat or Int beyond i128 (`leb_step`, with its facts `facts`), or any other value
/// encoded whole. Returns where the part stands after the piece, or `None` once it has ended.
fn part_step<H: PageHost, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>>(
    h: &mut H,
    (l, o, val): (&LeafInfo, u16, &SVal),
    facts: &mut Option<key::LebFacts>,
    from: u64,
    put: &mut F,
) -> Result<Option<u64>, Fault> {
    if matches!(l.ty, Ty::Nat | Ty::Int) && matches!(val, SVal::B { .. } | SVal::Huge { .. }) {
        return leb_step(h, (l, o, val), facts, from, put);
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
    let taken = from.saturating_add(n as u64);
    let ends = taken >= total;
    // Bytes of a fixed width, or with no `00`, handed on as they are; others escaped into a buffer first.
    if fixed || key::zero_free(bytes) {
        let f = &mut Fwd { h: &mut *h, put: &mut *put };
        if tag {
            key::enc_opt_tag(f, true)?;
        }
        f.put(bytes)?;
        if !fixed && ends {
            key::enc_bytes_end(f)?;
        }
    } else {
        let mut esc = [0u8; ESC_BUF];
        let f = &mut SliceSink::new(&mut esc);
        if tag {
            key::enc_opt_tag(f, true)?;
        }
        key::enc_bytes_piece(f, bytes)?;
        if ends {
            key::enc_bytes_end(f)?;
        }
        put(h, f.written())?;
    }
    if ends {
        return Ok(None);
    }
    if n == 0 {
        return Err(rt(Corruption::Bounds, taken));
    }
    Ok(Some(taken))
}

/// Magnitude bytes one piece of a Nat or Int beyond i128 writes: their 7-bit groups lie in at most
/// `LEB_CHUNK` + 2 bytes of its LEB form.
const LEB_STEP: usize = LEB_CHUNK / 8 * 7;

/// Reads bytes [at, at + out.len()) of the stored value `v` of the leaf of ordinal `o` into `out`, all of them.
fn read_at<H: PageHost + ?Sized>(h: &H, v: &SVal, o: u16, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    match bytes_at(h, v, o, at, out)? {
        Some((n, _)) if n == out.len() => Ok(()),
        _ => Err(rt(Corruption::Bounds, at)),
    }
}

/// One piece of the Nat or Int `val` of leaf `l`, ordinal `o`, beyond i128, a key part written a piece at a
/// time: its canonical (signed) LEB form read `LEB_CHUNK` bytes at a time, from `from`. Until its facts are
/// known (`facts`), a piece reads the form's last bytes and, for a negative, scans its next bytes for its
/// lowest non-zero byte; the piece that finds them writes the part's null tag and head. Each piece then writes
/// the next `LEB_STEP` bytes of its magnitude, the most significant first (`from` of them written). Returns
/// where the part stands after the piece, or `None` once it has ended.
fn leb_step<H: PageHost, F: FnMut(&mut H, &[u8]) -> Result<(), Fault>>(
    h: &mut H,
    (l, o, val): (&LeafInfo, u16, &SVal),
    facts: &mut Option<key::LebFacts>,
    from: u64,
    put: &mut F,
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
