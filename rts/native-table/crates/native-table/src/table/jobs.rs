//! Maintenance: the jobs a table's writes leave pending, their order, and the one step a write's first call
//! runs.
//!
//! The one job is the compaction queue: compactions and group drops. A step runs its next piece: a compaction
//! rewrites one whole group (`seal`), a drop frees one group. A queued compaction is urgent while a patch chain
//! or a group's patch units passed their bounds; the seal of a full group is no job: the write that fills the
//! group runs it (`write::freeze`).
//!
//! **Compaction queue** `NTQB` (area 0, zero-filled when allocated): {magic u32 @0; `FORMAT_TABLE` u16 @4;
//! kind u8 @6 (1 compaction); head u16 @8 (the next entry to pop); entries written u16 @10; next block u64
//! @16 (towards the tail); 508 group numbers u64 from @32}. The table root names the head block at 112 and
//! the tail block at 120, and at 128 a group a write recorded for the next step to push (`after_patch`). A
//! group is queued at most once (its group page's flag bit 1).

use crate::consts::{magic, COMPACT_DEAD, FORMAT_TABLE, PATCH_BLOCKS_MAX, PATCH_FOLD_BLOCKS, PATCH_UNITS_PER_DIR};
use crate::fault::{rt, Corruption, Fault};
use crate::host::{wake_from_state, Wake, STATE_PENDING, STATE_URGENT};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, PageHost, Region};

use super::buffer::{self, DESC_BIT};
use super::drain;
use super::group::GroupView;
use super::layout::Shape;
use super::page;
use super::patch;
use super::root;
use super::seal;

const Q_KIND: u64 = 6;
const Q_HEAD: u64 = 8;
const Q_ENTRIES: u64 = 10;
const Q_NEXT: u64 = 16;
const Q_FIRST: u64 = 32;
const Q_PER_BLOCK: u64 = 508;

// ---------------------------------------------------------------- the compaction queue

/// Pushes group g onto the compaction queue.
fn push<H: PageHost>(h: &mut H, tbl: u64, g: u64) -> Result<(), Fault> {
    let tail = root::u64_at(&*h, tbl, root::COMPACT_TAIL)?;
    let n = if tail == 0 { Q_PER_BLOCK } else { u64::from(rd_u16(h.get(Region::Block(tail))?, Q_ENTRIES)?) };
    let tail = if n >= Q_PER_BLOCK {
        let b = crate::area::alloc_zeroed(h, tbl, 0)?;
        {
            let s = h.get_mut(Region::Block(b))?;
            put_u32(s, 0, magic::NTQB)?;
            put_u16(s, 4, FORMAT_TABLE)?;
            put_u8(s, Q_KIND, 1)?;
        }
        if tail != 0 {
            put_u64(h.get_mut(Region::Block(tail))?, Q_NEXT, b)?;
        } else {
            root::set_u64(h, tbl, root::COMPACT_HEAD, b)?;
        }
        root::set_u64(h, tbl, root::COMPACT_TAIL, b)?;
        b
    } else {
        tail
    };
    let s = h.get_mut(Region::Block(tail))?;
    let n = u64::from(rd_u16(s, Q_ENTRIES)?);
    put_u64(s, Q_FIRST.wrapping_add(n.wrapping_mul(8)), g)?;
    put_u16(s, Q_ENTRIES, u16::try_from(n.wrapping_add(1)).map_err(|_| rt(Corruption::Count, n))?)
}

/// Pops the queue's first group number, freeing an emptied head block.
fn pop<H: PageHost>(h: &mut H, tbl: u64) -> Result<Option<u64>, Fault> {
    loop {
        let head = root::u64_at(&*h, tbl, root::COMPACT_HEAD)?;
        if head == 0 {
            return Ok(None);
        }
        let (i, n, next) = {
            let s = h.get(Region::Block(head))?;
            (u64::from(rd_u16(s, Q_HEAD)?), u64::from(rd_u16(s, Q_ENTRIES)?), rd_u64(s, Q_NEXT)?)
        };
        if i < n {
            let g = rd_u64(h.get(Region::Block(head))?, Q_FIRST.wrapping_add(i.wrapping_mul(8)))?;
            let i2 = i.wrapping_add(1);
            if i2 >= n {
                free_head(h, tbl, head, next)?;
            } else {
                put_u16(h.get_mut(Region::Block(head))?, Q_HEAD, u16::try_from(i2).unwrap_or(0))?;
            }
            return Ok(Some(g));
        }
        free_head(h, tbl, head, next)?;
    }
}

fn free_head<H: PageHost>(h: &mut H, tbl: u64, head: u64, next: u64) -> Result<(), Fault> {
    root::set_u64(h, tbl, root::COMPACT_HEAD, next)?;
    if next == 0 {
        root::set_u64(h, tbl, root::COMPACT_TAIL, 0)?;
    }
    crate::area::free_block(h, tbl, 0, head)
}

/// Whether the compaction queue holds a group, or the root a group to queue.
pub fn queued<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(root::u64_at(h, tbl, root::COMPACT_HEAD)? != 0 || root::u64_at(h, tbl, root::GROUP_TO_QUEUE)? != 0)
}

/// A step's first piece: the root's group to queue, when it names one, pushed onto the compaction queue.
fn push_recorded<H: PageHost>(h: &mut H, tbl: u64) -> Result<(), Fault> {
    let r = root::u64_at(&*h, tbl, root::GROUP_TO_QUEUE)?;
    if r == 0 {
        return Ok(());
    }
    push(h, tbl, r.wrapping_sub(1))?;
    root::set_u64(h, tbl, root::GROUP_TO_QUEUE, 0)
}

/// Queues group g's compaction (or drop), once; `urgent` makes the compaction job urgent.
pub fn queue<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64, urgent: bool) -> Result<(), Fault> {
    let flags = buffer::u16_of(&*h, pg, page::P_FLAGS)?;
    if flags & u64::from(page::PF_QUEUED) == 0 {
        put_u16(h.get_mut(Region::Block(pg))?, page::P_FLAGS, u16::try_from(flags).unwrap_or(0) | page::PF_QUEUED)?;
        push(h, tbl, g)?;
    }
    if urgent {
        let f = root::u8_at(&*h, tbl, root::FLAGS)?;
        if f & root::FLAG_COMPACT_URGENT == 0 {
            root::set_u8(h, tbl, root::FLAGS, f | root::FLAG_COMPACT_URGENT)?;
        }
    }
    Ok(())
}

/// After a delete of a sealed row: a group with no live row, or whose dead positions reach `COMPACT_DEAD`,
/// is queued.
pub fn after_delete<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    let live = buffer::u16_of(&*h, pg, page::P_LIVE)?;
    let dead = buffer::u16_of(&*h, pg, page::P_DEAD)?;
    if live == 0 || dead >= COMPACT_DEAD {
        queue(h, tbl, g, pg, false)?;
    }
    Ok(())
}

/// After a patch: a chain at `PATCH_FOLD_BLOCKS` blocks, or live patch bytes at half the image's variable
/// bytes, queue the group's compaction; a chain at `PATCH_BLOCKS_MAX` blocks (which passed the first trigger)
/// or a 255th unit make it urgent. A group whose 255th unit finds it not queued passes no fold trigger: it is
/// recorded in the root's group to queue, which the write changes anyway, and the next step pushes it.
pub fn after_patch<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64, u: &patch::Upserted) -> Result<(), Fault> {
    let urgent = u.blocks >= PATCH_BLOCKS_MAX || u.units > PATCH_UNITS_PER_DIR;
    let var = variable_bytes(&*h, pg)?;
    if u.blocks >= PATCH_FOLD_BLOCKS || u.live_bytes.saturating_mul(2) >= var.max(1) {
        return queue(h, tbl, g, pg, urgent);
    }
    if !urgent {
        return Ok(());
    }
    let flags = buffer::u16_of(&*h, pg, page::P_FLAGS)?;
    let recorded = root::u64_at(&*h, tbl, root::GROUP_TO_QUEUE)?;
    if flags & u64::from(page::PF_QUEUED) != 0 || recorded != 0 {
        // Queued already, or another group recorded with no step run since: the queue takes this one now.
        return queue(h, tbl, g, pg, true);
    }
    put_u16(h.get_mut(Region::Block(pg))?, page::P_FLAGS, u16::try_from(flags).unwrap_or(0) | page::PF_QUEUED)?;
    root::set_u64(h, tbl, root::GROUP_TO_QUEUE, g.checked_add(1).ok_or(rt(Corruption::Range, g))?)?;
    let f = root::u8_at(&*h, tbl, root::FLAGS)?;
    if f & root::FLAG_COMPACT_URGENT == 0 {
        root::set_u8(h, tbl, root::FLAGS, f | root::FLAG_COMPACT_URGENT)?;
    }
    Ok(())
}

/// A group image's bytes past its front metadata: its cells, inline data and out-of-line values.
fn variable_bytes<H: PageHost + ?Sized>(h: &H, pg: u64) -> Result<u64, Fault> {
    let gv = GroupView::open(h, pg)?;
    let hd = gv.reader().bytes(0, super::group::HEADER)?;
    let hh = u64::from(rd_u32(hd, super::group::G_H)?);
    Ok(gv.segs_at.saturating_sub(hh))
}

/// After a rewrite: the group it installed is queued to be dropped when it holds no live row. Its new image
/// holds no patch, and fewer than `COMPACT_DEAD` dead positions (`seal::compacted_form`).
pub fn after_rewrite<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    if buffer::u16_of(&*h, pg, page::P_LIVE)? == 0 {
        queue(h, tbl, g, pg, false)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- group drops

/// Drops group page `pg` of group g, which holds no live row: its directory entry is cleared, and the page is
/// freed with its image, long-value list, patches and extremes block.
fn drop_group<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    let mut dir = root::directory(&*h, tbl)?;
    let before = dir;
    dir.clear(h, tbl, g)?;
    if dir != before {
        root::set_directory(h, tbl, dir)?;
    }
    drain::free_group_page(h, tbl, pg)
}

// ---------------------------------------------------------------- steps

/// What one step ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ran {
    Nothing,
    Compaction,
    Drop,
}

/// Runs one step: the compaction queue's next piece. Returns what ran (`Ran::Nothing` when no job is pending).
pub fn step<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<Ran, Fault> {
    push_recorded(h, tbl)?;
    let ran = if queued(&*h, tbl)? { compaction(h, tbl, shape)? } else { Ran::Nothing };
    settle(h, tbl)?;
    Ok(ran)
}

/// A compaction step: the queue's next group compacted whole, or dropped when it holds no live row.
fn compaction<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape) -> Result<Ran, Fault> {
    let Some(g) = pop(h, tbl)? else { return Ok(Ran::Nothing) };
    let pg = root::group(&*h, tbl, g)?;
    if pg == 0 || pg & DESC_BIT != 0 {
        // The group was dropped since it was queued.
        return Ok(Ran::Compaction);
    }
    let flags = buffer::u16_of(&*h, pg, page::P_FLAGS)?;
    put_u16(h.get_mut(Region::Block(pg))?, page::P_FLAGS, u16::try_from(flags).unwrap_or(0) & !page::PF_QUEUED)?;
    if buffer::u16_of(&*h, pg, page::P_LIVE)? == 0 {
        drop_group(h, tbl, g, pg)?;
        return Ok(Ran::Drop);
    }
    seal::rewrite(h, tbl, shape, pg, seal::COMPACTION)?;
    Ok(Ran::Compaction)
}

/// The maintenance state: pending while a compaction waits, urgent while an urgent one does; the root's state
/// byte is written only when it changes.
pub fn settle<H: PageHost>(h: &mut H, tbl: u64) -> Result<Wake, Fault> {
    let comp_w = queued(&*h, tbl)?;
    let mut flags = root::u8_at(&*h, tbl, root::FLAGS)?;
    let old_flags = flags;
    if !comp_w {
        flags &= !root::FLAG_COMPACT_URGENT;
    }
    if flags != old_flags {
        root::set_u8(h, tbl, root::FLAGS, flags)?;
    }
    let mut s = 0u8;
    if comp_w {
        s |= STATE_PENDING;
    }
    if comp_w && flags & root::FLAG_COMPACT_URGENT != 0 {
        s |= STATE_URGENT;
    }
    let old = root::u8_at(&*h, tbl, root::STATE)?;
    if s != old {
        root::set_u8(h, tbl, root::STATE, s)?;
    }
    Ok(wake_from_state(s))
}
