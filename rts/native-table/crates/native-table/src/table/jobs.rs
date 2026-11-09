//! Maintenance: the jobs a table's writes leave pending, their order, and the one unit a write's first step
//! runs.
//!
//! Jobs, in order: seals (the frozen queue, oldest first); compactions and group drops (the compaction
//! queue); the column release (job slot 0); the builds of indexes a schema change added (job slot 1,
//! `indexes::build_unit`); the drain. An urgent job runs first: the seals while two groups
//! are frozen, a compaction while a patch chain or a group's patch units passed their bounds. A rewrite runs from the snapshot to its last unit in its own job's units, its state
//! block named from its job slot, so it continues whatever the queues hold meanwhile.
//!
//! **One unit, one bound.** A unit starts with one job and, when that job runs out of work or its next
//! piece does not fit, goes on with the next pending job (urgent ones first), skipping every job whose next
//! piece does not fit, each job at most once: the bound (`UNIT_WORK` estimated instructions, `BYTE_UNIT`
//! bytes, `UNIT_BLOCKS_MAX` changed blocks) is counted across all of them. A rewrite's flip ends its job's
//! share of the unit, so no unit both allocates a group page and flips it.
//!
//! **Compaction queue** `NTQB` (area 0, zero-filled when allocated): {magic u32 @0; `FORMAT_TABLE` u16 @4;
//! kind u8 @6 (1 compaction); head u16 @8 (the next entry to pop); entries written u16 @10; next block u64
//! @16 (towards the tail); 508 group numbers u64 from @32}. The table root names the head block at 112 and
//! the tail block at 120, and at 128 a group a write recorded for the next unit to push (`after_patch`). A
//! group is queued at most once (its group page's flag bit 1).

use core::cell::Cell;

use crate::consts::{
    magic, BYTE_UNIT, COMPACT_DEAD, FORMAT_TABLE, PATCH_BLOCKS_MAX, PATCH_FOLD_BLOCKS, PATCH_UNITS_PER_DIR,
    UNIT_BLOCKS_MAX,
};
use crate::fault::{rt, Corruption, Fault};
use crate::host::{wake_from_state, Budget, Progress, Wake, STATE_PENDING, STATE_URGENT};
use crate::mem::{put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, PageHost, Region};
use crate::radix::{Units, HEIGHT_MAX};

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

/// A unit's first step: the root's group to queue, when it names one, pushed onto the compaction queue.
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
/// is queued. A group a rewrite holds is checked when the rewrite ends.
pub fn after_delete<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    if buffer::u64_of(&*h, pg, page::P_REWRITE)? != 0 {
        return Ok(());
    }
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
/// recorded in the root's group to queue, which the write changes anyway, and the next unit pushes it.
pub fn after_patch<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64, u: &patch::Upserted) -> Result<(), Fault> {
    if buffer::u64_of(&*h, pg, page::P_REWRITE)? != 0 {
        return Ok(());
    }
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
        // Queued already, or another group recorded with no unit run since: the queue takes this one now.
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

/// After a rewrite's last unit: the group it installed is queued when it has no live row (to be dropped),
/// or when the writes during the rewrite left it with `COMPACT_DEAD` dead positions or long patch chains.
pub fn after_rewrite<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    if root::group(&*h, tbl, g)? != pg {
        return Ok(());
    }
    let live = buffer::u16_of(&*h, pg, page::P_LIVE)?;
    let dead = buffer::u16_of(&*h, pg, page::P_DEAD)?;
    let mut due = live == 0 || dead >= COMPACT_DEAD;
    let mut urgent = false;
    let dir = buffer::u64_of(&*h, pg, page::P_PATCH_DIR)?;
    if dir != 0 {
        let units = patch::units(&*h, dir)?;
        let mut longest = 0u64;
        patch::for_each_unit(&*h, dir, |u| {
            longest = longest.max(u.blocks);
            Ok(())
        })?;
        urgent = longest >= PATCH_BLOCKS_MAX || units > PATCH_UNITS_PER_DIR;
        let var = variable_bytes(&*h, pg)?;
        due = due
            || urgent
            || longest >= PATCH_FOLD_BLOCKS
            || patch::live_bytes(&*h, dir)?.saturating_mul(2) >= var.max(1);
    }
    if due {
        queue(h, tbl, g, pg, urgent)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- group drops

/// Drops group page `pg` of group g, which holds no live row, in one piece: its directory entry is cleared,
/// and the page goes to the drain with its image, long-value list and patches.
fn drop_group<H: PageHost>(h: &mut H, tbl: u64, g: u64, pg: u64) -> Result<(), Fault> {
    let mut dir = root::directory(&*h, tbl)?;
    let before = dir;
    dir.clear(h, tbl, g)?;
    if dir != before {
        root::set_directory(h, tbl, dir)?;
    }
    drain::push(h, tbl, &drain::Rec::new(drain::K_GROUP_PAGE, pg, 0))
}

// ---------------------------------------------------------------- units

/// What one unit did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ran {
    Nothing,
    Seal,
    Compaction,
    Drop,
    Release,
    Build,
    Drain,
}

fn seal_work<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(seal::running(h, tbl, seal::SEAL)? != 0 || root::u64_at(h, tbl, root::FROZEN_HEAD)? != 0)
}

fn compaction_work<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(seal::running(h, tbl, seal::COMPACTION)? != 0 || queued(h, tbl)?)
}

fn release_work<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(seal::running(h, tbl, seal::RELEASE)? != 0 || root::u8_at(h, tbl, root::FLAGS)? & root::FLAG_RELEASE != 0)
}

/// The seals are urgent from the freeze of a second group; an update of a frozen group's row raises nothing
/// (the seal is pending from the freeze and first in order), so it writes no root block.
fn seal_urgent<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(root::u32_at(h, tbl, root::FROZEN_WAITING)? >= 2)
}

fn compaction_urgent<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<bool, Fault> {
    Ok(root::u8_at(h, tbl, root::FLAGS)? & root::FLAG_COMPACT_URGENT != 0 && compaction_work(h, tbl)?)
}

/// Distinct blocks a unit's count keeps by address; past them the count is past the bound.
const SEEN: usize = 32;

/// The 4 KiB pages one unit changed, in any memory, counted exactly: every block a write was lent for once (a
/// page's sixteen), the host's own pages each `page_alloc` may change (`PageHost::alloc_changes`), and for n
/// frees the host's free-page stack and state block (1 + ⌈n/511⌉ + 1).
pub struct Pages {
    seen: [Cell<u64>; SEEN],
    n: Cell<u64>,
    last: Cell<u64>,
    allocs: Cell<u64>,
    frees: Cell<u64>,
    alloc_changes: u64,
}

impl Pages {
    pub fn new(alloc_changes: u64) -> Pages {
        Pages {
            seen: [const { Cell::new(0) }; SEEN],
            n: Cell::new(0),
            last: Cell::new(0),
            allocs: Cell::new(0),
            frees: Cell::new(0),
            alloc_changes,
        }
    }

    pub fn count(&self) -> u64 {
        let f = self.frees.get();
        let free_pages = if f == 0 { 0 } else { f.div_ceil(511).saturating_add(2) };
        self.n.get().saturating_add(self.allocs.get()).saturating_add(free_pages)
    }

    /// The host's own pages one more `page_alloc` may change, the page it returns not included.
    pub fn alloc_changes(&self) -> u64 {
        self.alloc_changes
    }

    pub fn has(&self, b: u64) -> bool {
        let n = usize::try_from(self.n.get()).unwrap_or(SEEN).min(SEEN);
        self.seen.iter().take(n).any(|c| c.get() == b)
    }

    pub fn freed(&self) -> bool {
        self.frees.get() > 0
    }

    fn mark(&self, b: u64) {
        if b == self.last.get() {
            return;
        }
        self.last.set(b);
        if self.has(b) {
            return;
        }
        if let Some(c) = self.seen.get(usize::try_from(self.n.get()).unwrap_or(SEEN)) {
            c.set(b);
        }
        self.n.set(self.n.get().saturating_add(1));
    }
}

/// The host a table runs every call on: the caller's host, and while a maintenance unit runs, the unit's
/// `Pages`, which count its writes, allocations and frees. Calls and units share this one type, so the
/// crate's code is compiled once for each host.
pub struct Tracked<'a, H: PageHost> {
    pub(super) h: &'a mut H,
    pages: Option<&'a Pages>,
}

impl<'a, H: PageHost> Tracked<'a, H> {
    /// The host, with no unit counting.
    pub(super) fn new(h: &'a mut H) -> Tracked<'a, H> {
        Tracked { h, pages: None }
    }

    #[inline(always)]
    fn note(&self, r: Region) {
        let Some(pages) = self.pages else { return };
        match r {
            Region::Block(b) => pages.mark(b),
            Region::Page(p) => {
                for i in 0..16u32 {
                    pages.mark(self.h.block_of(p, i));
                }
            }
        }
    }
}

/// Every method inlined: outside a unit, a host access costs the host's own and one test of `pages`.
impl<H: PageHost> PageHost for Tracked<'_, H> {
    #[inline(always)]
    fn block_of(&self, page: u64, i: u32) -> u64 {
        self.h.block_of(page, i)
    }
    #[inline(always)]
    fn get(&self, r: Region) -> Result<&[u8], Fault> {
        self.h.get(r)
    }
    #[inline(always)]
    fn get_mut(&mut self, r: Region) -> Result<&mut [u8], Fault> {
        self.note(r);
        self.h.get_mut(r)
    }
    #[inline(always)]
    fn pair_mut(&mut self, a: Region, b: Region) -> Result<(&mut [u8], &mut [u8]), Fault> {
        self.note(a);
        self.note(b);
        self.h.pair_mut(a, b)
    }
    #[inline(always)]
    fn read_write(&mut self, r: Region, wr: Region) -> Result<(&[u8], &mut [u8]), Fault> {
        self.note(wr);
        self.h.read_write(r, wr)
    }
    #[inline(always)]
    fn page_alloc(&mut self) -> Result<u64, Fault> {
        if let Some(pages) = self.pages {
            pages.allocs.set(pages.allocs.get().saturating_add(self.h.alloc_changes()));
        }
        self.h.page_alloc()
    }
    #[inline(always)]
    fn page_free(&mut self, p: u64) -> Result<(), Fault> {
        if let Some(pages) = self.pages {
            pages.frees.set(pages.frees.get().saturating_add(1));
        }
        self.h.page_free(p)
    }
    #[inline(always)]
    fn alloc_changes(&self) -> u64 {
        self.h.alloc_changes()
    }
    #[inline(always)]
    fn test_fill(&self) -> Option<u8> {
        self.h.test_fill()
    }
}

/// What a unit's readings of the host's meter saw: the count at the last one, and the longest stretch between
/// two of them, which the look-ahead before each piece takes as the next piece's most.
pub struct Readings {
    last: Cell<u64>,
    longest: Cell<u64>,
    /// Whether the host has a meter: 0 not known yet, 1 no, 2 yes.
    metered: Cell<u8>,
}

impl Readings {
    pub const fn new() -> Readings {
        Readings { last: Cell::new(0), longest: Cell::new(0), metered: Cell::new(0) }
    }
}

impl Default for Readings {
    fn default() -> Self {
        Self::new()
    }
}

/// The host's instruction meter as one unit reads it (`Budget::used()` since the unit began, 0 where the host
/// has no meter: the builder, the test host), its readings, and the pages the unit changed.
#[derive(Clone, Copy)]
pub struct Clock<'b> {
    meter: &'b dyn Budget,
    start: u64,
    pages: &'b Pages,
    readings: &'b Readings,
}

/// On a host without a meter, the unit's count is its own estimate scaled by `EST_NUM / EST_DEN`: the
/// estimates are calibrated on wasmtime fuel, and the replica's count of the same work is up to about 2.4
/// times as much (1.21 per fuel, and the bulk copies and fills fuel counts as one instruction each).
const EST_NUM: u64 = 5;
const EST_DEN: u64 = 2;

/// On a host with a meter, the least the look-ahead before a piece takes as the piece's most: what any one
/// piece of any job costs at most, as the replica counts it (`seal::PIECE_MAX`, which bounds every piece,
/// estimated and counted alike); a piece that may cost more checks the room for its own estimate first. The
/// in-place walk, which reads the meter itself once every `walk::CHECK_EVERY` of its estimate, takes
/// `STRETCH_METERED` as the most between two of its readings: that estimate as the replica counts it, a last
/// position's value and its piece's end. And what a unit's end after its last piece costs at most (a share's
/// saves, the staged output's flush, the next job's checks, the maintenance state settled).
pub const PIECE_METERED: u64 = seal::PIECE_MAX;
pub const STRETCH_METERED: u64 = 16_000;
pub const TAIL_METERED: u64 = 14_000;
/// A decision's note when it does not check the unit's pages (`Clock::note`).
#[cfg(feature = "meter-notes")]
pub const NO_PAGES: u64 = u64::MAX;

impl<'b> Clock<'b> {
    pub fn start(meter: &'b dyn Budget, pages: &'b Pages, readings: &'b Readings) -> Self {
        Clock { meter, start: meter.used(), pages, readings }
    }

    /// The look-ahead before a piece estimated at `est`: on a host with a meter, the most of that estimate as
    /// the host counts it (scaled), `PIECE_METERED` and the longest stretch the unit saw between two readings,
    /// plus the unit's end; else the scaled estimate.
    pub fn ahead(&self, est: u64) -> u64 {
        if self.metered() {
            scaled(est).max(PIECE_METERED).max(self.readings.longest.get()).saturating_add(TAIL_METERED)
        } else {
            scaled(est)
        }
    }

    /// With the `meter-notes` feature, tells a metered host the look-ahead a decision took for the work it lets
    /// begin, `ahead` less the unit's end (0 when the work ends there), and the pages it took that work to
    /// change at most (`NO_PAGES` when it does not check them; `Budget::note`).
    #[cfg(feature = "meter-notes")]
    #[track_caller]
    pub fn note(&self, ahead: u64, pages: u64) {
        self.note_site(ahead, pages, core::panic::Location::caller());
    }
    #[cfg(feature = "meter-notes")]
    pub fn note_site(&self, ahead: u64, pages: u64, at: &'static core::panic::Location<'static>) {
        if self.metered() {
            self.meter.note(ahead.saturating_sub(TAIL_METERED), pages, at.file(), at.line());
        }
    }

    /// The look-ahead before a stretch of the in-place walk: on a host with a meter, the longest stretch the
    /// unit saw between two readings, at least `STRETCH_METERED`, plus the unit's end.
    pub fn ahead_walk(&self) -> u64 {
        STRETCH_METERED.max(self.readings.longest.get()).saturating_add(TAIL_METERED)
    }

    /// Whether the host has a meter: its count rose since the unit began (asked once a unit).
    pub fn metered(&self) -> bool {
        match self.readings.metered.get() {
            0 => {
                let m = self.since() > 0;
                self.readings.metered.set(if m { 2 } else { 1 });
                m
            }
            k => k == 2,
        }
    }

    pub fn pages(&self) -> u64 {
        self.pages.count()
    }

    /// The pages one more piece may change at most: its own (`PIECE_PAGES`), one allocation's (a block, its
    /// area's page-list block and the host's own pages) and the radix node a block list may take with it (a
    /// work space's first block), and the table root's at the unit's end when the unit has not changed it
    /// yet.
    pub fn piece_pages(&self, tbl: u64) -> u64 {
        let root = u64::from(!self.pages.has(tbl));
        PIECE_PAGES.saturating_add(3).saturating_add(self.pages.alloc_changes()).saturating_add(root)
    }

    pub fn changed(&self, b: u64) -> bool {
        self.pages.has(b)
    }

    pub fn freed(&self) -> bool {
        self.pages.freed()
    }

    /// The host's own pages one more `page_alloc` may change.
    pub fn alloc_changes(&self) -> u64 {
        self.pages.alloc_changes()
    }

    /// Whether one more piece's pages (`piece_pages`) fit the unit.
    pub fn pages_fit(&self, tbl: u64) -> bool {
        self.pages().saturating_add(self.piece_pages(tbl)) <= UNIT_BLOCKS_MAX
    }

    /// The pages work may change that changes `k` blocks and makes `allocs` allocations besides (each changing
    /// the host's own pages, `alloc_changes`), and the table root when the unit has not changed it yet.
    #[inline]
    pub fn pages_for(&self, k: u64, allocs: u64, tbl: u64) -> u64 {
        k.saturating_add(allocs.saturating_mul(self.alloc_changes())).saturating_add(u64::from(!self.changed(tbl)))
    }

    /// Whether work estimated at `est` fits the unit beside its count so far (`count(work)`), the unit's end
    /// left room, and with it `pages` more changed pages (`None`: not checked).
    #[cfg_attr(feature = "meter-notes", track_caller)]
    #[inline(always)]
    pub fn room(&self, work: u64, est: u64, pages: Option<u64>) -> bool {
        let ok = self.count(work).saturating_add(self.ahead(est)) <= UNIT_WORK
            && pages.is_none_or(|p| self.pages().saturating_add(p) <= UNIT_BLOCKS_MAX);
        // The decision's look-ahead and pages, told to a metered host.
        #[cfg(feature = "meter-notes")]
        self.note(if ok { self.ahead(est) } else { 0 }, pages.map_or(NO_PAGES, |p| if ok { p } else { 0 }));
        ok
    }

    /// Instructions the meter counted since the unit began.
    pub fn since(&self) -> u64 {
        self.meter.used().saturating_sub(self.start)
    }

    /// On a host with a meter, whether its reading now (noted) and the look-ahead before a piece of at most
    /// `PIECE_METERED` pass `UNIT_WORK`: `count` and `ahead` in one.
    pub fn over(&self) -> bool {
        let s = self.since();
        let r = self.readings;
        let longest = r.longest.get().max(s.saturating_sub(r.last.get()));
        r.longest.set(longest);
        r.last.set(s);
        s.saturating_add(longest.max(PIECE_METERED)).saturating_add(TAIL_METERED) > UNIT_WORK
    }

    /// The unit's count c: the host's meter where it has one (it counted since the unit began, as
    /// `ic0.performance_counter` always does; each reading noted), else the unit's own estimate `work`,
    /// scaled. The builder and the test host have no meter.
    pub fn count(&self, work: u64) -> u64 {
        let s = self.since();
        if s > 0 {
            let r = self.readings;
            r.longest.set(r.longest.get().max(s.saturating_sub(r.last.get())));
            r.last.set(s);
            s
        } else {
            scaled(work)
        }
    }
}

/// An estimate as a host without a meter counts it: `EST_NUM / EST_DEN` of it, saturating (tested against a
/// bound rather than by an overflowing product, which wasm computes in a call).
pub fn scaled(est: u64) -> u64 {
    let half = est / EST_DEN;
    if half > u64::MAX / EST_NUM / 2 {
        return u64::MAX;
    }
    half.wrapping_mul(EST_NUM).wrapping_add(est % EST_DEN)
}

/// The estimate a count without a meter stands for: `scaled` undone.
pub fn unscaled(count: u64) -> u64 {
    // count / EST_NUM · EST_DEN ≤ count: no overflow.
    (count / EST_NUM).wrapping_mul(EST_DEN)
}

/// The most blocks one piece of any job changes besides an allocation and the table root: its output's, its
/// state block and one more structure's.
pub const PIECE_PAGES: u64 = 3;

/// What one unit spent across the jobs it touched: its own estimate of its instructions (a `Clock` counts the
/// host's meter instead where it has one), value and image bytes read or written, and changed 4 KiB pages
/// (counted exactly, by `Pages`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    pub work: u64,
    pub bytes: u64,
    pub blocks: u64,
}

/// A unit's bound on its instructions, counted across every job it touches.
pub const UNIT_WORK: u64 = 96_000;
/// What a unit's own work costs that no piece's estimate counts (its start and end, each job's state read and
/// saved, the jobs' checks between them), as wasmtime counts it: a unit's estimate starts there, so that on a
/// host without a meter the unit's count leaves it room.
const UNIT_OVERHEAD: u64 = 5_000;
/// The drain's state load and save: its top record read and written.
const DRAIN_LOAD: u64 = 2_000;
/// One drain piece's allowance: radix entries examined (a buffer descriptor's row counts `ROW_ENTRIES`, an item
/// of its item table four, a long row image's head 24: a row and its head, 32, the most one step takes
/// without moving on), and pages freed. Small, so that a piece stays within one stretch between two readings
/// of a host's meter.
const DRAIN_PIECE_ENTRIES: u64 = 32;
const DRAIN_PIECE_PAGES: u64 = 4;
/// Estimated instructions, as wasmtime counts them (fuel), per radix entry examined, page freed, block freed
/// and record worked on: each at least the most measured (`scripts/unit_fuel.sh`).
const DRAIN_ENTRY_WORK: u64 = 240;
const DRAIN_PAGE_WORK: u64 = 400;
const DRAIN_BLOCK_WORK: u64 = 400;
const DRAIN_RECORD_WORK: u64 = 1_500;
/// The most one drain piece costs: its whole allowance at those rates, every block of a unit freed in it.
const DRAIN_PIECE_WORK: u64 = DRAIN_RECORD_WORK
    + DRAIN_PIECE_ENTRIES * DRAIN_ENTRY_WORK
    + DRAIN_PIECE_PAGES * DRAIN_PAGE_WORK
    + crate::radix::DRAIN_BLOCKS * DRAIN_BLOCK_WORK;
/// The pages a drain piece changes besides the radix blocks it frees: its drain block and one more for a
/// drain block emptied or pushed, the host's free-page stack and state block before the unit's first free
/// (three), or one more stack block after it, and the table root when the unit has not changed it yet.
fn drain_pages(clock: &Clock<'_>, tbl: u64) -> u64 {
    clock.pages_for(if clock.freed() { 3 } else { 5 }, 0, tbl)
}
/// The pages a rewrite's first piece in a unit may change: a snapshot's state block, pending page and work
/// page, each allocated (two area page-list blocks and the host's own pages besides), the table root, the
/// descriptor or group page it marks, and one piece's pages after it.
fn start_pages(clock: &Clock<'_>, tbl: u64) -> u64 {
    6u64.saturating_add(clock.alloc_changes().saturating_mul(3)).saturating_add(clock.piece_pages(tbl))
}
/// Estimated instructions of a group drop, and of a compaction queue pop.
const DROP_WORK: u64 = 12_000;
const POP_WORK: u64 = 400;

/// The kinds of job, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Job {
    Seal,
    Compaction,
    Release,
    Build,
    Drain,
}

const JOBS: [Job; 5] = [Job::Seal, Job::Compaction, Job::Release, Job::Build, Job::Drain];

fn index(j: Job) -> usize {
    match j {
        Job::Seal => 0,
        Job::Compaction => 1,
        Job::Release => 2,
        Job::Build => 3,
        Job::Drain => 4,
    }
}

fn pending_job<H: PageHost + ?Sized>(h: &H, tbl: u64, j: Job) -> Result<bool, Fault> {
    match j {
        Job::Seal => seal_work(h, tbl),
        Job::Compaction => compaction_work(h, tbl),
        Job::Release => release_work(h, tbl),
        Job::Build => Ok(super::indexes::building(h, tbl)? != 0),
        Job::Drain => drain::pending(h, tbl),
    }
}

fn urgent_job<H: PageHost + ?Sized>(h: &H, tbl: u64, j: Job) -> Result<bool, Fault> {
    Ok(match j {
        Job::Seal => seal_work(h, tbl)? && seal_urgent(h, tbl)?,
        Job::Compaction => compaction_urgent(h, tbl)?,
        Job::Release | Job::Build | Job::Drain => false,
    })
}

/// Whether job j's next piece fits beside what the unit spent: its state's load and save, its next piece (a
/// rewrite's piece, its snapshot, a group drop or a flip, at most `seal::PIECE_MAX`; a drain piece), and the
/// blocks its first piece may change. The count is the host's meter where it has one, else the estimates.
fn fits(j: Job, s: &Spent, clock: &Clock<'_>, tbl: u64) -> bool {
    let (piece, pages) = match j {
        Job::Seal | Job::Compaction | Job::Release => (seal::PIECE_MAX, start_pages(clock, tbl)),
        // A build checks the room for each row itself, its first included when it does not begin the unit.
        Job::Build => (0, 0),
        Job::Drain => {
            (DRAIN_LOAD.wrapping_add(DRAIN_PIECE_WORK), drain_pages(clock, tbl).saturating_add(u64::from(HEIGHT_MAX)))
        }
    };
    // A unit past `BYTE_UNIT` bytes has no room: no estimate fits it.
    clock.room(s.work, if s.bytes < BYTE_UNIT { piece } else { u64::MAX }, Some(pages))
}

/// Whether a rewrite's next piece fits beside what the unit spent (`fits`), for a job that runs another
/// job's rewrite.
pub fn rewrite_fits(s: &Spent, clock: &Clock<'_>, tbl: u64) -> bool {
    fits(Job::Compaction, s, clock, tbl)
}

/// The next job the unit has not touched: urgent jobs first, each in order.
fn next_job<H: PageHost + ?Sized>(h: &H, tbl: u64, done: &[bool; 5]) -> Result<Option<Job>, Fault> {
    for urgent_only in [true, false] {
        for j in JOBS {
            if done.get(index(j)).copied().unwrap_or(true) || !pending_job(h, tbl, j)? {
                continue;
            }
            if !urgent_only || urgent_job(h, tbl, j)? {
                return Ok(Some(j));
            }
        }
    }
    Ok(None)
}

/// The urgent kinds' codes, a format the rotation cursor stores: 1 seal, 2 compaction, then six kinds this
/// table has no job of (subtree counts, dead-range coalescing, conversion, merge, purge, end trim). The column
/// release and the drain are never urgent, so they have none.
const URGENT_CODES: u64 = 8;

fn urgent_code(j: Job) -> Option<u64> {
    match j {
        Job::Seal => Some(1),
        Job::Compaction => Some(2),
        Job::Release | Job::Build | Job::Drain => None,
    }
}

/// The rotation cursor of the tail block's hot flags (bits 8–15): the urgent kind code the last unit that had
/// a choice started with, 0 for none yet; a stored value above 8 is `Corrupt(Runtime, Range)`.
fn rotation<H: PageHost + ?Sized>(h: &H, tail: u64) -> Result<u64, Fault> {
    let c = (buffer::u64_of(h, tail, buffer::H_FLAGS)? >> 8) & 0xFF;
    if c > URGENT_CODES {
        return Err(rt(Corruption::Range, c));
    }
    Ok(c)
}

/// The job a unit starts with: with two or more urgent kinds waiting, the waiting kind whose code is the first
/// above the rotation cursor, wrapping from 8 to 1, and the cursor takes that code; otherwise the next job by
/// `next_job`, the cursor left as it is.
fn first_job<H: PageHost>(h: &mut H, tbl: u64) -> Result<Option<Job>, Fault> {
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let cursor = rotation(&*h, tail)?;
    let mut waiting: [Option<Job>; 9] = [None; 9];
    let mut n = 0u32;
    for j in JOBS {
        if let Some(c) = urgent_code(j) {
            if urgent_job(&*h, tbl, j)? {
                if let Some(w) = waiting.get_mut(usize::try_from(c).unwrap_or(0)) {
                    *w = Some(j);
                }
                n = n.wrapping_add(1);
            }
        }
    }
    if n < 2 {
        return next_job(&*h, tbl, &[false; 5]);
    }
    for k in 0..URGENT_CODES {
        let c = (cursor.wrapping_add(k) % URGENT_CODES).wrapping_add(1);
        if let Some(j) = waiting.get(usize::try_from(c).unwrap_or(0)).copied().flatten() {
            let flags = buffer::u64_of(&*h, tail, buffer::H_FLAGS)?;
            put_u64(h.get_mut(Region::Block(tail))?, buffer::H_FLAGS, (flags & !0xFF00) | (c << 8))?;
            return Ok(Some(j));
        }
    }
    Ok(None)
}

/// Runs one unit: the first job (`first_job`), then, while a further job's next piece fits the one bound
/// the unit shares across jobs, the next pending job, each job at most once. `meter` is the host's
/// instruction meter, which ends a unit early when it counts more than the unit's own estimate. Returns the
/// first job's kind (`Ran::Nothing` when no job is pending) and what the unit spent.
pub fn unit<H: PageHost>(
    h: &mut Tracked<'_, H>,
    tbl: u64,
    shape: &Shape,
    meter: &dyn Budget,
) -> Result<(Ran, Spent), Fault> {
    let pages = Pages::new(h.alloc_changes());
    let readings = Readings::new();
    let mut tracked = Tracked { h: &mut *h.h, pages: Some(&pages) };
    let h = &mut tracked;
    let clock = Clock::start(meter, &pages, &readings);
    let mut spent = Spent { work: UNIT_OVERHEAD, ..Spent::default() };
    let mut ran = Ran::Nothing;
    let mut done = [false; 5];
    push_recorded(h, tbl)?;
    let mut next = first_job(h, tbl)?;
    while let Some(j) = next {
        if let Some(d) = done.get_mut(index(j)) {
            *d = true;
        }
        if ran == Ran::Nothing || fits(j, &spent, &clock, tbl) {
            let r = run(h, tbl, shape, j, &mut spent, &clock, ran == Ran::Nothing)?;
            if ran == Ran::Nothing {
                ran = r;
            }
        }
        next = next_job(&*h, tbl, &done)?;
    }
    settle(h, tbl)?;
    spent.blocks = pages.count();
    Ok((ran, spent))
}

fn run<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    j: Job,
    spent: &mut Spent,
    clock: &Clock<'_>,
    lead: bool,
) -> Result<Ran, Fault> {
    match j {
        Job::Seal => {
            let st = seal::running(&*h, tbl, seal::SEAL)?;
            if st != 0 {
                seal::step(h, tbl, shape, st, spent, clock)?;
            } else {
                let desc = root::u64_at(&*h, tbl, root::FROZEN_HEAD)?;
                let st = seal::begin_seal(h, tbl, shape, desc)?;
                begun(&*h, st, spent)?;
            }
            Ok(Ran::Seal)
        }
        Job::Compaction => compaction_unit(h, tbl, shape, spent, clock),
        Job::Release => super::evolve::release_unit(h, tbl, shape, spent, clock),
        Job::Build => {
            super::indexes::build_unit(h, tbl, shape, spent, clock, lead)?;
            Ok(Ran::Build)
        }
        Job::Drain => {
            // Pieces, each with an allowance of its own, while the count leaves room for one more: the unit's
            // pages and blocks are counted across them.
            let mut work = spent.work.saturating_add(DRAIN_LOAD);
            let mut left = Units::DRAIN;
            let mut first = true;
            loop {
                // The radix blocks this piece may free: what the unit's pages leave beside the drain's own.
                let room = UNIT_BLOCKS_MAX.saturating_sub(clock.pages()).saturating_sub(drain_pages(clock, tbl));
                if !first {
                    let stop = clock.count(work).saturating_add(clock.ahead(DRAIN_PIECE_WORK)) > UNIT_WORK || room == 0;
                    #[cfg(feature = "meter-notes")]
                    clock.note(
                        if stop { 0 } else { clock.ahead(DRAIN_PIECE_WORK) },
                        if stop { 0 } else { drain_pages(clock, tbl).saturating_add(room) },
                    );
                    if stop {
                        break;
                    }
                }
                first = false;
                let mut u = Units {
                    pages: left.pages.min(DRAIN_PIECE_PAGES),
                    entries: left.entries.min(DRAIN_PIECE_ENTRIES),
                    blocks: left.blocks.min(room),
                };
                let given = u;
                let (p, changed) = drain::step(h, tbl, shape, &mut u)?;
                let (e, pg, b) = (
                    given.entries.saturating_sub(u.entries),
                    given.pages.saturating_sub(u.pages),
                    given.blocks.saturating_sub(u.blocks),
                );
                left.entries = left.entries.saturating_sub(e);
                left.pages = left.pages.saturating_sub(pg);
                left.blocks = left.blocks.saturating_sub(b);
                work = work
                    .saturating_add(DRAIN_RECORD_WORK)
                    // Each count is below its piece's allowance: the products stay small.
                    .saturating_add(e.min(1 << 32).wrapping_mul(DRAIN_ENTRY_WORK))
                    .saturating_add(pg.min(1 << 32).wrapping_mul(DRAIN_PAGE_WORK))
                    .saturating_add(b.min(1 << 32).wrapping_mul(DRAIN_BLOCK_WORK));
                if p == Progress::Done || (!changed && e == 0 && pg == 0 && b == 0) {
                    break;
                }
            }
            spent.work = work;
            Ok(Ran::Drain)
        }
    }
}

/// Adds a rewrite's snapshot to what the unit spent: the estimate its state block records and the live
/// bitmap it copied.
pub fn begun<H: PageHost + ?Sized>(h: &H, st: u64, spent: &mut Spent) -> Result<(), Fault> {
    spent.work = spent.work.saturating_add(seal::work_of(h, st)?);
    spent.bytes = spent.bytes.saturating_add(512);
    Ok(())
}

fn compaction_unit<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    spent: &mut Spent,
    clock: &Clock<'_>,
) -> Result<Ran, Fault> {
    let st = seal::running(&*h, tbl, seal::COMPACTION)?;
    if st != 0 {
        seal::step(h, tbl, shape, st, spent, clock)?;
        return Ok(Ran::Compaction);
    }
    let Some(g) = pop(h, tbl)? else { return Ok(Ran::Nothing) };
    spent.work = spent.work.saturating_add(POP_WORK);
    let pg = root::group(&*h, tbl, g)?;
    if pg == 0 || pg & DESC_BIT != 0 {
        // The group was dropped since it was queued.
        return Ok(Ran::Compaction);
    }
    let flags = buffer::u16_of(&*h, pg, page::P_FLAGS)?;
    put_u16(h.get_mut(Region::Block(pg))?, page::P_FLAGS, u16::try_from(flags).unwrap_or(0) & !page::PF_QUEUED)?;
    if buffer::u64_of(&*h, pg, page::P_REWRITE)? != 0 {
        // Another rewrite holds the group; its end checks the group again.
        return Ok(Ran::Compaction);
    }
    if buffer::u16_of(&*h, pg, page::P_LIVE)? == 0 {
        drop_group(h, tbl, g, pg)?;
        spent.work = spent.work.saturating_add(DROP_WORK);
        return Ok(Ran::Drop);
    }
    let st = seal::begin_rewrite(h, tbl, shape, pg, seal::COMPACTION)?;
    begun(&*h, st, spent)?;
    Ok(Ran::Compaction)
}

/// The maintenance state: pending while any job waits, urgent while an urgent one does; the root's state
/// byte is written only when it changes.
pub fn settle<H: PageHost>(h: &mut H, tbl: u64) -> Result<Wake, Fault> {
    let frozen = root::u32_at(&*h, tbl, root::FROZEN_WAITING)?;
    let draining = drain::pending(&*h, tbl)?;
    let seal_w = seal_work(&*h, tbl)?;
    let comp_w = compaction_work(&*h, tbl)?;
    let rel_w = release_work(&*h, tbl)?;
    let build_w = super::indexes::building(&*h, tbl)? != 0;
    let mut flags = root::u8_at(&*h, tbl, root::FLAGS)?;
    let old_flags = flags;
    if !comp_w {
        flags &= !root::FLAG_COMPACT_URGENT;
    }
    if flags != old_flags {
        root::set_u8(h, tbl, root::FLAGS, flags)?;
    }
    let mut s = 0u8;
    if frozen > 0 || seal_w || comp_w || rel_w || build_w || draining {
        s |= STATE_PENDING;
    }
    if (seal_w && seal_urgent(&*h, tbl)?) || (comp_w && compaction_urgent(&*h, tbl)?) {
        s |= STATE_URGENT;
    }
    let old = root::u8_at(&*h, tbl, root::STATE)?;
    if s != old {
        root::set_u8(h, tbl, root::STATE, s)?;
    }
    Ok(wake_from_state(s))
}
