//! A change of the table's row type (`change`), O(columns + leaves), plus the seal of the open group.
//!
//! The new schema image keeps every leaf record of the table's at its ordinal (its id, kind, code, flags,
//! width and the version that added it), with these changes only: a non-key leaf dropped (its kind becomes
//! `node::DROPPED`, its nullable flag set, its column flagged dropped; a dropped leaf never comes back); new
//! leaves after the last ordinal, each nullable and added by the new version. Columns keep their ids. The
//! primary index's declarations stay as they are; a secondary index the new image no longer declares byte for
//! byte is removed (its descriptor cleared, its tree left allocated); the image declares no index the table
//! does not hold, and none with a part on a dropped leaf. The image's version is the table's + 1. Every refusal
//! comes before the first write.
//!
//! Rows written before the change keep the old row layout, so the open group is sealed first (`write::freeze`,
//! the seal a full group takes) whenever it has a position, dead ones included: the open group is then one
//! empty block, the hot state's only block. The integer leaves' totals follow: the hot state's running records
//! are laid out anew in that block by the rule of room the new schema gives (`Shape::open`), its first row byte
//! the new shape's `hot_h`, which every later block copies; a new leaf's record starts at zero, as every row
//! holds null for it; the records the hot state has no room for are written anew at the totals space's end
//! when they change.

use crate::consts::{HOT_MAX, PAGE};
use crate::fault::{rt, Corruption, Fault, SchemaError};
use crate::index::secondary::Secondary;
use crate::mem::{put, put_u16, put_u32, rd_u32, rd_u8, sub, PageHost, Region};
use crate::schema::{node, SchemaView, LEAF_KEY, LEAF_NULLABLE};
use crate::stats::running::LIST_AT;

use super::buffer;
use super::group::align8;
use super::hot::RunCursor;
use super::indexes::{DECL, PRIMARY};
use super::layout::{Shape, Ty};
use super::root;
use super::totals::{At, Totals};

/// Bytes of the buffer the new hot state is built in: `HOT_MAX`.
type Hot = [u8; 2_048];
const _: () = assert!(HOT_MAX == 2_048);

// The functions here that take no host are `#[inline]`, so that they are compiled only into `change`: compiled
// into the crate itself, they change how the crate's other functions are compiled, and every write's
// instruction count with it.

#[inline]
fn refuse(e: SchemaError) -> Fault {
    Fault::Schema(e)
}

/// Declaration i of a schema image: its bytes, and whether it is a secondary one each of whose parts is a live
/// leaf of the image.
#[inline]
fn view_decl<'a>(v: &SchemaView<'a>, i: u64) -> Result<(&'a [u8], bool), Fault> {
    // i < 64.
    let b = sub(v.bytes, u64::from(v.layout.indexes).wrapping_add(i.wrapping_mul(DECL)), DECL)?;
    if rd_u8(b, 1)? & PRIMARY != 0 {
        return Ok((b, false));
    }
    for k in 0..u64::from(rd_u8(b, 4)?) {
        let leaf = rd_u32(b, 8u64.wrapping_add(k.wrapping_mul(4)))?;
        for o in 0..v.leaves {
            let l = v.leaf(o)?;
            if l.id == leaf && l.kind == node::DROPPED {
                return Ok((b, false));
            }
        }
    }
    Ok((b, true))
}

/// Whether schema image `v` has a declaration of the same bytes as `d`, secondary and live when `live`.
#[inline]
fn view_has_decl(v: &SchemaView<'_>, d: &[u8], live: bool) -> Result<bool, Fault> {
    for i in 0..u64::from(v.indexes) {
        let (b, l) = view_decl(v, i)?;
        if b == d && l == live {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether two schema images declare the same indexes, byte for byte.
#[inline]
fn same_decls(a: &SchemaView<'_>, b: &SchemaView<'_>) -> Result<bool, Fault> {
    let n = u64::from(a.indexes).wrapping_mul(DECL);
    Ok(a.indexes == b.indexes
        && sub(a.bytes, u64::from(a.layout.indexes), n)? == sub(b.bytes, u64::from(b.layout.indexes), n)?)
}

/// Checks a new schema image against the table's, reading the images alone, before anything is written.
/// Returns the ids of the secondary indexes it removes (bit i for id i).
fn check<H: PageHost + ?Sized>(h: &H, shape: &Shape, image: &[u8]) -> Result<u64, Fault> {
    let old = SchemaView::open(h.get(Region::Page(shape.page))?)?;
    let new = SchemaView::open(image)?;
    if new.version != old.version.wrapping_add(1)
        || old.version == u32::MAX
        || new.leaves < old.leaves
        || new.columns < old.columns
    {
        return Err(refuse(SchemaError::ColumnChanged));
    }
    for o in 0..old.leaves {
        let a = old.leaf(o)?;
        let b = new.leaf(o)?;
        if b.dropped != 0 {
            return Err(refuse(SchemaError::NewDropped));
        }
        let same = a.id == b.id && a.code == b.code && a.width == b.width && a.added == b.added;
        if same && a.kind == b.kind && a.flags == b.flags {
            continue;
        }
        if a.kind == node::DROPPED {
            return Err(refuse(SchemaError::RestoredDropped));
        }
        if a.flags & LEAF_KEY != 0 {
            return Err(refuse(SchemaError::KeyChanged));
        }
        // The one change an old leaf may take: the drop mark.
        if !(same && b.kind == node::DROPPED && b.flags == a.flags | LEAF_NULLABLE) {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    for o in old.leaves..new.leaves {
        let b = new.leaf(o)?;
        if b.dropped != 0 || b.kind == node::DROPPED {
            return Err(refuse(SchemaError::NewDropped));
        }
        if b.added != new.version || b.flags & LEAF_KEY != 0 || b.flags & LEAF_NULLABLE == 0 {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    for i in 0..old.columns {
        let a = old.column(i)?;
        let b = new.column(i)?;
        if a.id != b.id || a.first_leaf != b.first_leaf || a.default_len != b.default_len {
            return Err(refuse(SchemaError::ColumnChanged));
        }
        let at_a = u64::from(old.layout.defaults).wrapping_add(u64::from(a.default_off));
        let at_b = u64::from(new.layout.defaults).wrapping_add(u64::from(b.default_off));
        if sub(old.bytes, at_a, u64::from(a.default_len))? != sub(new.bytes, at_b, u64::from(b.default_len))? {
            return Err(refuse(SchemaError::ColumnChanged));
        }
        if a.flags & 1 != 0 && b.flags & 1 == 0 {
            return Err(refuse(SchemaError::RestoredDropped));
        }
    }
    // The indexes: the primary declarations stay as they are; a secondary index the new image no longer
    // declares as it was is removed; the image declares no other.
    super::indexes::check(&new)?;
    let mut removed = 0u64;
    let mut primaries = 0u64;
    for i in 0..u64::from(old.indexes) {
        let (d, live) = view_decl(&old, i)?;
        if rd_u8(d, 1)? & PRIMARY != 0 {
            primaries = primaries.wrapping_add(1);
            if !view_has_decl(&new, d, false)? {
                return Err(refuse(SchemaError::KeyChanged));
            }
        } else if live && !view_has_decl(&new, d, true)? {
            removed |= 1u64.wrapping_shl(u32::from(rd_u8(d, 0)?));
        }
    }
    for i in 0..u64::from(new.indexes) {
        let (d, live) = view_decl(&new, i)?;
        if rd_u8(d, 1)? & PRIMARY != 0 {
            primaries = primaries.wrapping_sub(1);
        } else if !live || !view_has_decl(&old, d, true)? {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    if primaries != 0 {
        return Err(refuse(SchemaError::KeyChanged));
    }
    Ok(removed)
}

/// Adopts a new schema image: checked first (every refusal before the first write), then written to a new
/// page; the open group sealed when it has a position; the running records laid out anew in its one empty
/// block; the root's schema fields and the shape it holds set; the secondary indexes the image no longer
/// declares cleared; the old schema page freed. An image of the table's schema hash and index declarations
/// changes nothing. Returns the table's shape.
pub fn change<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, image: &[u8]) -> Result<Shape, Fault> {
    let v = SchemaView::open(image)?;
    if v.hash == shape.hash && same_decls(&v, &SchemaView::open(h.get(Region::Page(shape.page))?)?)? {
        return Ok(*shape);
    }
    if image.len() as u64 > PAGE {
        return Err(refuse(SchemaError::ColumnChanged));
    }
    for o in 0..v.leaves {
        if Ty::of(&v.leaf(o)?).is_none() {
            return Err(refuse(SchemaError::ColumnChanged));
        }
    }
    let removed = check(&*h, shape, image)?;
    // An open group without positions is one empty block: blocks are added only by inserts.
    let (_, desc, tail) = buffer::open_group(&*h, tbl)?;
    let pos = buffer::u16_of(&*h, tail, buffer::H_OPEN_POS)?;
    if pos == 0
        && (buffer::u16_of(&*h, desc, buffer::D_BLOCKS)? != 1 || buffer::u16_of(&*h, tail, buffer::B_SLOTS)? != 0)
    {
        return Err(rt(Corruption::Count, desc));
    }
    // The new image goes to a page of its own, which a refusal of its layout frees at once: it is the
    // table's only once the root names it.
    let page = crate::area::image_page_alloc(h, tbl)?;
    put(h.get_mut(Region::Page(page))?, 0, image)?;
    let new_shape = match Shape::open(&*h, page) {
        Ok(s) => s,
        Err(e) => {
            crate::area::image_page_free(h, tbl, page)?;
            return Err(e);
        }
    };
    if pos > 0 {
        super::write::freeze(h, tbl, shape)?;
    }
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let mut buf = [0u8; 2_048];
    let r = build_hot(&*h, tail, shape, &new_shape, &mut buf)?;
    relay_totals(h, tbl, tail, shape, &new_shape, &buf, r)?;
    lay_hot(h, tail, &new_shape, &buf, r)?;
    root::set_u64(h, tbl, root::SCHEMA_HASH, new_shape.hash)?;
    root::set_u32(h, tbl, root::SCHEMA_VERSION, new_shape.version)?;
    root::set_u64(h, tbl, root::SCHEMA_PAGE, page)?;
    new_shape.store(h, tbl)?;
    for id in 1..64u8 {
        if removed & 1u64.wrapping_shl(u32::from(id)) != 0 {
            Secondary::clear(h, tbl, id)?;
        }
    }
    crate::area::image_page_free(h, tbl, shape.page)?;
    Ok(new_shape)
}

/// The new hot state's running-record list and records, in `buf` from offset `LIST_AT`, by the rule of room
/// `Shape::open` gives the new schema: each integer leaf in ordinal order while its record and list entry fit
/// `HOT_MAX`. A record is copied from its old place, in the hot state of tail block `tail` or in the totals
/// space; a leaf the old version lacks gets a zero one. Returns the list's length R: the new shape's `run_r`,
/// its records ending at the new shape's `hot_h`.
fn build_hot<H: PageHost + ?Sized>(h: &H, tail: u64, old: &Shape, new: &Shape, buf: &mut Hot) -> Result<u64, Fault> {
    let mut oc = RunCursor::open(h, tail)?;
    let mut sp = Totals::read(h, tail)?;
    let (mut r, mut recs) = (0u64, 0u64);
    let mut at = LIST_AT.wrapping_add(new.run_list);
    for o in 0..new.leaves {
        let was = place_old(h, old, &mut oc, tail, o)?;
        let l = new.leaf(h, o)?;
        let Some(k) = l.ty.running() else { continue };
        let bytes = k.bytes();
        let list = align8(r.wrapping_add(1).wrapping_mul(2));
        if LIST_AT.wrapping_add(list).wrapping_add(recs).wrapping_add(bytes) > HOT_MAX {
            continue;
        }
        put_u16(buf, LIST_AT.wrapping_add(r.wrapping_mul(2)), l.ordinal)?;
        let rec = sub_mut(buf, at, bytes)?;
        match was {
            At::Hot(y) => put(rec, 0, sub(h.get(Region::Block(tail))?, y, bytes)?)?,
            At::Space(y) => {
                let s = sp.as_mut().ok_or(rt(Corruption::Link, tail))?;
                let off = s.recs.wrapping_add(y);
                s.read_bytes(h, off, rec)?;
            }
            At::None => {}
        }
        // Records of at most 32 bytes, fewer than 1,024 of them.
        r = r.wrapping_add(1);
        recs = recs.wrapping_add(bytes);
        at = at.wrapping_add(bytes);
    }
    if r != new.run_r || at != new.hot_h {
        return Err(rt(Corruption::Count, r));
    }
    Ok(r)
}

/// The totals space for the new schema version: the records of the integer leaves the new hot state (`buf`,
/// whose list has `r` entries) has no room for, in ordinal order. When they are the records the space holds,
/// at the same offsets, nothing is written; otherwise they are written anew at the space's end, each from its
/// old place (the hot state of tail block `tail`, still the old one, or the space) or zero for a new leaf, and
/// the space's records start there.
fn relay_totals<H: PageHost>(
    h: &mut H,
    tbl: u64,
    tail: u64,
    old: &Shape,
    new: &Shape,
    buf: &Hot,
    r: u64,
) -> Result<(), Fault> {
    let (mut oc, mut nc) = (RunCursor::open(&*h, tail)?, RunCursor::of_list(r));
    let (mut old_end, mut new_end, mut same) = (0u64, 0u64, true);
    for o in 0..new.leaves {
        let was = place_old(&*h, old, &mut oc, tail, o)?;
        if let At::Space(x) = was {
            old_end = x.wrapping_add(old.leaf(&*h, o)?.ty.running().map_or(0, |k| k.bytes()));
        }
        let ln = new.leaf(&*h, o)?;
        if let At::Space(x) = nc.place(buf, &ln)? {
            new_end = x.wrapping_add(ln.ty.running().map_or(0, |k| k.bytes()));
            same &= was == At::Space(x);
        }
    }
    if same && old_end == new_end {
        return Ok(());
    }
    let mut sp = Totals::ensure(h, tbl, tail)?;
    let old_recs = sp.recs;
    let base = sp.append(h, tbl, tail, new_end)?;
    sp.set_recs(h, base)?;
    let (mut oc, mut nc) = (RunCursor::open(&*h, tail)?, RunCursor::of_list(r));
    for o in 0..new.leaves {
        let was = place_old(&*h, old, &mut oc, tail, o)?;
        let ln = new.leaf(&*h, o)?;
        let At::Space(x) = nc.place(buf, &ln)? else { continue };
        let bytes = ln.ty.running().map_or(0, |k| k.bytes());
        let mut rb = [0u8; 32];
        let rec = sub_mut(&mut rb, 0, bytes)?;
        match was {
            At::Hot(y) => put(rec, 0, sub(h.get(Region::Block(tail))?, y, bytes)?)?,
            At::Space(y) => sp.read_bytes(&*h, old_recs.wrapping_add(y), rec)?,
            At::None => {}
        }
        sp.write_bytes(h, base.wrapping_add(x), rec)?;
    }
    Ok(())
}

/// Leaf ordinal `o`'s place in the old schema version's hot state (`oc`, in tail block `tail`); `None` for a
/// leaf the old version lacks.
fn place_old<H: PageHost + ?Sized>(h: &H, old: &Shape, oc: &mut RunCursor, tail: u64, o: u64) -> Result<At, Fault> {
    if o >= old.leaves {
        return Ok(At::None);
    }
    oc.place(h.get(Region::Block(tail))?, &old.leaf(h, o)?)
}

/// Writes the new hot state's list and records (`buf`, a list of `r` entries) into tail block `tail`, the open
/// group's one block, which holds no row: its first row byte and its rows' end become the new shape's `hot_h`.
fn lay_hot<H: PageHost>(h: &mut H, tail: u64, new: &Shape, buf: &Hot, r: u64) -> Result<(), Fault> {
    let hh = new.hot_h;
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let b = h.get_mut(Region::Block(tail))?;
    put(b, LIST_AT, sub(buf, LIST_AT, hh.wrapping_sub(LIST_AT))?)?;
    put_u16(b, buffer::H_H, w16(hh)?)?;
    put_u16(b, buffer::B_ROWS_END, w16(hh)?)?;
    put_u16(b, buffer::H_RUN_R, w16(r)?)?;
    put_u32(b, buffer::H_RUN_VERSION, new.version)
}

#[inline]
fn sub_mut(b: &mut [u8], at: u64, n: u64) -> Result<&mut [u8], Fault> {
    let s = usize::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
    let e = usize::try_from(at.wrapping_add(n)).map_err(|_| rt(Corruption::Range, at))?;
    b.get_mut(s..e).ok_or(rt(Corruption::Bounds, at))
}
