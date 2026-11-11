//! The overlay: a row's current values. A sealed row's leaf reads its patch column first, then its image cell.
//! A buffered row's leaf reads its row image.

use crate::fault::{rt, Corruption, Fault};
use crate::mem::{PageHost, Region};

use super::buffer::{self, Located, RowLoc};
use super::cur::{self, SVal};
use super::group::{GroupMeta, GroupView, LeafDir};
use super::layout::{LeafInfo, Shape};
use super::page;
use super::patch;
use super::row::{RowRef, Walk};

/// A sealed group as its rows' readers and writers see it.
#[derive(Clone, Copy, Debug)]
pub struct Sealed {
    pub page: u64,
    pub meta: GroupMeta,
    /// The patch directory (0: none).
    pub dir: u64,
}

/// The group whose page is `pg`.
pub fn sealed<H: PageHost + ?Sized>(h: &H, pg: u64) -> Result<Sealed, Fault> {
    let gv = GroupView::open(h, pg)?;
    let dir = crate::mem::rd_u64(h.get(Region::Block(pg))?, page::P_PATCH_DIR)?;
    Ok(Sealed { page: pg, meta: gv.meta(), dir })
}

/// Whether every row of the sealed group whose page is `pg` holds null for leaf ordinal `o`, which schema version
/// `added` added, known without reading a row: the group was packed under an earlier version, so the leaf lies
/// past its leaf directory, and no live patch entry names the leaf.
pub fn all_null<H: PageHost + ?Sized>(h: &H, pg: u64, o: u16, added: u32) -> Result<bool, Fault> {
    let b = h.get(Region::Block(pg))?;
    let dir = crate::mem::rd_u64(b, page::P_PATCH_DIR)?;
    Ok(crate::mem::rd_u32(b, page::P_VERSION)? < added
        && (dir == 0 || patch::unit(h, dir, u64::from(o))?.is_none_or(|u| u.live == 0)))
}

pub fn sealed_live<H: PageHost + ?Sized>(h: &H, s: &Sealed, pos: u64) -> Result<bool, Fault> {
    if pos >= s.meta.p {
        return Ok(false);
    }
    page::live(h, s.page, pos)
}

/// Leaf `l`'s current value at position `pos` of a sealed group.
pub fn sealed_value<H: PageHost + ?Sized>(h: &H, s: &Sealed, l: &LeafInfo, pos: u64) -> Result<SVal, Fault> {
    if let Some(e) = patch_entry(h, s, l, pos)? {
        return cur::from_entry(h, &e, l);
    }
    image_value(h, &s.meta, l, pos)
}

/// The patch entry holding leaf `l` at position `pos` of a sealed group, if any.
fn patch_entry<H: PageHost + ?Sized>(h: &H, s: &Sealed, l: &LeafInfo, pos: u64) -> Result<Option<patch::Entry>, Fault> {
    if s.dir != 0 {
        return patch::lookup(h, s.dir, u64::from(l.ordinal), pos);
    }
    Ok(None)
}

/// Leaf `l`'s image cell at position `pos`.
pub fn image_value<H: PageHost + ?Sized>(h: &H, m: &GroupMeta, l: &LeafInfo, pos: u64) -> Result<SVal, Fault> {
    let gv = GroupView::from_meta(h, m);
    cell_value(&gv, &gv.leaf(u64::from(l.ordinal))?, l, gv.slot(pos)?, pos)
}

/// Leaf `l`'s current value at position `pos` of sealed group `s` (viewed by `gv`), its leaf directory entry
/// `ld` and the position's stored slot `slot` found by the caller.
pub fn leaf_value<H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    s: &Sealed,
    l: &LeafInfo,
    ld: Option<&LeafDir>,
    slot: Option<u64>,
    pos: u64,
) -> Result<SVal, Fault> {
    if let Some(e) = patch_entry(gv.h, s, l, pos)? {
        return cur::from_entry(gv.h, &e, l);
    }
    match ld {
        Some(ld) => cell_value(gv, ld, l, slot, pos),
        None => Err(rt(Corruption::Leaf, pos)),
    }
}

/// Leaf `l`'s image cell (directory entry `ld`) at position `pos`, whose stored slot is `slot`.
fn cell_value<H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    ld: &LeafDir,
    l: &LeafInfo,
    slot: Option<u64>,
    pos: u64,
) -> Result<SVal, Fault> {
    cur::from_cell(gv, ld, l, slot.ok_or(rt(Corruption::Link, pos))?, pos)
}

/// A buffered row as its readers and writers see it.
#[derive(Clone, Copy, Debug)]
pub struct Buffered {
    pub loc: RowLoc,
}

/// The buffered row `at`, which its slot names live.
pub fn buffered<H: PageHost + ?Sized>(h: &H, at: &Located) -> Result<Buffered, Fault> {
    let loc = buffer::slot_row(h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    Ok(Buffered { loc })
}

/// Whether a buffered row is live: its slot.
pub fn buffered_live<H: PageHost + ?Sized>(h: &H, at: &Located) -> Result<bool, Fault> {
    let slot = buffer::read_slot(h.get(Region::Block(at.block))?, at.slot)?;
    Ok(slot.flags & buffer::SLOT_DEAD == 0)
}

/// Leaf `l`'s current value in a buffered row, `walk` positioned at the leaf (and moved past it).
pub fn buffered_value<H: PageHost + ?Sized>(
    shape: &Shape,
    row: &RowRef<'_, H>,
    walk: &mut Walk,
    l: &LeafInfo,
) -> Result<SVal, Fault> {
    let vi = walk.var;
    let s = walk.next(shape, row, l)?;
    cur::from_stored(row, shape, l, &s, vi)
}

/// Leaf `l`'s current value in a buffered row, found from its ordinal (O(L) for the leaf's offsets).
pub fn buffered_leaf<H: PageHost + ?Sized>(h: &H, shape: &Shape, b: &Buffered, l: &LeafInfo) -> Result<SVal, Fault> {
    let row = RowRef::new(h, b.loc)?;
    let mut walk = Walk::of(shape, &row)?;
    for o in 0..u64::from(l.ordinal) {
        walk.skip(&shape.leaf(h, o)?);
    }
    buffered_value(shape, &row, &mut walk, l)
}
