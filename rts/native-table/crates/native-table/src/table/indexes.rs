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
//! Nat or Int) is built in the caller's scratch, its length found first. The walks (`walks`) take the caller's
//! buffer for the keys they hold and answer `Scratch` with the bytes it needs when a key is longer.

use crate::consts::INDEXES_MAX;
use crate::fault::{rt, Corruption, Fault, ImageId, SchemaError};
use crate::host::Scratch;
use crate::index::secondary::{Secondary, KIND_SECONDARY};
use crate::key::{KeyKind, KeyPart, KeySpec, LenSink, Sink, SliceSink};
use crate::mem::{rd_u16, rd_u32, rd_u8, PageHost, Region};
use crate::schema::{SchemaView, LEAF_NULLABLE};
use crate::tree::{IndexShape, Path};

use super::cell::Val;
use super::group::GroupView;
use super::layout::{key_kind, LeafInfo, Shape};
use super::primary::Primary;
use super::{cell, overlay, read, row, RowSource, Table, KEY_STACK, PIECE};

/// Declarations a schema image may hold.
const DECLS_MAX: u64 = 64;
/// Bytes of a declaration in the schema image.
pub(super) const DECL: u64 = crate::schema::DECL_BYTES;
/// Declaration flags bit 0: a primary index.
pub(super) const PRIMARY: u8 = 1;
/// Declaration flags bit 3: a unique index, which holds a key with no null part for one row at most.
pub const UNIQUE: u8 = 8;
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

/// The cells a write gives some leaves of a stored row, read in place of the row's own: none for `()`.
pub(super) trait Over {
    fn val(&mut self, l: &LeafInfo) -> Option<Result<Val<'_>, Fault>>;
}

impl Over for () {
    #[inline(always)]
    fn val(&mut self, _: &LeafInfo) -> Option<Result<Val<'_>, Fault>> {
        None
    }
}

/// A row source taken as `dyn RowSource`, so that the unique checks are made once for every source type and
/// leave the code of the writes as it is.
pub(super) struct Dyn<'a, R: ?Sized>(pub &'a mut R);

impl<R: RowSource + ?Sized> RowSource for Dyn<'_, R> {
    fn cell(&mut self, leaf: u32) -> Result<super::CellIn<'_>, Fault> {
        self.0.cell(leaf)
    }
    fn fill(&mut self, leaf: u32, at: u64, dst: &mut [u8]) -> Result<(), Fault> {
        self.0.fill(leaf, at, dst)
    }
}

/// The leaves of the list, from the cells of the source.
impl<R: RowSource + ?Sized> Over for (&[u32], &mut R) {
    fn val(&mut self, l: &LeafInfo) -> Option<Result<Val<'_>, Fault>> {
        self.0.contains(&l.id).then(|| row::cell_of(&mut *self.1, l))
    }
}

/// Writes the key the stored row `r` has under index `d` into `s`, from its current cells (the overlay's
/// values), each leaf `over` gives from that; `piece` takes a value that does not lie in one piece of table
/// memory.
fn enc_live<H: PageHost + ?Sized, O: Over + ?Sized, S: Sink>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: &read::Live,
    over: &mut O,
    piece: &mut [u8],
    s: &mut S,
) -> Result<(), Fault> {
    match *r {
        read::Live::Buffered(at) => {
            let b = overlay::buffered(h, &at)?;
            let rr = row::RowRef::new(h, b.loc)?;
            // One walk while the parts' ordinals rise; a part before the walk's place starts it again.
            let mut walk = row::Walk::of(shape, &rr)?;
            for &o in d.ords() {
                let o = u64::from(o);
                if o < walk.bit {
                    walk = row::Walk::of(shape, &rr)?;
                }
                walk.skip_to(shape, h, o)?;
                let l = shape.leaf(h, o)?;
                if let Some(v) = over.val(&l) {
                    cell::enc_key(s, &l, &v?)?;
                    continue;
                }
                let v = overlay::buffered_value(shape, &rr, &mut walk, &l)?;
                cell::enc_key(s, &l, &read::sval_val(h, &l, &v, piece)?)?;
            }
        }
        read::Live::Sealed(ref sg, pos) => {
            let gv = GroupView::from_meta(h, &sg.meta);
            let slot = gv.slot(pos)?;
            for &o in d.ords() {
                let l = shape.leaf(h, u64::from(o))?;
                if let Some(v) = over.val(&l) {
                    cell::enc_key(s, &l, &v?)?;
                    continue;
                }
                cell::enc_key(s, &l, &read::sealed_cell(&gv, sg, &l, slot, pos, piece)?)?;
            }
        }
    }
    Ok(())
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
fn key_of_stored<H: PageHost + ?Sized, O: Over + ?Sized>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: &read::Live,
    over: &mut O,
    piece: &mut [u8; PIECE],
    buf: &mut [u8; KEY_STACK],
) -> Result<Option<usize>, Fault> {
    let mut s = SliceSink::new(buf);
    match enc_live(shape, h, d, r, over, piece, &mut s) {
        Ok(()) => Ok(Some(s.len)),
        Err(Fault::Scratch { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Takes the row `rid` in index `d` under the key the cells of `src` give.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn take_source<H: PageHost, R: RowSource + ?Sized, S: Scratch>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    src: &mut R,
    rid: u64,
    buf: &mut [u8; KEY_STACK],
    scratch: &mut S,
    path: &mut Path,
) -> Result<(), Fault> {
    match key_of_source(shape, &*h, d, src, buf) {
        Ok(n) => change(h, tbl, d.id, buf.get(..n).unwrap_or(&[]), rid, true, path),
        Err(Fault::Scratch { .. }) => {
            let key = source_key(shape, &*h, d, src, scratch)?;
            change(h, tbl, d.id, key, rid, true, path)
        }
        Err(e) => Err(e),
    }
}

/// The key the cells of `src` give index `d`, longer than `KEY_STACK`, in `scratch`: its length found first.
#[inline(never)]
fn source_key<'s, H: PageHost + ?Sized, R: RowSource + ?Sized, S: Scratch>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    src: &mut R,
    scratch: &'s mut S,
) -> Result<&'s mut [u8], Fault> {
    let mut c = LenSink(0);
    enc_source(shape, h, d, src, &mut c)?;
    let need = usize::try_from(c.0).map_err(|_| Fault::Scratch { need: c.0 })?;
    let buf = scratch.get(need)?;
    enc_source(shape, h, d, src, &mut SliceSink::new(buf))?;
    Ok(buf)
}

/// The key the stored row `r` has under index `d`, longer than `KEY_STACK`, in `scratch`: its length found
/// first, with each value that does not lie in one piece of table memory copied into a piece of scratch as
/// long as the longest of them.
#[inline(never)]
fn stored_key<'s, H: PageHost + ?Sized, O: Over + ?Sized, S: Scratch>(
    shape: &Shape,
    h: &H,
    d: &Decl,
    r: &read::Live,
    over: &mut O,
    scratch: &'s mut S,
) -> Result<&'s [u8], Fault> {
    let mut piece = PIECE;
    let len = loop {
        let mut c = LenSink(0);
        match enc_live(shape, h, d, r, over, scratch.get(piece)?, &mut c) {
            Ok(()) => break c.0,
            Err(Fault::Scratch { need }) => {
                piece = usize::try_from(need).map_err(|_| Fault::Scratch { need })?.max(piece.saturating_mul(2));
            }
            Err(e) => return Err(e),
        }
    };
    let n = usize::try_from(len).map_err(|_| Fault::Scratch { need: len })?;
    let (p, key) = scratch.get(piece.saturating_add(n))?.split_at_mut(piece);
    enc_live(shape, h, d, r, over, p, &mut SliceSink::new(key))?;
    Ok(key)
}

/// E(k) of the stored row `r`, its key leaves encoded as an index's parts are: at the start of `buf`, or in
/// `scratch` when longer than `KEY_STACK`.
pub(super) fn primary_key<'b, H: PageHost + ?Sized, S: Scratch>(
    shape: &Shape,
    h: &H,
    r: &read::Live,
    buf: &'b mut [u8; KEY_STACK],
    scratch: &'b mut S,
) -> Result<&'b [u8], Fault> {
    let mut d = Decl { id: 0, w: shape.key_w, parts: 0, ords: [0; PARTS_MAX] };
    for o in 0..shape.leaves {
        if u64::from(d.parts) == shape.key_leaves {
            break;
        }
        if shape.leaf(h, o)?.key {
            let ord = d.ords.get_mut(usize::from(d.parts)).ok_or(rt(Corruption::Count, o))?;
            *ord = u16::try_from(o).map_err(|_| rt(Corruption::Leaf, o))?;
            d.parts = d.parts.wrapping_add(1);
        }
    }
    let mut piece = [0u8; PIECE];
    match key_of_stored(shape, h, &d, r, &mut (), &mut piece, buf)? {
        Some(n) => Ok(buf.get(..n).unwrap_or(&[])),
        None => stored_key(shape, h, &d, r, &mut (), scratch),
    }
}

impl<H: PageHost> Table<'_, H> {
    /// The unique indexes among the declarations `mask` sets (bit i: declaration i) whose key, with no null part,
    /// a row other than `own` holds: the key the cells of `src` give, or with `set` = (r, leaves), the key the
    /// stored row r has once its leaves `leaves` take the cells of `src`. A key's encoding and a descent of its
    /// index's tree each, and one more descent when one row holds it and `own` is given.
    #[inline(never)]
    pub(super) fn held<S: Scratch>(
        &self,
        src: &mut dyn RowSource,
        set: Option<(&read::Live, &[u32])>,
        own: Option<u64>,
        mask: u64,
        scratch: &mut S,
    ) -> Result<u64, Fault> {
        let (h, shape) = (&*self.host, &self.shape);
        let (mut buf, mut piece) = ([0u8; KEY_STACK], [0u8; PIECE]);
        let mut held = 0u64;
        for i in 0..shape.index_decls {
            if mask >> i & 1 == 0 {
                continue;
            }
            let Some(d) = decl(shape, h, i)? else { continue };
            let key = match set {
                None => match key_of_source(shape, h, &d, src, &mut buf) {
                    Ok(n) => buf.get(..n).unwrap_or(&[]),
                    Err(Fault::Scratch { .. }) => source_key(shape, h, &d, src, scratch)?,
                    Err(e) => return Err(e),
                },
                Some((r, leaves)) => {
                    match key_of_stored(shape, h, &d, r, &mut (leaves, &mut *src), &mut piece, &mut buf)? {
                        Some(n) => buf.get(..n).unwrap_or(&[]),
                        None => stored_key(shape, h, &d, r, &mut (leaves, &mut *src), scratch)?,
                    }
                }
            };
            if d.spec(&d.parts(shape, h)?).has_null(key, ImageId::Runtime)? {
                continue;
            }
            let ix = Secondary::read(h, self.root, d.id)?;
            let n = ix.count(h, key)?;
            let own = match own {
                Some(rid) if n == 1 => ix.holds(h, key, rid)?,
                _ => false,
            };
            if n > i128::from(own) {
                held |= 1u64.wrapping_shl(u32::try_from(i).unwrap_or(0));
            }
        }
        Ok(held)
    }

    /// Refuses (`Unique`) an upsert of the row `src` whose key under a unique index, with no null part, a row
    /// other than the one of the row's key holds: the row's key is looked up only when a row holds one.
    #[inline(never)]
    pub(super) fn unique_row<S: Scratch>(&self, src: &mut dyn RowSource, scratch: &mut S) -> Result<(), Fault> {
        let held = self.held(src, None, None, self.shape.unique, scratch)?;
        if held == 0 {
            return Ok(());
        }
        let mut kb = [0u8; KEY_STACK];
        let e = self.key_in(src, &mut kb, scratch)?;
        let own = match Primary::read(&*self.host, self.root)? {
            Some(p) => super::primary::lookup(&*self.host, &p, e, &mut Path::new())?.found.map(|(rid, _, _)| rid),
            None => None,
        };
        if own.is_none() || self.held(src, None, own, held, scratch)? != 0 {
            return Err(Fault::Unique);
        }
        Ok(())
    }

    /// Takes the new row `rid` in every secondary index, under the key the cells of `src` give each; `buf` and
    /// `path` are the caller's.
    pub(super) fn index_insert<R: RowSource + ?Sized, S: Scratch>(
        &mut self,
        src: &mut R,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        scratch: &mut S,
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        for i in 0..shape.index_decls {
            let Some(d) = decl(shape, &*h, i)? else { continue };
            take_source(h, tbl, shape, &d, src, rid, buf, scratch, path)?;
        }
        Ok(())
    }

    /// Drops the stored row `rid`, lying at `r`, from each secondary index whose declaration's bit `mask` sets
    /// (`take`: takes it in each), under the key its stored cells give each. Inlined into each caller, so a remove
    /// keeps its mask and direction known in its own code.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn index_stored<S: Scratch>(
        &mut self,
        r: &read::Live,
        mask: u64,
        take: bool,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        scratch: &mut S,
        path: &mut Path,
    ) -> Result<(), Fault> {
        let (h, tbl, shape) = (&mut *self.host, self.root, &self.shape);
        let mut piece = [0u8; PIECE];
        for i in 0..shape.index_decls {
            if mask >> i & 1 == 0 {
                continue;
            }
            let Some(d) = decl(shape, &*h, i)? else { continue };
            let key = match key_of_stored(shape, &*h, &d, r, &mut (), &mut piece, buf)? {
                Some(n) => buf.get(..n).unwrap_or(&[]),
                None => stored_key(shape, &*h, &d, r, &mut (), scratch)?,
            };
            change(h, tbl, d.id, key, rid, take, path)?;
        }
        Ok(())
    }

    /// Before the stored row `rid`, lying at `r`, takes the cells of `src`: moves its entry in every secondary
    /// index whose key the new cells change, from the key its stored cells give to theirs.
    pub(super) fn index_update<R: RowSource + ?Sized, S: Scratch>(
        &mut self,
        r: read::Row,
        src: &mut R,
        rid: u64,
        buf: &mut [u8; KEY_STACK],
        scratch: &mut S,
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
            let old = match key_of_stored(shape, &*h, &d, lv, &mut (), &mut piece, buf)? {
                Some(n) => buf.get(..n).unwrap_or(&[]),
                None => stored_key(shape, &*h, &d, lv, &mut (), scratch)?,
            };
            let same = {
                let mut cmp = super::KeyCompare { want: old, at: 0, same: true };
                enc_source(shape, &*h, &d, src, &mut cmp)?;
                cmp.same && cmp.at == old.len()
            };
            if same {
                continue;
            }
            change(h, tbl, d.id, old, rid, false, path)?;
            take_source(h, tbl, shape, &d, src, rid, buf, scratch, path)?;
        }
        Ok(())
    }
}

/// The rows whose key under index `index` the cells of `key` give: one descent of the index's tree. A key longer
/// than `KEY_STACK` is encoded into `scratch`.
pub fn count<H: PageHost + ?Sized, K: RowSource + ?Sized, S: Scratch>(
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

/// `count` for a key longer than `KEY_STACK`, which `scratch` takes.
#[inline(never)]
fn count_long<H: PageHost + ?Sized, K: RowSource + ?Sized, S: Scratch>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    key: &mut K,
    scratch: &mut S,
) -> Result<i128, Fault> {
    let key = source_key(shape, h, d, key, scratch)?;
    Secondary::read(h, tbl, d.id)?.count(h, key)
}

/// The rows whose first `parts` parts under index `index` have the cells of `key` (`Secondary::count_prefix`). A
/// key longer than `KEY_STACK` is encoded into `scratch`.
#[inline(never)]
pub fn count_prefix<H: PageHost + ?Sized, K: RowSource + ?Sized, S: Scratch>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    index: u8,
    parts: u8,
    key: &mut K,
    scratch: &mut S,
) -> Result<i128, Fault> {
    let mut d = find(shape, h, index)?;
    d.parts = d.parts.min(parts);
    let ix = Secondary::read(h, tbl, index)?;
    let mut buf = [0u8; KEY_STACK];
    match key_of_source(shape, h, &d, key, &mut buf) {
        Ok(n) => ix.count_prefix(h, buf.get_mut(..n).unwrap_or(&mut [])),
        Err(Fault::Scratch { .. }) => ix.count_prefix(h, source_key(shape, h, &d, key, scratch)?),
        Err(e) => Err(e),
    }
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
