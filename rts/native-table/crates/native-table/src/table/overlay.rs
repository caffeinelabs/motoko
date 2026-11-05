//! The overlay: a row's current values. A sealed row's leaf reads its post-snapshot patch first (while a
//! rewrite holds a snapshot of its group), then its patch column, then its image cell, then the leaf's
//! default when the image lacks the leaf. A buffered row's leaf reads its post-snapshot patch first (while
//! a seal holds a snapshot of its group), then its row image, then its default when the row's schema version
//! lacks the leaf. Liveness comes from the pending group page while a rewrite holds a snapshot.

use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{rd_u16, rd_u8, PageHost, Region};

use super::buffer::{self, Located, RowLoc};
use super::cur::{self, SVal};
use super::group::{GroupMeta, GroupView, LeafDir};
use super::layout::{LeafInfo, Shape};
use super::page;
use super::patch;
use super::row::{RowRef, Walk};
use super::seal::Rw;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// A sealed group as its rows' readers and writers see it.
#[derive(Clone, Copy, Debug)]
pub struct Sealed {
    pub page: u64,
    pub meta: GroupMeta,
    /// The patch directory (0: none).
    pub dir: u64,
    /// The post-snapshot patch directory while a rewrite holds a snapshot (0: none).
    pub post: u64,
    /// The page whose live bitmap holds the group's liveness.
    pub live_page: u64,
    /// The rewrite holding a snapshot of the group, before its flip.
    pub rw: Option<Rw>,
}

/// The group whose page is `pg`.
pub fn sealed<H: PageHost + ?Sized>(h: &H, pg: u64) -> Result<Sealed, Fault> {
    let gv = GroupView::open(h, pg)?;
    let b = h.get(Region::Block(pg))?;
    let dir = crate::mem::rd_u64(b, page::P_PATCH_DIR)?;
    let st = crate::mem::rd_u64(b, page::P_REWRITE)?;
    let mut s = Sealed { page: pg, meta: gv.meta(), dir, post: 0, live_page: pg, rw: None };
    if st != 0 {
        let rw = Rw::read(h, st)?;
        if rw.holds() && rw.pending != pg {
            s.post = crate::mem::rd_u64(h.get(Region::Block(rw.pending))?, page::P_POST_PATCH)?;
            s.live_page = rw.pending;
            s.rw = Some(rw);
        }
    }
    Ok(s)
}

/// Whether position `pos` of the group is live.
pub fn sealed_live<H: PageHost + ?Sized>(h: &H, s: &Sealed, pos: u64) -> Result<bool, Fault> {
    if pos >= s.meta.p {
        return Ok(false);
    }
    page::live(h, s.live_page, pos)
}

/// Leaf `l`'s current value at position `pos` of a sealed group.
pub fn sealed_value<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    s: &Sealed,
    l: &LeafInfo,
    pos: u64,
) -> Result<SVal, Fault> {
    if let Some(e) = patch_entry(h, s, l, pos)? {
        return cur::from_entry(h, &e, l);
    }
    image_value(h, shape, &s.meta, l, pos)
}

/// The patch entry holding leaf `l` at position `pos` of a sealed group (the post-snapshot patch first), if
/// any.
fn patch_entry<H: PageHost + ?Sized>(h: &H, s: &Sealed, l: &LeafInfo, pos: u64) -> Result<Option<patch::Entry>, Fault> {
    let o = u64::from(l.ordinal);
    if s.post != 0 {
        if let Some(e) = patch::lookup(h, s.post, o, pos)? {
            return Ok(Some(e));
        }
    }
    if s.dir != 0 {
        return patch::lookup(h, s.dir, o, pos);
    }
    Ok(None)
}

/// Leaf `l`'s image cell at position `pos` (its default when the image lacks the leaf).
pub fn image_value<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    m: &GroupMeta,
    l: &LeafInfo,
    pos: u64,
) -> Result<SVal, Fault> {
    let gv = GroupView::from_meta(h, m);
    let Some(ld) = gv.leaf_by_ordinal(u64::from(l.ordinal))? else { return super::defaults::of(h, shape, l) };
    cell_value(&gv, &ld, l, gv.slot(pos)?, pos)
}

/// Leaf `l`'s current value at position `pos` of sealed group `s` (viewed by `gv`), its leaf directory entry
/// `ld` and the position's stored slot `slot` found by the caller.
pub fn leaf_value<H: PageHost + ?Sized>(
    gv: &GroupView<'_, H>,
    shape: &Shape,
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
        None => super::defaults::of(gv.h, shape, l),
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
    pub at: Located,
    pub loc: RowLoc,
    /// The post-snapshot patch directory while a seal holds a snapshot of the group (0: none).
    pub post: u64,
    /// The seal holding a snapshot of the group, before its flip.
    pub rw: Option<Rw>,
}

/// The seal holding a snapshot of buffered group `desc`, if any.
pub fn seal_of<H: PageHost + ?Sized>(h: &H, desc: u64) -> Result<Option<Rw>, Fault> {
    let d = h.get(Region::Block(desc))?;
    if rd_u16(d, buffer::D_FLAGS)? & buffer::SNAPSHOT == 0 {
        return Ok(None);
    }
    let pending = crate::mem::rd_u64(d, buffer::D_PENDING)?;
    let st = crate::mem::rd_u64(h.get(Region::Block(pending))?, page::P_REWRITE)?;
    if st == 0 {
        return Ok(None);
    }
    let rw = Rw::read(h, st)?;
    Ok(if rw.before_flip() { Some(rw) } else { None })
}

/// The buffered row `at`, which its slot names live.
pub fn buffered<H: PageHost + ?Sized>(h: &H, at: &Located) -> Result<Buffered, Fault> {
    let loc = buffer::slot_row(h, at.block, at.slot)?.ok_or(rt(Corruption::Link, at.block))?;
    let rw = if at.frozen { seal_of(h, at.desc)? } else { None };
    let post = match rw {
        Some(r) => crate::mem::rd_u64(h.get(Region::Block(r.pending))?, page::P_POST_PATCH)?,
        None => 0,
    };
    Ok(Buffered { at: *at, loc, post, rw })
}

/// Whether a buffered row is live: its slot, or the pending page's bit while a seal holds the group.
pub fn buffered_live<H: PageHost + ?Sized>(h: &H, at: &Located) -> Result<bool, Fault> {
    let slot = buffer::read_slot(h.get(Region::Block(at.block))?, at.slot)?;
    if slot.flags & buffer::SLOT_DEAD != 0 {
        return Ok(false);
    }
    if at.frozen {
        if let Some(rw) = seal_of(h, at.desc)? {
            let b = h.get(Region::Block(rw.pending))?;
            return Ok(rd_u8(b, page::P_BITMAP.wrapping_add(at.pos / 8))? >> (at.pos % 8) & 1 != 0);
        }
    }
    Ok(true)
}

/// Leaf `l`'s current value in a buffered row, `walk` positioned at the leaf (and moved past it).
pub fn buffered_value<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    b: &Buffered,
    row: &RowRef<'_, H>,
    walk: &mut Walk,
    l: &LeafInfo,
) -> Result<SVal, Fault> {
    let vi = walk.var;
    let s = walk.next(shape, row, l)?;
    if b.post != 0 {
        if let Some(e) = patch::lookup(h, b.post, u64::from(l.ordinal), b.at.pos)? {
            return cur::from_entry(h, &e, l);
        }
    }
    match s {
        super::row::Stored::Absent => super::defaults::of(h, shape, l),
        s => cur::from_stored(row, shape, l, &s, vi),
    }
}

/// Leaf `l`'s current value in a buffered row, found from its ordinal (O(L) for the leaf's offsets).
pub fn buffered_leaf<H: PageHost + ?Sized>(h: &H, shape: &Shape, b: &Buffered, l: &LeafInfo) -> Result<SVal, Fault> {
    let row = RowRef::new(h, b.loc)?;
    let mut walk = Walk::of(shape, &row)?;
    for o in 0..u64::from(l.ordinal) {
        walk.skip(&shape.leaf(h, o)?);
    }
    buffered_value(h, shape, b, &row, &mut walk, l)
}
