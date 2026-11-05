//! Rewrites: the seal of a frozen group, the compaction of a sealed group and its column release. Each
//! builds the group's new image with one writer, in units of bounded work, from a snapshot of the group, and
//! keeps every cursor in its state block, so a rewrite resumes in any later call and the image's bytes do
//! not depend on where its units end.
//!
//! **Steps.** (1) The *snapshot*: a state block, a pending group page and a work page; the group's liveness
//! is copied, and the group is marked (a frozen group's descriptor flag and pending page, a sealed group's
//! page at 648). From then on nothing writes the snapshot's bytes: a delete clears the row's live bit on
//! the pending page, an update writes a post-snapshot patch there. (2) *Pass 1*, leaf by leaf over the
//! stored positions: the leaf's form and sizes, its plan in the work space.
//! (3) The *layout*, leaf by leaf, from the sizes alone. (4) The *image*, written in order (header, leaf
//! directory, rank map, cell area, inline data region, out-of-line area, segment table); a huge value's
//! reference is copied into the new long-value list. (8) The *flip*, O(1): the directory names the pending
//! page, which takes the image, the long-value list, the post-snapshot patches and the deletes made since the
//! snapshot. (9) The *last units*: the copied huge values' old referents are cleared, then the old
//! structures, the work page and the state block go to the drain.
//!
//! **State block** (`NTRW`, area 0, zero-filled when allocated), at the offsets the rewrite state block's
//! table gives: {magic u32 @0; `FORMAT_TABLE` u16 @4; kind u8 @6 (1 seal, 2 compaction, 3 column release);
//! step u8 @7; group u64 @8; pending group page u64 @16; the working log's first and last blocks u64 @24 and
//! @32 (the work page, below); the output image's record 80 B @48; the new long-value list u64 @128 and its
//! entries u32 @136; the latest drop's schema version at the snapshot u32 @140; cursors: leaf u32 @144,
//! position u32 @148, value byte offset u64 @160; H u32 @176; segments S u32 @180; inline data region u64
//! @184; out-of-line area u64 @192; segment table u64 @200; stored rows n u16 @212; units done u64 @216; then,
//! from @384, the working state: the control region @384–895 (this rewrite's other cursors and fields, `RW_*`
//! below), bytes 896–2,943 zero, and pass 1's accumulators of the leaf in work from @2,944.}
//!
//! **Work page** (area 0), which stands as the working log: a work space (`stats::space`) whose locator of
//! leaf o names leaf o's plan; the row layouts' index follows in it. The snapshot's liveness
//! bitmap lies in its bytes 128–639, which the group page layout gives the live bitmap.

use crate::consts::{magic, BLOCK, BYTE_UNIT, COMPACT_DEAD, FIXED_MAX, FORMAT_TABLE, HUGE_VALUE, LONG_VALUE, PAGE};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::image::Image;
use crate::mem::{
    fill, put, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, Owner, PageHost, Region,
};
use crate::stats::space::Space;

use super::buffer::{self, RowLoc, SLOT_DEAD};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::drain;
use super::group::{self, form, seg, GroupMeta, GroupView, GF_COMPACTED, GF_HUGE, GF_OOL, LF_OOL, LF_VALIDITY};
use super::jobs::{Clock, Spent};
use super::layout::{LeafInfo, RowHead, Shape, Ty};
use super::page;
use super::patch;
use super::root;
use super::row::{self, RowRef};
use super::versions;

mod walk;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

// ---------------------------------------------------------------- the state block

pub const RW_KIND: u64 = 6;
pub const RW_STEP: u64 = 7;
pub const RW_GROUP: u64 = 8;
pub const RW_PENDING: u64 = 16;
pub const RW_WORK: u64 = 24;
pub const RW_LOG_LAST: u64 = 32;
pub const RW_IMAGE: u64 = 48;
pub const RW_LV: u64 = 128;
pub const RW_LV_COUNT: u64 = 136;
pub const RW_GEN: u64 = 140;
pub const RW_LEAF: u64 = 144;
pub const RW_Q: u64 = 148;
pub const RW_VAT: u64 = 160;
pub const RW_H: u64 = 176;
pub const RW_S: u64 = 180;
pub const RW_INLINE: u64 = 184;
pub const RW_OOL: u64 = 192;
pub const RW_SEGS: u64 = 200;
pub const RW_N: u64 = 212;
pub const RW_UNITS: u64 = 216;
// ---- the control region, @384–895
/// The step's part in work.
pub const RW_SUB: u64 = 512;
/// An index a step walks: a block record.
pub const RW_POS: u64 = 520;
pub const RW_SOURCE: u64 = 528;
pub const RW_P: u64 = 536;
pub const RW_LIVE: u64 = 538;
pub const RW_LEAVES: u64 = 540;
pub const RW_LV_TAIL: u64 = 544;
pub const RW_X: u64 = 552;
pub const RW_V: u64 = 556;
pub const RW_C: u64 = 560;
pub const RW_RUN: u64 = 568;
pub const RW_OOLT: u64 = 576;
pub const RW_ACC: u64 = 584;
pub const RW_FLAGS: u64 = 666;
pub const RW_VERSION: u64 = 600;
pub const RW_SEG: u64 = 604;
pub const RW_SEG_Q: u64 = 608;
pub const RW_TOTAL: u64 = 616;
pub const RW_DIR: u64 = 624;
/// The estimated instructions and the bytes read or written of the last unit (status replies, tests).
pub const RW_WORK_LAST: u64 = 632;
pub const RW_BYTES_LAST: u64 = 640;
/// The snapshot's liveness bitmap, 512 bytes in the work page.
pub const RW_SNAP: u64 = super::page::P_BITMAP;
/// Pass 1's accumulators of the leaf in work.
pub const RW_ACCS: u64 = 2_944;

/// Rewrite kinds.
pub const SEAL: u8 = 1;
pub const COMPACTION: u8 = 2;
pub const RELEASE: u8 = 3;

/// Steps.
pub const STEP_SNAPSHOT: u8 = 1;
pub const STEP_PASS1: u8 = 2;
pub const STEP_LAYOUT: u8 = 3;
pub const STEP_WRITE: u8 = 4;
pub const STEP_FLIP: u8 = 8;
pub const STEP_LAST: u8 = 9;

/// Flags @214: bit 0 the image is compacted; bit 1 the liveness pass found a block record whose first
/// position is not its block's, or a block with no slot; bit 2 the snapshot found every block record's first
/// position its block's and every block holding a slot, so the records' first positions rise strictly and a
/// walk over the group's positions takes the blocks in record order; bit 3 the flip gave the group's extremes
/// block to the pending page, and the source still names it until the last units hand the source to the drain.
const RWF_COMPACTED: u16 = 1;
const RWF_RECORDS_APART: u16 = 2;
const RWF_RECORDS_IN_ORDER: u16 = 4;
const RWF_EXT: u16 = 8;

fn st_u64<H: PageHost + ?Sized>(h: &H, st: u64, off: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(st))?, off)
}
fn st_u32<H: PageHost + ?Sized>(h: &H, st: u64, off: u64) -> Result<u64, Fault> {
    Ok(u64::from(rd_u32(h.get(Region::Block(st))?, off)?))
}
fn st_u8<H: PageHost + ?Sized>(h: &H, st: u64, off: u64) -> Result<u8, Fault> {
    rd_u8(h.get(Region::Block(st))?, off)
}
fn set_u64<H: PageHost>(h: &mut H, st: u64, off: u64, v: u64) -> Result<(), Fault> {
    put_u64(h.get_mut(Region::Block(st), Owner(0))?, off, v)
}
fn set_u32<H: PageHost>(h: &mut H, st: u64, off: u64, v: u64) -> Result<(), Fault> {
    put_u32(h.get_mut(Region::Block(st), Owner(0))?, off, u32::try_from(v).map_err(|_| rt(Corruption::Range, v))?)
}

/// A rewrite's fixed facts, read once per unit.
#[derive(Clone, Copy, Debug)]
pub struct Rw {
    pub st: u64,
    pub kind: u8,
    pub step: u8,
    pub g: u64,
    pub pending: u64,
    pub work: u64,
    pub source: u64,
    pub p: u64,
    pub n: u64,
    pub compacted: bool,
    /// The frozen group's block records rise strictly, each first position its block's (a seal).
    pub ordered: bool,
    pub version: u32,
    pub dir: u64,
}

impl Rw {
    pub fn read<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<Rw, Fault> {
        let b = h.get(Region::Block(st))?;
        if rd_u32(b, 0)? != magic::NTRW {
            return Err(rt(Corruption::Header, st));
        }
        Ok(Rw {
            st,
            kind: rd_u8(b, RW_KIND)?,
            step: rd_u8(b, RW_STEP)?,
            g: rd_u64(b, RW_GROUP)?,
            pending: rd_u64(b, RW_PENDING)?,
            work: rd_u64(b, RW_WORK)?,
            source: rd_u64(b, RW_SOURCE)?,
            p: u64::from(rd_u16(b, RW_P)?),
            n: u64::from(rd_u16(b, RW_N)?),
            compacted: rd_u16(b, RW_FLAGS)? & RWF_COMPACTED != 0,
            ordered: rd_u16(b, RW_FLAGS)? & RWF_RECORDS_IN_ORDER != 0,
            version: rd_u32(b, RW_VERSION)?,
            dir: rd_u64(b, RW_DIR)?,
        })
    }

    /// Whether the rewrite still holds its snapshot: the flip has not happened.
    pub fn before_flip(&self) -> bool {
        self.step < STEP_LAST
    }

    /// Whether the rewrite holds a snapshot of its group now: from S to its flip. A compaction's or column
    /// release's S ends its snapshot step; a seal's, its liveness pass (its descriptor's flags say so).
    pub fn holds(&self) -> bool {
        self.before_flip() && (self.kind == SEAL || self.step > STEP_SNAPSHOT)
    }
}

/// Whether the snapshot held position q live (its bitmap in the work page `work`).
fn snap_live<H: PageHost + ?Sized>(h: &H, work: u64, q: u64) -> Result<bool, Fault> {
    if q >= 4_096 {
        return Ok(false);
    }
    Ok(rd_u8(h.get(Region::Block(work))?, RW_SNAP.wrapping_add(q / 8))? >> (q % 8) & 1 != 0)
}

fn snap_word<H: PageHost + ?Sized>(h: &H, work: u64, w: u64) -> Result<u64, Fault> {
    rd_u64(h.get(Region::Block(work))?, RW_SNAP.wrapping_add(w.wrapping_mul(8)))
}

/// The first stored position at or after q: every position of an image that is not compacted, else the
/// positions live at the snapshot. `None` past the last.
fn next_stored<H: PageHost + ?Sized>(h: &H, rw: &Rw, q: u64) -> Result<Option<u64>, Fault> {
    if q >= rw.p {
        return Ok(None);
    }
    if !rw.compacted {
        return Ok(Some(q));
    }
    next_live(h, rw, q)
}

/// The first position at or after q live at the snapshot, a word of its bitmap at a time. `None` past the
/// last.
fn next_live<H: PageHost + ?Sized>(h: &H, rw: &Rw, q: u64) -> Result<Option<u64>, Fault> {
    if q >= rw.p {
        return Ok(None);
    }
    let mut w = q / 64;
    let mut word = snap_word(h, rw.work, w)? & (u64::MAX << (q % 64));
    loop {
        if word != 0 {
            let r = w.wrapping_mul(64).wrapping_add(u64::from(word.trailing_zeros()));
            return Ok(if r < rw.p { Some(r) } else { None });
        }
        w = w.wrapping_add(1);
        if w.wrapping_mul(64) >= rw.p {
            return Ok(None);
        }
        word = snap_word(h, rw.work, w)?;
    }
}

// ---------------------------------------------------------------- the meter

/// What the unit has spent, from what other jobs spent before this one in the same unit: an estimate of its
/// instructions and the value and image bytes read or written; the unit's changed 4 KiB pages are counted
/// exactly by its `Clock`. The rewrite stops after the piece that reaches the first limit: `WORK_MAX`
/// instructions of the unit's count (the host's meter where it has one, else the estimate), `BYTE_UNIT`
/// bytes, or pages that leave no room for one more piece's (`Clock::pages_fit`). A piece costs at most
/// `PIECE_MAX` instructions, estimated and counted alike, unless it begins only when the unit has room for its
/// own estimate (a row layout's piece), so the unit stays within
/// `UNIT_BLOCKS_MAX` changed pages and `jobs::UNIT_WORK`. A piece of the in-place walk runs to the room left
/// below `WORK_MAX`, or `walk::PIECE_WORK` when less is left, by estimates above what its work counts: it
/// too ends within `PIECE_MAX` of `WORK_MAX`.
struct Meter<'c> {
    work: u64,
    bytes: u64,
    tbl: u64,
    clock: Clock<'c>,
    /// Whether this rewrite's share of the unit has done a piece of work yet: the count ends it only after
    /// one, so every unit makes progress whatever the jobs before it spent.
    progress: bool,
    /// The rewrite's state block and, from pass 1 on, its work space: where its row layouts lie.
    st: u64,
    ws: Option<Space>,
    /// The last row layout looked up, and the last leaf's offsets in it.
    lay: Option<versions::Entry>,
    lay_head: RowHead,
    lay_at: Option<(u64, Offsets)>,
    /// A schema version whose row layout a read found missing: the share ends, and the next unit builds it.
    pending: u32,
    /// The leaf whose values the share reads from a sealed source: its patch unit and its directory entry,
    /// found once per share; and the last default read, by leaf.
    src_leaf: Option<(u64, Option<patch::Unit>, Option<group::LeafDir>)>,
    dflt: Option<(u64, SVal)>,
    /// A piece found no room for itself: the share ends.
    ended: bool,
    /// The leaf the share's last walk over the leaf list found: the next walk from its ordinal finds it again.
    leaf_at: Option<LeafInfo>,
}

/// A leaf's offsets in a row image: its fixed cell, its item table index and its bitmap bit.
type Offsets = (u64, u64, u64);

/// The count at which a rewrite's share of a unit ends, and the most one piece of it costs: a value read and
/// what it feeds, a piece of a value's bytes or of a row layout, a leaf begun or finished.
const WORK_MAX: u64 = 72_000;
pub const PIECE_MAX: u64 = super::jobs::UNIT_WORK - WORK_MAX;
/// On a host with a meter, the estimate at which a piece of the in-place walk ends at the latest: the walk
/// reads the meter itself (`walk::CHECK_EVERY`) and ends when the unit is full.
const WALK_METERED: u64 = 1_000_000;
/// Estimated instructions of each piece, as wasmtime counts them (fuel), each at least the most a piece of
/// its kind was measured to cost (`scripts/unit_fuel.sh`), so that a unit cut by its estimates alone stays
/// within the bound: a value read (its slot or cell located, its row's item table read, a patch looked up), a
/// cell written, a leaf begun or finished, a slot of a seal's liveness pass, a leaf of a walk over the leaf
/// list and a piece of a row layout's build.
const COST_VALUE: u64 = 3_000;
const COST_CELL: u64 = 400;
const COST_LEAF: u64 = 3_200;
const COST_SLOT: u64 = 50;
const COST_SKIP: u64 = 300;
const COST_PATCH: u64 = 1_200;
const COST_OOL: u64 = 2_000;
const COST_SEALED: u64 = 2_500;
const COST_ACCUM: u64 = 1_500;
/// A leaf's pass 1 accumulators loaded and stored.
const COST_ACC: u64 = 5_000;
const COST_SPAN: u64 = 80;
const COST_LAYOUT: u64 = 14_000;
/// A row layout found in the work space and a leaf's offsets read from it.
const COST_FIND: u64 = 2_000;

impl<'c> Meter<'c> {
    fn new(spent: &Spent, clock: Clock<'c>, tbl: u64, st: u64) -> Meter<'c> {
        Meter {
            work: spent.work,
            bytes: spent.bytes,
            tbl,
            clock,
            progress: false,
            st,
            ws: None,
            lay: None,
            lay_head: RowHead::default(),
            lay_at: None,
            pending: 0,
            src_leaf: None,
            dflt: None,
            ended: false,
            leaf_at: None,
        }
    }
    /// Leaf `l`'s default, read once per share; `None` when reading it filled the unit (the share ends).
    fn default_of<H: PageHost + ?Sized>(&mut self, h: &H, shape: &Shape, l: &LeafInfo) -> Result<Option<SVal>, Fault> {
        let o = u64::from(l.ordinal);
        if let Some((k, v)) = self.dflt {
            if k == o {
                return Ok(Some(v));
            }
        }
        let worked = self.progress;
        let v = super::defaults::of(h, shape, l)?;
        self.lead(COST_LEAF);
        self.dflt = Some((o, v));
        Ok(if worked && self.full() { None } else { Some(v) })
    }
    /// The unit's instructions so far: the host's meter where it has one, else its own estimate.
    fn used(&self) -> u64 {
        self.clock.count(self.work)
    }
    /// Whether the share ends before its next piece: on a host with a meter, read before every piece, when the
    /// count and the look-ahead (the longest stretch the unit saw between two readings, at least
    /// `PIECE_METERED`, and the unit's end) pass `UNIT_WORK`; without one, at `WORK_MAX` of the scaled
    /// estimate.
    #[cfg_attr(feature = "meter-notes", track_caller)]
    fn full(&mut self) -> bool {
        let over = if self.clock.metered() { self.clock.over() } else { self.used() >= WORK_MAX };
        let full = self.ended
            || self.pending != 0
            || (self.progress && over)
            || self.bytes >= BYTE_UNIT
            || (self.progress && !self.clock.pages_fit(self.tbl));
        #[cfg(feature = "meter-notes")]
        if self.progress {
            let (ahead, pages) = if full { (0, 0) } else { (self.clock.ahead(0), self.clock.piece_pages(self.tbl)) };
            self.clock.note(ahead, pages);
        }
        full
    }
    /// The estimate at which a piece of the in-place walk ends. With a meter, `WALK_METERED`: the meter is
    /// read before every piece. Without one, the room left below `WORK_MAX`, at least `walk::PIECE_WORK`;
    /// the walk's estimates are above what its work counts, so a piece ends within the last position's work
    /// of `WORK_MAX`, or of `walk::PIECE_WORK` past where it began.
    fn walk_limit(&self) -> u64 {
        if self.clock.metered() {
            WALK_METERED
        } else {
            // The room is in scaled units; the walk's estimates are not.
            let room = WORK_MAX.saturating_sub(self.used());
            super::jobs::unscaled(room).max(walk::PIECE_WORK)
        }
    }
    /// What the in-place walk asks before a position once every `walk::CHECK_EVERY` of its estimate: with a
    /// meter, whether the unit's count and look-ahead pass `UNIT_WORK`; without one, never (its limit ends
    /// it).
    #[cfg_attr(feature = "meter-notes", track_caller)]
    fn walk_stop(&self) -> impl Fn() -> bool + 'c {
        let clock = self.clock;
        let metered = clock.metered();
        // The count past which the unit is full, fixed for the piece: the walk's stretches between two
        // readings stay below `STRETCH_METERED`, so the look-ahead does not change while it runs.
        let ahead = clock.ahead_walk();
        let limit = super::jobs::UNIT_WORK.saturating_sub(ahead);
        #[cfg(feature = "meter-notes")]
        let at = core::panic::Location::caller();
        move || {
            let stop = metered && clock.count(0) > limit;
            #[cfg(feature = "meter-notes")]
            note_at(&clock, if stop { 0 } else { ahead }, at);
            stop
        }
    }
    /// Whether work of `est` more fits the unit after the work done, its end left room: always for the
    /// share's first piece.
    #[cfg_attr(feature = "meter-notes", track_caller)]
    fn room(&self, est: u64) -> bool {
        !self.progress || self.clock.room(self.work, est, None)
    }
    /// Whether leaf `o`'s offsets in a row of schema version v are the ones kept.
    fn has_layout(&self, v: u32, o: u64) -> bool {
        matches!((&self.lay, self.lay_at), (Some(e), Some((k, _))) if e.version == v && k == o)
    }
    /// Leaf `o`'s offsets in a row of schema version v, the version's layout then in `lay_head`; `None` when
    /// the version has no row layout yet (it is then pending, and the share ends). The last leaf's are kept.
    #[inline(always)]
    fn layout<H: PageHost + ?Sized>(&mut self, h: &H, v: u32, o: u64) -> Result<Option<Offsets>, Fault> {
        if let (Some(e), Some((k, at))) = (&self.lay, self.lay_at) {
            if e.version == v && k == o {
                return Ok(Some(at));
            }
        }
        self.layout_of(h, v, o)
    }
    fn layout_of<H: PageHost + ?Sized>(&mut self, h: &H, v: u32, o: u64) -> Result<Option<Offsets>, Fault> {
        let e = match self.lay {
            Some(e) if e.version == v => e,
            _ => {
                self.lay_at = None;
                let ws = self.ws.ok_or(rt(Corruption::Link, self.st))?;
                match versions::find(h, self.st, &ws, v)? {
                    Some((_, e)) => {
                        self.lay = Some(e);
                        self.lay_head = e.head();
                        e
                    }
                    None => {
                        self.pending = v;
                        return Ok(None);
                    }
                }
            }
        };
        let ws = self.ws.ok_or(rt(Corruption::Link, self.st))?;
        let at = versions::offsets(h, &ws, &e, o)?;
        self.lay_at = Some((o, at));
        Ok(Some(at))
    }
    /// A piece of work done.
    fn spend(&mut self, work: u64) {
        self.work = self.work.saturating_add(work);
        self.progress = true;
    }
    /// Work that leads to a piece without being one: a leaf's state loaded, a value read whose use is still to
    /// come. It is counted, but it is no progress: the share may end after it only once a piece that it
    /// leads to has saved its effect, so every unit makes progress whatever was spent before it.
    fn lead(&mut self, work: u64) {
        self.work = self.work.saturating_add(work);
    }
    /// Value or image bytes read or written: three instructions a byte (read, staged, written out).
    fn bytes(&mut self, n: u64) {
        self.bytes = self.bytes.saturating_add(n);
        // n below 2^40 (bytes a unit reads): the product stays in u64, with no overflowing product's call.
        self.work = self.work.saturating_add(n.min(1 << 40).wrapping_mul(3));
        self.progress = true;
    }
    /// A piece that changed blocks (an old referent cleared, output written): the unit's
    /// `Clock` counts them.
    fn touch(&mut self) {
        self.progress = true;
    }
}

/// `Clock::note` from a closure, at the place the closure was made.
#[cfg(feature = "meter-notes")]
fn note_at(clock: &Clock<'_>, ahead: u64, at: &'static core::panic::Location<'static>) {
    clock.note_site(ahead, super::jobs::NO_PAGES, at);
}

// ---------------------------------------------------------------- the snapshot's values

/// A null's `Fixed` cell: zeros, as long as the longest (`FIXED_MAX` bytes).
static FIXED_ZEROS: [u8; 1_016] = [0; 1_016];
const _: () = assert!(FIXED_MAX == 1_016);

/// Where the snapshot's rows are read from.
#[derive(Clone, Copy)]
enum Source {
    Buffered { desc: u64 },
    Sealed { meta: GroupMeta, dir: u64 },
}

fn source<H: PageHost + ?Sized>(h: &H, rw: &Rw) -> Result<Source, Fault> {
    if rw.kind == SEAL {
        return Ok(Source::Buffered { desc: rw.source });
    }
    let gv = GroupView::open(h, rw.source)?;
    Ok(Source::Sealed { meta: gv.meta(), dir: rw.dir })
}

/// A long value's image bytes at `HUGE_VALUE` content bytes (`long::image_bytes`).
const HUGE_IMAGE: u64 = super::long::HEAD + HUGE_VALUE;
const _: () = assert!(HUGE_IMAGE % 8 == 0);

/// Whether a rewrite adopts the long value `img` names by its reference: only one past `HUGE_VALUE` content
/// bytes, told by the reference's image length without reading the value (a write after the flip may have
/// released it). A buffered row's long item of at most `HUGE_VALUE` bytes (a frozen row's update writes every
/// item longer than `LONG_VALUE` so) is placed by its length and copied like the same value inline, so the
/// image does not depend on the rows' history.
fn adopts(img: &Image) -> bool {
    img.length > HUGE_IMAGE
}

/// Leaf `l`'s value at position q of the snapshot (`Null` for a position dead there); `None` when the share
/// ends first: the row's schema version has no row layout yet (`m` then holds it pending, and the next unit
/// builds the layout), or the leaf's patch unit, directory entry or default, found once per share, filled
/// the unit.
fn value<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    src: &Source,
    l: &LeafInfo,
    m: &mut Meter,
    q: u64,
) -> Result<Option<SVal>, Fault> {
    match *src {
        Source::Buffered { desc } => {
            let Some((block, slot)) = buffer::locate_in(h, desc, q)? else { return Ok(Some(SVal::Null)) };
            let Some(loc) = buffer::slot_row(h, block, slot)? else { return Ok(Some(SVal::Null)) };
            let worked = m.progress;
            let long = matches!(loc, RowLoc::Long(_));
            if long {
                // A row image stored as a long value: each of its reads goes through its image.
                m.lead(COST_OOL);
            }
            let r = RowRef::new(h, loc)?;
            let v = r.version()?;
            // A long row image's head, read through its image, is a piece of its own, and so is a row layout
            // not in the meter's cache: the meter is read after either, and the share ends there when it had
            // done a piece before and the unit is full (the next unit reads the value again).
            if long && m.full() && worked {
                return Ok(None);
            }
            if !l.at(v) {
                return m.default_of(h, shape, l);
            }
            let found = m.has_layout(v, u64::from(l.ordinal));
            let Some((f, vi, b)) = m.layout(h, v, u64::from(l.ordinal))? else { return Ok(None) };
            if !found {
                m.lead(COST_FIND);
                if m.full() && worked {
                    return Ok(None);
                }
            }
            let head = m.lay_head;
            let s = row::stored_at(&head, &r, l, f, vi, b)?;
            // A long item's reference, read from the row again, is a piece of its own too.
            if matches!(s, row::Stored::Item(row::Item::Long { .. })) {
                m.lead(COST_OOL);
                if m.full() && worked {
                    return Ok(None);
                }
            }
            let s = cur::from_stored_in(&r, &head, l, &s, vi)?;
            // A long item a rewrite does not adopt (`adopts`) is read as bytes of its long value, so it is
            // sized and placed by its length as the same value inline would be.
            Ok(Some(match s {
                SVal::Huge { img, len, .. } if !adopts(&img) => SVal::B { at: cur::At::Long { img }, len },
                s => s,
            }))
        }
        Source::Sealed { meta, dir } => {
            let o = u64::from(l.ordinal);
            let gv = GroupView::from_meta(h, &meta);
            // A sealed image's cell is found through its leaf's segments, more than a row's offsets cost.
            m.lead(COST_SEALED);
            let (unit, ld) = match m.src_leaf {
                Some((k, u, d)) if k == o => (u, d),
                _ => {
                    // Once per leaf and share: its patch unit (a walk over the directory's blocks) and its
                    // directory entry.
                    let worked = m.progress;
                    let u = patch::unit(h, dir, o)?;
                    let d = gv.leaf_by_ordinal(o)?;
                    m.lead(COST_LEAF.wrapping_add(patch::dir_blocks(h, dir)?.wrapping_mul(COST_CELL)));
                    m.src_leaf = Some((o, u, d));
                    // A piece of its own: the share ends here when it filled the unit.
                    if worked && m.full() {
                        return Ok(None);
                    }
                    (u, d)
                }
            };
            if let Some(u) = unit {
                m.lead(COST_PATCH);
                if let Some(e) = patch::lookup_in(h, &u, q)? {
                    return Ok(Some(cur::from_entry(h, &e, l)?));
                }
            }
            let Some(ld) = ld else {
                return m.default_of(h, shape, l);
            };
            let Some(s) = gv.slot(q)? else { return Ok(Some(SVal::Null)) };
            let s = cur::from_cell(&gv, &ld, l, s, q)?;
            // A value out of line: its directory entry found first.
            if matches!(s, SVal::Huge { .. }) || s.len() > LONG_VALUE {
                m.lead(COST_OOL);
            }
            Ok(Some(s))
        }
    }
}

// ---------------------------------------------------------------- plans in the work space

const PLAN: u64 = 80;

#[derive(Clone, Copy, Debug, Default)]
struct Plan {
    form: u8,
    flags: u8,
    width: u64,
    written: bool,
    run: u64,
    ool_k: u64,
    huge: u64,
    ool_bytes: u64,
    seq_base: u64,
    seq_step: u64,
    first_seg: u64,
    segs: u64,
    cell_at: u64,
    run_rel: u64,
    ool_rel: u64,
}

impl Plan {
    fn encode(&self) -> Result<[u8; 80], Fault> {
        let mut b = [0u8; 80];
        let r32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
        let r16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
        put_u8(&mut b, 0, self.form)?;
        put_u8(&mut b, 1, self.flags)?;
        put_u16(&mut b, 2, r16(self.width)?)?;
        put_u8(&mut b, 4, u8::from(self.written))?;
        put_u64(&mut b, 8, self.run)?;
        put_u32(&mut b, 16, r32(self.ool_k)?)?;
        put_u32(&mut b, 20, r32(self.huge)?)?;
        put_u64(&mut b, 24, self.ool_bytes)?;
        put_u64(&mut b, 32, self.seq_base)?;
        put_u64(&mut b, 40, self.seq_step)?;
        put_u32(&mut b, 48, r32(self.first_seg)?)?;
        put_u32(&mut b, 52, r32(self.segs)?)?;
        put_u64(&mut b, 56, self.cell_at)?;
        put_u64(&mut b, 64, self.run_rel)?;
        put_u64(&mut b, 72, self.ool_rel)?;
        Ok(b)
    }

    fn from_bytes(b: &[u8; 80]) -> Result<Plan, Fault> {
        Ok(Plan {
            form: rd_u8(b, 0)?,
            flags: rd_u8(b, 1)?,
            width: u64::from(rd_u16(b, 2)?),
            written: rd_u8(b, 4)? != 0,
            run: rd_u64(b, 8)?,
            ool_k: u64::from(rd_u32(b, 16)?),
            huge: u64::from(rd_u32(b, 20)?),
            ool_bytes: rd_u64(b, 24)?,
            seq_base: rd_u64(b, 32)?,
            seq_step: rd_u64(b, 40)?,
            first_seg: u64::from(rd_u32(b, 48)?),
            segs: u64::from(rd_u32(b, 52)?),
            cell_at: rd_u64(b, 56)?,
            run_rel: rd_u64(b, 64)?,
            ool_rel: rd_u64(b, 72)?,
        })
    }

    fn validity(&self) -> bool {
        self.flags & LF_VALIDITY != 0
    }
}

fn work_space<H: PageHost + ?Sized>(h: &H, tbl: u64, rw: &Rw) -> Result<Space, Fault> {
    Space::read_raw(h, tbl, rw.work, rw.g)
}

fn load_plan<H: PageHost + ?Sized>(h: &H, ws: &Space, o: u64) -> Result<Option<Plan>, Fault> {
    let at = ws.locator(h, o)?;
    if at == 0 {
        return Ok(None);
    }
    let mut b = [0u8; 80];
    ws.read_bytes(h, at, &mut b)?;
    Ok(Some(Plan::from_bytes(&b)?))
}

fn store_plan<H: PageHost>(h: &mut H, ws: &mut Space, o: u64, pl: &Plan) -> Result<(), Fault> {
    let mut at = ws.locator(&*h, o)?;
    if at == 0 {
        at = ws.append(h, PLAN)?;
        ws.set_locator(h, o, at)?;
    }
    ws.write_bytes(h, at, &pl.encode()?)
}

// ---------------------------------------------------------------- pass 1 accumulators

/// Pass 1's accumulators of the leaf in work: what its form and sizes follow from. An integer leaf's largest
/// value (unsigned) and its smallest and largest (signed), and its `Seq` test; the sizes of a variable leaf's
/// values.
#[derive(Clone, Copy, Debug, Default)]
struct Acc {
    flags: u64,
    umax: u64,
    imin: i64,
    imax: i64,
    seq_q0: u64,
    seq_v0: i128,
    seq_step: i128,
    sizes: [u64; 8],
}

const A_STORED_NULL: u64 = 1;
const A_ANY_BIG: u64 = 2;
const A_SEQ_OK: u64 = 4;
const A_SEQ_FIRST: u64 = 8;
const A_SEQ_STEP: u64 = 16;

const ACC_BYTES: usize = 136;

impl Acc {
    fn load<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<Acc, Fault> {
        let raw = crate::mem::sub(h.get(Region::Block(st))?, RW_ACCS, ACC_BYTES as u64)?;
        let b: &[u8; ACC_BYTES] = raw.try_into().map_err(|_| rt(Corruption::Bounds, RW_ACCS))?;
        // Fixed offsets in a fixed array: each read is one load.
        let u = |at: usize| acc_u64(b, at);
        let w = |at: usize| u128::from(acc_u64(b, at)) | (u128::from(acc_u64(b, at.wrapping_add(8))) << 64);
        let mut sizes = [0u64; 8];
        for (i, s) in sizes.iter_mut().enumerate() {
            *s = u(72usize.wrapping_add(i.wrapping_mul(8)));
        }
        Ok(Acc {
            flags: u(0),
            umax: u(8),
            imin: u(16) as i64,
            imax: u(24) as i64,
            seq_q0: u(32),
            seq_v0: w(40) as i128,
            seq_step: w(56) as i128,
            sizes,
        })
    }

    fn store<H: PageHost>(&self, h: &mut H, st: u64) -> Result<(), Fault> {
        let raw = crate::mem::sub_mut(h.get_mut(Region::Block(st), Owner(0))?, RW_ACCS, ACC_BYTES as u64)?;
        let b: &mut [u8; ACC_BYTES] = raw.try_into().map_err(|_| rt(Corruption::Bounds, RW_ACCS))?;
        // The low and high halves of the 128-bit values: the casts keep each half.
        #[allow(clippy::cast_possible_truncation)]
        let w = |b: &mut [u8; ACC_BYTES], at: usize, x: i128| {
            set_acc_u64(b, at, x as u64);
            set_acc_u64(b, at.wrapping_add(8), (x >> 64) as u64);
        };
        set_acc_u64(b, 0, self.flags);
        set_acc_u64(b, 8, self.umax);
        set_acc_u64(b, 16, self.imin as u64);
        set_acc_u64(b, 24, self.imax as u64);
        set_acc_u64(b, 32, self.seq_q0);
        w(b, 40, self.seq_v0);
        w(b, 56, self.seq_step);
        for (i, s) in self.sizes.iter().enumerate() {
            set_acc_u64(b, 72usize.wrapping_add(i.wrapping_mul(8)), *s);
        }
        Ok(())
    }

    fn has(&self, f: u64) -> bool {
        self.flags & f != 0
    }
}

/// The u64 at `at` of the accumulators' bytes (`at` + 8 ≤ `ACC_BYTES` at every call: 0 past them).
#[inline(always)]
fn acc_u64(b: &[u8; ACC_BYTES], at: usize) -> u64 {
    b.get(at..).and_then(|s| s.first_chunk::<8>()).map_or(0, |c| u64::from_le_bytes(*c))
}

#[inline(always)]
fn set_acc_u64(b: &mut [u8; ACC_BYTES], at: usize, x: u64) {
    if let Some(d) = b.get_mut(at..).and_then(|s| s.first_chunk_mut::<8>()) {
        *d = x.to_le_bytes();
    }
}

/// Sizes: 0 run, 1 run (Leb), 2 out-of-line values, 3 (Leb), 4 out-of-line bytes, 5 (Leb), 6 huge, 7 (Leb).
fn size(acc: &mut Acc, len: u64, long: bool, leb: bool) {
    let k = usize::from(leb);
    let s = &mut acc.sizes;
    let at = |i: usize| i.wrapping_add(k);
    if long {
        if let Some(x) = s.get_mut(at(2)) {
            *x = x.wrapping_add(1);
        }
        if let Some(x) = s.get_mut(at(4)) {
            *x = x.wrapping_add(8);
        }
        if let Some(x) = s.get_mut(at(6)) {
            *x = x.wrapping_add(1);
        }
    } else if len > LONG_VALUE {
        if let Some(x) = s.get_mut(at(2)) {
            *x = x.wrapping_add(1);
        }
        if let Some(x) = s.get_mut(at(4)) {
            *x = x.wrapping_add(len);
        }
    } else if let Some(x) = s.get_mut(at(0)) {
        *x = x.wrapping_add(len);
    }
}

fn seq_add(acc: &mut Acc, q: u64, v: i128) {
    if !acc.has(A_SEQ_OK) {
        return;
    }
    let qi = i128::from(q);
    if !acc.has(A_SEQ_FIRST) {
        acc.flags |= A_SEQ_FIRST;
        acc.seq_q0 = q;
        acc.seq_v0 = v;
        return;
    }
    let q0 = i128::from(acc.seq_q0);
    if !acc.has(A_SEQ_STEP) {
        let dq = qi.wrapping_sub(q0);
        let dv = v.wrapping_sub(acc.seq_v0);
        if dq <= 0 || dv.checked_rem(dq) != Some(0) {
            acc.flags &= !A_SEQ_OK;
        } else if let Some(s) = dv.checked_div(dq) {
            acc.seq_step = s;
            acc.flags |= A_SEQ_STEP;
        } else {
            acc.flags &= !A_SEQ_OK;
        }
        return;
    }
    let want = acc.seq_step.checked_mul(qi.wrapping_sub(q0)).and_then(|x| x.checked_add(acc.seq_v0));
    if want != Some(v) {
        acc.flags &= !A_SEQ_OK;
    }
}

/// (base, step) when every live value lies on base + step · q, with base in [lo, hi] and step an i64.
fn seq_params(acc: &Acc, lo: i128, hi: i128) -> Option<(i128, i128)> {
    if !acc.has(A_SEQ_OK) || !acc.has(A_SEQ_FIRST) {
        return None;
    }
    let step = if acc.has(A_SEQ_STEP) { acc.seq_step } else { 0 };
    let base = acc.seq_v0.checked_sub(step.checked_mul(i128::from(acc.seq_q0))?)?;
    if base < lo || base > hi || step < i128::from(i64::MIN) || step > i128::from(i64::MAX) {
        return None;
    }
    Some((base, step))
}

/// The range of an integer leaf's values in 64-bit forms.
fn int_range(t: Ty) -> (i128, i128) {
    match t {
        Ty::U(w) => (0, 1i128.wrapping_shl(u32::from(w).wrapping_mul(8)).wrapping_sub(1)),
        Ty::I(w) => {
            let top = 1i128.wrapping_shl(u32::from(w).wrapping_mul(8).wrapping_sub(1));
            (top.wrapping_neg(), top.wrapping_sub(1))
        }
        Ty::Nat => (0, i128::from(u64::MAX)),
        _ => (i128::from(i64::MIN), i128::from(i64::MAX)),
    }
}

fn u_form(max: u64) -> u8 {
    if max <= 0xFF {
        form::U8
    } else if max <= 0xFFFF {
        form::U16
    } else if max <= u64::from(u32::MAX) {
        form::U32
    } else {
        form::U64
    }
}

fn i_form(min: i64, max: i64) -> u8 {
    if min >= i64::from(i8::MIN) && max <= i64::from(i8::MAX) {
        form::I8
    } else if min >= i64::from(i16::MIN) && max <= i64::from(i16::MAX) {
        form::I16
    } else if min >= i64::from(i32::MIN) && max <= i64::from(i32::MAX) {
        form::I32
    } else {
        form::I64
    }
}

// ---------------------------------------------------------------- the output

/// The image being written in order, staged 4 KiB at a time; the stage is flushed at every unit's end.
struct Out {
    img: Image,
    tbl: u64,
    stage: [u8; 4_096],
    staged: usize,
}

impl Out {
    fn pos(&self) -> u64 {
        self.img.length.wrapping_add(self.staged as u64)
    }

    fn put<H: PageHost>(&mut self, h: &mut H, m: &mut Meter, b: &[u8]) -> Result<(), Fault> {
        m.touch();
        m.bytes(b.len() as u64);
        let mut rest = b;
        while !rest.is_empty() {
            let n = self.stage.len().saturating_sub(self.staged).min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, 0))?;
            let end = self.staged.saturating_add(n);
            crate::mem::copy_bytes(self.stage.get_mut(self.staged..end).ok_or(rt(Corruption::Range, 0))?, now);
            self.staged = end;
            rest = later;
            if self.staged == self.stage.len() {
                self.flush(h)?;
            }
        }
        Ok(())
    }

    fn flush<H: PageHost>(&mut self, h: &mut H) -> Result<(), Fault> {
        if self.staged > 0 {
            let s = self.stage.get(..self.staged).ok_or(rt(Corruption::Range, 0))?;
            self.img.append(h, self.tbl, s)?;
            self.staged = 0;
        }
        Ok(())
    }

    /// Zeros up to image offset `off`; at most what the meter allows per call is not enforced here: padding
    /// is bounded by one image block per part.
    fn pad_to<H: PageHost>(&mut self, h: &mut H, m: &mut Meter, off: u64) -> Result<(), Fault> {
        let at = self.pos();
        if off < at {
            return Err(rt(Corruption::Layout, off));
        }
        let mut left = off.wrapping_sub(at);
        let zeros = [0u8; 512];
        while left > 0 {
            let n = left.min(512);
            self.put(h, m, zeros.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))?;
            left = left.wrapping_sub(n);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- begin

/// Whether a rewrite of a group of p positions, `live` of them live at its snapshot, writes the compacted
/// form: the one rule of every rewrite (seal, compaction, column release), so the same live rows give the same
/// bytes whatever rewrite writes them. Dead positions reaching `COMPACT_DEAD` are left out with a rank map;
/// fewer are written as nulls.
pub fn compacted_form(p: u64, live: u64) -> bool {
    p.saturating_sub(live) >= COMPACT_DEAD
}

/// The root slot of a rewrite kind (job slots 4–6).
pub fn slot_of(kind: u8) -> u64 {
    root::JOB_SLOTS.wrapping_add(32u64.wrapping_mul(3u64.wrapping_add(u64::from(kind))))
}

/// The job slot kind code of a rewrite kind, appended after the column release's 1: 5 a seal, 6 a compaction,
/// 7 a column release's rewrite.
pub fn slot_code(kind: u8) -> u8 {
    kind.wrapping_add(4)
}

/// The rewrite of a kind in progress, if any: its state block. A slot naming a state block under another
/// kind code is `Corrupt(Runtime, Range)`.
pub fn running<H: PageHost + ?Sized>(h: &H, tbl: u64, kind: u8) -> Result<u64, Fault> {
    let at = slot_of(kind);
    let st = root::u64_at(h, tbl, at.wrapping_add(8))?;
    if st != 0 && root::u8_at(h, tbl, at)? != slot_code(kind) {
        return Err(rt(Corruption::Range, at));
    }
    Ok(st)
}

fn set_running<H: PageHost>(h: &mut H, tbl: u64, kind: u8, st: u64) -> Result<(), Fault> {
    let at = slot_of(kind);
    let b = h.get_mut(Region::Block(tbl), Owner(0))?;
    fill(b, at, 32, 0)?;
    if st != 0 {
        put_u8(b, at, slot_code(kind))?;
        put_u8(b, at.wrapping_add(1), 1)?;
        put_u64(b, at.wrapping_add(8), st)?;
    }
    Ok(())
}

/// The latest drop: the highest schema version that dropped a leaf, 0 when none did.
pub fn latest_drop<H: PageHost + ?Sized>(_h: &H, shape: &Shape) -> Result<u32, Fault> {
    Ok(shape.dropped)
}

/// A new state block, pending group page and work page for a rewrite of group g: the snapshot's first unit.
#[allow(clippy::too_many_arguments)]
fn begin<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    kind: u8,
    g: u64,
    source: u64,
    bitmap: &[u8; 512],
    p: u64,
    live: u64,
    compacted: bool,
    dir: u64,
) -> Result<u64, Fault> {
    let st = crate::area::alloc_zeroed(h, tbl, 0)?;
    let pg = crate::area::alloc_block(h, tbl, 0)?;
    let work = crate::area::alloc_block(h, tbl, 0)?;
    let n = if compacted { live } else { p };
    let gen = latest_drop(&*h, shape)?;
    {
        let b = h.get_mut(Region::Block(st), Owner(0))?;
        put_u32(b, 0, magic::NTRW)?;
        put_u16(b, 4, FORMAT_TABLE)?;
        put_u8(b, RW_KIND, kind)?;
        put_u8(b, RW_STEP, STEP_SNAPSHOT)?;
        put_u64(b, RW_GROUP, g)?;
        put_u64(b, RW_PENDING, pg)?;
        put_u64(b, RW_WORK, work)?;
        Image::EMPTY.put_record(b, RW_IMAGE)?;
        put_u32(b, RW_GEN, gen)?;
        put_u64(b, RW_SOURCE, source)?;
        put_u16(b, RW_P, u16::try_from(p).map_err(|_| rt(Corruption::Range, p))?)?;
        put_u16(b, RW_LIVE, u16::try_from(live).map_err(|_| rt(Corruption::Range, live))?)?;
        put_u16(b, RW_N, u16::try_from(n).map_err(|_| rt(Corruption::Range, n))?)?;
        put_u16(b, RW_FLAGS, if compacted { RWF_COMPACTED } else { 0 })?;
        put_u32(b, RW_VERSION, shape.version)?;
        put_u64(b, RW_DIR, dir)?;
        put_u64(b, RW_LOG_LAST, work)?;
        // The snapshot's own estimated work: a seal's pass over the group's slots, or a copy of the live
        // bitmap, and the blocks it makes.
        let work = snapshot_work().wrapping_add(512);
        put_u64(b, RW_WORK_LAST, work)?;
        put_u64(b, RW_BYTES_LAST, 512)?;
    }
    {
        let b = h.get_mut(Region::Block(pg), Owner(0))?;
        fill(b, 0, page::P_FIELDS, 0)?;
        put_u32(b, 0, magic::NTGP)?;
        put_u16(b, 4, FORMAT_TABLE)?;
        put_u64(b, page::P_GROUP, g)?;
        put_u16(b, page::P_POSITIONS, u16::try_from(p).map_err(|_| rt(Corruption::Range, p))?)?;
        put_u16(b, page::P_LIVE, u16::try_from(live).map_err(|_| rt(Corruption::Range, live))?)?;
        put(b, page::P_BITMAP, bitmap)?;
        put_u64(b, page::P_REWRITE, st)?;
    }
    Space::begin(h, tbl, work, g, shape.leaves)?;
    put(h.get_mut(Region::Block(work), Owner(0))?, RW_SNAP, bitmap)?;
    set_running(h, tbl, kind, st)?;
    Ok(st)
}

// The snapshot reads slot entries of 8 bytes, flags at 4.
const _: () = assert!(buffer::SLOT == 8);

/// Starts the seal of the frozen group whose descriptor is `desc`: the snapshot. Returns the state block.
pub fn begin_seal<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, desc: u64) -> Result<u64, Fault> {
    let g = buffer::u64_of(&*h, desc, buffer::D_GROUP)?;
    {
        let d = h.get(Region::Block(desc))?;
        if rd_u32(d, 0)? != magic::NTBD
            || rd_u16(d, buffer::D_FLAGS)? & buffer::FROZEN == 0
            || rd_u16(d, 4)? > FORMAT_TABLE
        {
            return Err(rt(Corruption::Header, desc));
        }
    }
    let p = buffer::u16_of(&*h, desc, buffer::D_POSITIONS)?;
    // The liveness is read in the snapshot step's units; the snapshot is taken when it is complete.
    let st = begin(h, tbl, shape, SEAL, g, desc, &[0u8; 512], p, 0, false, 0)?;
    let pg = st_u64(&*h, st, RW_PENDING)?;
    let b = h.get_mut(Region::Block(desc), Owner(0))?;
    let flags = rd_u16(b, buffer::D_FLAGS)?;
    put_u16(b, buffer::D_FLAGS, flags | buffer::LIVENESS)?;
    put_u64(b, buffer::D_PENDING, pg)?;
    Ok(st)
}

/// A seal's liveness pass, block record by block record from the cursor (`RW_POS` the record, `RW_Q` the
/// positions before it): each live slot's bit into the pending page's bitmap. At its end the snapshot is
/// taken: the bitmap is copied to the state block, the live count and the compacted form decided, and the
/// descriptor marked. `true` at the snapshot.
fn liveness<H: PageHost>(h: &mut H, rw: &Rw, m: &mut Meter) -> Result<bool, Fault> {
    let desc = rw.source;
    let blocks = buffer::u16_of(&*h, desc, buffer::D_BLOCKS)?.min(crate::consts::BUFFER_BLOCKS_MAX);
    let mut k = st_u32(&*h, rw.st, RW_POS)?;
    let mut slots = st_u32(&*h, rw.st, RW_Q)?;
    while k < blocks {
        if m.full() {
            let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
            put_u32(b, RW_POS, u32::try_from(k).map_err(|_| rt(Corruption::Range, k))?)?;
            put_u32(b, RW_Q, u32::try_from(slots).map_err(|_| rt(Corruption::Range, slots))?)?;
            return Ok(false);
        }
        let blk_ref = buffer::u64_of(&*h, desc, buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)))?;
        let mut words = [0u64; 66];
        let mut apart = false;
        let first_word = slots / 64;
        let n = {
            let blk = h.get(Region::Block(blk_ref))?;
            if rd_u32(blk, 0)? != magic::NTBF || rd_u64(blk, buffer::B_GROUP)? != rw.g {
                return Err(rt(Corruption::Header, blk_ref));
            }
            if u64::from(rd_u16(blk, buffer::B_FIRST)?) != slots {
                return Err(rt(Corruption::Order, blk_ref));
            }
            // The slot array grows down from the block's end: slot 0 is its last entry.
            let n = u64::from(rd_u16(blk, buffer::B_SLOTS)?);
            let rec_first = u64::from(rd_u16(
                h.get(Region::Block(desc))?,
                buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16)).wrapping_add(8),
            )?);
            if rec_first != slots || n == 0 {
                apart = true;
            }
            let array =
                crate::mem::sub(blk, BLOCK.wrapping_sub(n.wrapping_mul(buffer::SLOT)), n.wrapping_mul(buffer::SLOT))?;
            let mut q = slots;
            for e in array.chunks_exact(8).rev() {
                if e.get(4).copied().unwrap_or(SLOT_DEAD) & SLOT_DEAD == 0 {
                    if let Some(w) =
                        words.get_mut(usize::try_from((q / 64).wrapping_sub(first_word)).unwrap_or(usize::MAX))
                    {
                        *w |= 1 << (q % 64);
                    }
                }
                q = q.wrapping_add(1);
            }
            n
        };
        // A block holds fewer than 4,096 / 8 slots, so its bits span at most 65 words.
        let last_word = slots.wrapping_add(n).div_ceil(64);
        if apart {
            let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
            let f = rd_u16(b, RW_FLAGS)?;
            put_u16(b, RW_FLAGS, f | RWF_RECORDS_APART)?;
        }
        let pb = h.get_mut(Region::Block(rw.pending), Owner(0))?;
        for (i, w) in words.iter().enumerate().take(usize::try_from(last_word.saturating_sub(first_word)).unwrap_or(0))
        {
            let at = page::P_BITMAP.wrapping_add(first_word.wrapping_add(i as u64).wrapping_mul(8));
            let old = rd_u64(pb, at)?;
            put_u64(pb, at, old | *w)?;
        }
        m.spend(n.wrapping_mul(COST_SLOT).wrapping_add(COST_LEAF));
        slots = slots.wrapping_add(n);
        k = k.wrapping_add(1);
    }
    if slots != rw.p {
        return Err(rt(Corruption::Count, desc));
    }
    // The snapshot.
    let mut bitmap = [0u8; 512];
    crate::mem::copy_bytes(&mut bitmap, crate::mem::sub(h.get(Region::Block(rw.pending))?, page::P_BITMAP, 512)?);
    let live = bitmap.iter().map(|b| u64::from(b.count_ones())).sum::<u64>();
    let compacted = compacted_form(rw.p, live);
    let n = if compacted { live } else { rw.p };
    put(h.get_mut(Region::Block(rw.work), Owner(0))?, RW_SNAP, &bitmap)?;
    {
        let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
        put_u16(b, RW_LIVE, u16::try_from(live).map_err(|_| rt(Corruption::Range, live))?)?;
        put_u16(b, RW_N, u16::try_from(n).map_err(|_| rt(Corruption::Range, n))?)?;
        let in_order = if rd_u16(b, RW_FLAGS)? & RWF_RECORDS_APART == 0 { RWF_RECORDS_IN_ORDER } else { 0 };
        put_u16(b, RW_FLAGS, if compacted { RWF_COMPACTED | in_order } else { in_order })?;
        put_u32(b, RW_POS, 0)?;
        put_u32(b, RW_Q, 0)?;
    }
    put_u16(
        h.get_mut(Region::Block(rw.pending), Owner(0))?,
        page::P_LIVE,
        u16::try_from(live).map_err(|_| rt(Corruption::Range, live))?,
    )?;
    let b = h.get_mut(Region::Block(desc), Owner(0))?;
    let flags = rd_u16(b, buffer::D_FLAGS)?;
    put_u16(b, buffer::D_FLAGS, (flags & !buffer::LIVENESS) | buffer::SNAPSHOT)?;
    m.spend(COST_LEAF);
    Ok(true)
}

/// Clears position `pos`'s bit in the pending page of a seal reading frozen group `desc`'s liveness, when one
/// is: a delete made before the seal's snapshot.
pub fn liveness_delete<H: PageHost>(h: &mut H, desc: u64, pos: u64) -> Result<(), Fault> {
    let (flags, pg) = {
        let d = h.get(Region::Block(desc))?;
        (rd_u16(d, buffer::D_FLAGS)?, rd_u64(d, buffer::D_PENDING)?)
    };
    if flags & buffer::LIVENESS == 0 {
        return Ok(());
    }
    let b = h.get_mut(Region::Block(pg), Owner(0))?;
    let at = page::P_BITMAP.wrapping_add(pos / 8);
    let x = rd_u8(b, at)?;
    put_u8(b, at, x & !(1 << (pos % 8)))
}

/// Starts the compaction (or the column release) of the sealed group whose page is `page`: its state block,
/// pending page and work page, the group page naming the state block so no other rewrite takes the group. The
/// snapshot is taken once the work page's locators are zeroed (`snapshot_rest`): until then the rewrite holds
/// no snapshot, and writes to the group are ordinary writes.
pub fn begin_rewrite<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, page: u64, kind: u8) -> Result<u64, Fault> {
    let (g, p) = {
        let b = h.get(Region::Block(page))?;
        if rd_u32(b, 0)? != magic::NTGP {
            return Err(rt(Corruption::Header, page));
        }
        (rd_u64(b, page::P_GROUP)?, u64::from(rd_u16(b, page::P_POSITIONS)?))
    };
    let st = begin(h, tbl, shape, kind, g, page, &[0u8; 512], p, 0, false, 0)?;
    put_u64(h.get_mut(Region::Block(page), Owner(0))?, page::P_REWRITE, st)?;
    Ok(st)
}

/// A compaction's or column release's snapshot S: the group page's liveness copied to the state block and the
/// pending page, the live count and the compacted form decided, and the patch directory the snapshot reads.
fn take_snapshot<H: PageHost>(h: &mut H, rw: &Rw) -> Result<(), Fault> {
    let (live, bitmap, dir) = {
        let b = h.get(Region::Block(rw.source))?;
        let mut bm = [0u8; 512];
        crate::mem::copy_bytes(&mut bm, crate::mem::sub(b, page::P_BITMAP, 512)?);
        (u64::from(rd_u16(b, page::P_LIVE)?), bm, rd_u64(b, page::P_PATCH_DIR)?)
    };
    let compacted = compacted_form(rw.p, live);
    let n = if compacted { live } else { rw.p };
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    put(h.get_mut(Region::Block(rw.work), Owner(0))?, RW_SNAP, &bitmap)?;
    {
        let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
        put_u16(b, RW_LIVE, w16(live)?)?;
        put_u16(b, RW_N, w16(n)?)?;
        let f = rd_u16(b, RW_FLAGS)?;
        put_u16(b, RW_FLAGS, if compacted { f | RWF_COMPACTED } else { f & !RWF_COMPACTED })?;
        put_u64(b, RW_DIR, dir)?;
    }
    let b = h.get_mut(Region::Block(rw.pending), Owner(0))?;
    put(b, page::P_BITMAP, &bitmap)?;
    put_u16(b, page::P_LIVE, w16(live)?)
}

// ---------------------------------------------------------------- one unit

/// Runs the rewrite whose state block is `st` within what is left of the unit `spent` describes, adding what
/// it spends. `true` when the rewrite is done (its state block gone to the drain).
pub fn step<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    st: u64,
    spent: &mut Spent,
    clock: &Clock<'_>,
) -> Result<bool, Fault> {
    let rw = Rw::read(&*h, st)?;
    let mut m = Meter::new(spent, *clock, tbl, st);
    let units = st_u64(&*h, st, RW_UNITS)?;
    set_u64(h, st, RW_UNITS, units.wrapping_add(1))?;
    if rw.step >= STEP_PASS1 {
        m.ws = Some(work_space(&*h, tbl, &rw)?);
    }
    // A row layout a read found missing is built first.
    let ready = rw.kind != SEAL || rw.step < STEP_PASS1 || layouts(h, shape, &rw, &mut m, units)?;
    let r = if !ready {
        false
    } else {
        match rw.step {
            STEP_SNAPSHOT => snapshot_rest(h, tbl, shape, &rw, &mut m).map(|()| false),
            STEP_PASS1 => pass1(h, tbl, shape, &rw, &mut m).map(|()| false),
            STEP_LAYOUT => layout(h, tbl, shape, &rw, &mut m).map(|()| false),
            STEP_WRITE => write(h, tbl, shape, &rw, &mut m).map(|()| false),
            STEP_FLIP => flip(h, tbl, shape, &rw, &mut m).map(|()| false),
            STEP_LAST => last(h, tbl, shape, &rw, &mut m),
            _ => Err(rt(Corruption::Header, st)),
        }?
    };
    if running(&*h, tbl, rw.kind)? != st {
        // Started again: the state block went to the drain.
        *spent = Spent { work: m.work, bytes: m.bytes, blocks: clock.pages() };
        return Ok(true);
    }
    if m.pending != 0 {
        versions::need(h, st, m.pending)?;
    }
    let b = h.get_mut(Region::Block(st), Owner(0))?;
    // The share's own estimate, unscaled: the unit's count reads the meter, or scales the estimates.
    put_u64(b, RW_WORK_LAST, m.work.saturating_sub(spent.work))?;
    put_u64(b, RW_BYTES_LAST, m.bytes.saturating_sub(spent.bytes))?;
    *spent = Spent { work: m.work, bytes: m.bytes, blocks: clock.pages() };
    Ok(r)
}

/// Builds the row layout a read found missing, in pieces; `true` once none is pending.
fn layouts<H: PageHost>(h: &mut H, shape: &Shape, rw: &Rw, m: &mut Meter, stamp: u64) -> Result<bool, Fault> {
    loop {
        if versions::pending(&*h, rw.st)? == 0 {
            return Ok(true);
        }
        // A piece of a layout's build may cost more than a piece with no estimate of its own: it begins only
        // when the unit has room for its estimate.
        if m.full() || !m.room(COST_LAYOUT) {
            return Ok(false);
        }
        let mut ws = m.ws.ok_or(rt(Corruption::Link, rw.st))?;
        let (touched, done) = versions::build_piece(h, shape, rw.st, &mut ws, stamp)?;
        m.ws = Some(ws);
        m.spend(COST_LAYOUT);
        for _ in 0..touched {
            m.touch();
        }
        if done {
            m.lay = None;
            m.lay_at = None;
        }
    }
}

fn next_step<H: PageHost>(h: &mut H, st: u64, step: u8) -> Result<(), Fault> {
    let b = h.get_mut(Region::Block(st), Owner(0))?;
    put_u8(b, RW_STEP, step)?;
    put_u32(b, RW_LEAF, 0)?;
    put_u32(b, RW_POS, 0)?;
    put_u32(b, RW_Q, 0)?;
    put_u64(b, RW_VAT, 0)?;
    put_u64(b, RW_SUB, 0)?;
    put_u32(b, RW_SEG, 0)?;
    put_u64(b, RW_SEG_Q, 0)?;
    put_u64(b, RW_ACC, 0)
}

/// The snapshot's later units: the work page's locators, zeroed in pieces.
fn snapshot_rest<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<(), Fault> {
    // A compaction or column release whose schema version a schema change passed before its snapshot: the
    // writes made meanwhile went to the group as ordinary writes, leaves the rewrite would not write among
    // them, so it starts again from a fresh begin (the group queued again, or met again by the release's
    // walk).
    if rw.kind != SEAL && shape.version != rw.version {
        abandon(h, tbl, rw.st)?;
        crate::mem::put_u64(h.get_mut(Region::Block(rw.source), Owner(0))?, page::P_REWRITE, 0)?;
        if rw.kind == COMPACTION {
            super::jobs::queue(h, tbl, rw.g, rw.source, false)?;
        }
        m.touch();
        m.ended = true;
        return Ok(());
    }
    if rw.kind == SEAL && st_u64(&*h, rw.st, RW_SUB)? == 0 {
        if !liveness(h, rw, m)? {
            return Ok(());
        }
        // The snapshot is taken now: the writes before it were ordinary writes to the rows, at whatever
        // schema version was current, so the seal writes the leaves of the version current at its snapshot.
        // Its work space holds nothing yet but its header, so it is begun again for that version's leaves.
        if shape.version != rw.version {
            let gen = latest_drop(&*h, shape)?;
            let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
            put_u32(b, RW_VERSION, shape.version)?;
            put_u32(b, RW_GEN, gen)?;
            Space::begin(h, tbl, rw.work, rw.g, shape.leaves)?;
            m.touch();
        }
        set_u64(h, rw.st, RW_SUB, 1)?;
        if m.full() {
            return Ok(());
        }
    }
    // A block's bytes a piece, the meter read before each.
    let mut ws = work_space(&*h, tbl, rw)?;
    loop {
        if m.full() {
            return Ok(());
        }
        let done = ws.zero_step(h, BLOCK)?;
        m.spend(COST_LEAF);
        m.touch();
        if done {
            break;
        }
    }
    if rw.kind != SEAL {
        take_snapshot(h, rw)?;
    }
    next_step(h, rw.st, STEP_PASS1)
}

// ---------------------------------------------------------------- pass 1

/// What a walk for the next leaf a rewrite writes found.
enum Next {
    Leaf(LeafInfo),
    /// None before this ordinal: the walk resumes there.
    From(u64),
    End,
}

/// Leaves one walk for the next written leaf examines at most.
const NEXT_SPAN: u64 = 128;

/// The next leaf the rewrite writes at or after ordinal o, examining at most `NEXT_SPAN` leaves (in place, a few
/// loads each), so that a stretch of leaves the rewrite skips is crossed in pieces.
fn next_leaf<H: PageHost + ?Sized>(h: &H, shape: &Shape, rw: &Rw, o: u64, m: &mut Meter) -> Result<Next, Fault> {
    if let Some(l) = m.leaf_at {
        if u64::from(l.ordinal) == o {
            m.lead(COST_SPAN);
            return Ok(Next::Leaf(l));
        }
    }
    let end = o.saturating_add(NEXT_SPAN).min(shape.leaves);
    let mut found = None;
    let mut walked = 0u64;
    shape.spans(h, o, end, |k, s| {
        walked = walked.wrapping_add(1);
        if s.at(rw.version) {
            found = Some(k);
            return Ok(false);
        }
        Ok(true)
    })?;
    m.lead(walked.wrapping_mul(COST_SPAN));
    Ok(match found {
        Some(k) => {
            let l = shape.leaf(h, k)?;
            m.leaf_at = Some(l);
            Next::Leaf(l)
        }
        None if end >= shape.leaves => Next::End,
        None => {
            // Every caller saves the ordinal the walk resumes at: progress.
            m.touch();
            Next::From(end)
        }
    })
}

/// Pass 1's sub-steps: a leaf begun, its stored positions scanned, its plan made.
const P_BEGIN: u64 = 0;
const P_SCAN: u64 = 1;
const P_FINISH: u64 = 2;

/// Pass 1, leaf by leaf: its sub-steps from `RW_SUB`.
fn pass1<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<(), Fault> {
    let src = source(&*h, rw)?;
    loop {
        if m.full() {
            return Ok(());
        }
        let o = st_u32(&*h, rw.st, RW_LEAF)?;
        let l = match next_leaf(&*h, shape, rw, o, m)? {
            Next::End => return next_step(h, rw.st, STEP_LAYOUT),
            Next::From(k) => {
                set_u32(h, rw.st, RW_LEAF, k)?;
                set_u64(h, rw.st, RW_SUB, P_BEGIN)?;
                continue;
            }
            Next::Leaf(l) => l,
        };
        if u64::from(l.ordinal) != o {
            set_u32(h, rw.st, RW_LEAF, u64::from(l.ordinal))?;
            set_u64(h, rw.st, RW_SUB, P_BEGIN)?;
        }
        let o = u64::from(l.ordinal);
        let sub = st_u64(&*h, rw.st, RW_SUB)?;
        match sub {
            P_BEGIN => {
                m.spend(COST_LEAF);
                let acc = Acc { flags: A_SEQ_OK, imin: i64::MAX, imax: i64::MIN, ..Acc::default() };
                acc.store(h, rw.st)?;
                let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
                put_u64(b, RW_SUB, P_SCAN)?;
                put_u32(b, RW_POS, 0)?;
                put_u32(b, RW_Q, 0)?;
                put_u64(b, RW_VAT, 0)?;
            }
            P_SCAN => scan(h, shape, rw, &src, &l, m)?,
            P_FINISH => {
                finish_leaf(h, tbl, rw, &l, m)?;
                let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
                put_u32(b, RW_LEAF, u32::try_from(o.wrapping_add(1)).map_err(|_| rt(Corruption::Range, o))?)?;
                put_u64(b, RW_SUB, P_BEGIN)?;
                let n = u64::from(rd_u32(b, RW_LEAVES)?);
                put_u32(b, RW_LEAVES, u32::try_from(n.wrapping_add(1)).map_err(|_| rt(Corruption::Range, n))?)?;
            }
            _ => return Err(rt(Corruption::Range, sub)),
        }
    }
}

/// Where the in-place walk takes up again after positions it left to the general path: the general path
/// reads the positions (or slots) up to `until`. A piece that stopped before reading any doubles that
/// stretch, up to `BACK_MAX`, so that rows the walk cannot read (moved rows, rows of another schema version,
/// long items) cost the walk little; a piece that read some sets it back to one.
#[derive(Clone, Copy, Default)]
struct Back {
    until: Option<u64>,
    run: u64,
}

const BACK_MAX: u64 = 64;

impl Back {
    /// Whether the walk reads position (or slot) `at`.
    fn walks(&self, at: u64) -> bool {
        self.until.is_none_or(|u| at > u)
    }

    /// The walk stopped at `at` after reading `read` positions.
    fn slow(&mut self, at: u64, read: u64) {
        self.run = if read > 0 { 1 } else { self.run.saturating_mul(2).clamp(1, BACK_MAX) };
        self.until = Some(at.saturating_add(self.run.wrapping_sub(1)));
    }
}

/// Leaf `l`'s place in the rows of a seal's snapshot, for the in-place walks: a leaf of a kind they read, in
/// rows of the snapshot's schema version, once that version's row layout is built (the general path builds
/// it at the first row of the version it reads). `None` when the walks do not read the leaf.
fn cut_of<H: PageHost + ?Sized>(h: &H, rw: &Rw, l: &LeafInfo, m: &mut Meter) -> Result<Option<walk::Cut>, Fault> {
    if rw.kind != SEAL || !l.at(rw.version) || walk::Kind::of(l.ty).is_none() {
        return Ok(None);
    }
    let Some(ws) = m.ws else { return Ok(None) };
    // The snapshot's version's row layout: the one the meter keeps, else found (and kept).
    let e = match m.lay {
        Some(e) if e.version == rw.version => e,
        _ => {
            let Some((_, e)) = versions::find(h, rw.st, &ws, rw.version)? else { return Ok(None) };
            m.lay = Some(e);
            m.lay_head = e.head();
            m.lay_at = None;
            e
        }
    };
    let Ok((f, vi, bit)) = versions::offsets(h, &ws, &e, u64::from(l.ordinal)) else { return Ok(None) };
    Ok(walk::Cut::new(l.ty, &e.head(), f, vi, bit))
}

/// The scan of pass 1 over the stored positions, from the cursor, the meter read before every piece: a
/// value read with what it feeds (its leaf's range, `Seq` test and sizes), or a piece of the in-place walk.
fn scan<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    m: &mut Meter,
) -> Result<(), Fault> {
    let t = l.ty;
    m.lead(COST_ACC);
    let mut acc = Acc::load(&*h, rw.st)?;
    // The accumulators' load and the leaf's place in the rows are two pieces: the share may end between
    // them, having changed nothing.
    if m.full() {
        return Ok(());
    }
    let mut q = st_u32(&*h, rw.st, RW_Q)?;
    // A value read at q whose feeding is still to come (the share may end between them; the next reads it
    // again).
    let mut read_s: Option<SVal> = None;
    let mut done = false;
    // The in-place walk, for a leaf it reads: its place in the rows, and the position it last left to the
    // general path.
    let cut = cut_of(&*h, rw, l, m)?;
    let int = matches!(t, Ty::U(_) | Ty::I(_) | Ty::Nat | Ty::Int);
    let mut back = Back::default();
    loop {
        if m.full() {
            break;
        }
        if let Some(cut) = cut.as_ref() {
            if read_s.is_none() && back.walks(q) {
                let stop = m.walk_stop();
                let (pc, qq) = walk::scan(&*h, rw, cut, &mut acc, q, m.walk_limit(), &stop);
                if qq != q {
                    m.spend(pc.work);
                } else {
                    // A walk that read no position leads to the general path's piece.
                    m.lead(pc.work);
                }
                if pc.bytes > 0 {
                    m.bytes(pc.bytes);
                }
                q = qq;
                if pc.end {
                    done = true;
                    break;
                }
                if pc.slow {
                    back.slow(q, pc.read);
                }
                continue;
            }
        }
        let s = match read_s.take() {
            Some(s) => s,
            None => {
                let Some(qq) = next_stored(&*h, rw, q)? else {
                    done = true;
                    break;
                };
                q = qq;
                let live = snap_live(&*h, rw.work, q)?;
                if !live {
                    m.spend(COST_CELL);
                    acc.flags |= A_STORED_NULL;
                    q = q.wrapping_add(1);
                    continue;
                }
                let Some(s) = value(&*h, shape, src, l, m, q)? else { break };
                m.lead(COST_VALUE);
                if s.len() > 64 {
                    // A long value's read is a piece of its own, which leads to what it feeds, the next.
                    read_s = Some(s);
                    continue;
                }
                s
            }
        };
        // What the value feeds: the range and `Seq` test, sizes.
        m.spend(COST_ACCUM);
        if s.is_null() {
            acc.flags |= A_STORED_NULL;
            acc.flags &= !A_SEQ_OK;
            q = q.wrapping_add(1);
            continue;
        }
        if int {
            match s {
                SVal::V(Val::U(x)) => {
                    acc.umax = acc.umax.max(x);
                    seq_add(&mut acc, q, i128::from(x));
                    if let Ok(y) = i64::try_from(x) {
                        acc.imin = acc.imin.min(y);
                        acc.imax = acc.imax.max(y);
                    }
                }
                SVal::V(Val::I(x)) => {
                    acc.imin = acc.imin.min(x);
                    acc.imax = acc.imax.max(x);
                    seq_add(&mut acc, q, i128::from(x));
                }
                _ => acc.flags |= A_ANY_BIG,
            }
        }
        match (t, s) {
            (Ty::Text | Ty::Blob, SVal::B { len, .. }) => size(&mut acc, len, false, false),
            (Ty::Text | Ty::Blob, SVal::Huge { len, .. }) => size(&mut acc, len, true, false),
            (Ty::Fixed(n), _) if u64::from(n) > FIXED_MAX => size(&mut acc, u64::from(n), false, false),
            (Ty::Nat | Ty::Int, SVal::B { len, .. }) => size(&mut acc, len, false, true),
            (Ty::Nat | Ty::Int, SVal::Huge { len, .. }) => size(&mut acc, len, true, true),
            (Ty::Nat | Ty::Int, SVal::V(v)) => {
                let mut b = [0u8; 19];
                size(&mut acc, cell::small_leb(&v, matches!(t, Ty::Int), &mut b) as u64, false, true);
            }
            _ => {}
        }
        q = q.wrapping_add(1);
    }
    acc.store(h, rw.st)?;
    set_u32(h, rw.st, RW_Q, q)?;
    if done {
        set_u64(h, rw.st, RW_SUB, P_FINISH)?;
    }
    Ok(())
}

/// The end of a leaf's pass 1, one piece: its form and its plan, from its accumulators.
fn finish_leaf<H: PageHost>(h: &mut H, tbl: u64, rw: &Rw, l: &LeafInfo, m: &mut Meter) -> Result<(), Fault> {
    let t = l.ty;
    m.spend(COST_LEAF.wrapping_add(COST_ACC));
    let acc = Acc::load(&*h, rw.st)?;
    let leb_form = matches!(t, Ty::Nat | Ty::Int) && acc.has(A_ANY_BIG);
    let mut ws = work_space(&*h, tbl, rw)?;
    let mut pl = Plan { written: true, ..Plan::default() };
    pl.form = match t {
        Ty::Bool => form::BOOL,
        Ty::U(_) => u_form(acc.umax),
        Ty::I(_) => i_form(acc.imin.min(acc.imax), acc.imax.max(acc.imin)),
        Ty::Nat if !leb_form => u_form(acc.umax),
        Ty::Int if !leb_form => i_form(acc.imin.min(acc.imax), acc.imax.max(acc.imin)),
        Ty::Nat | Ty::Int => form::LEB,
        Ty::F32 => form::F32,
        Ty::F64 => form::F64,
        Ty::Char => form::U32,
        Ty::Text => form::UTF8,
        Ty::Blob => form::BINARY,
        Ty::Principal => form::FIXED,
        Ty::Fixed(n) if u64::from(n) <= FIXED_MAX => form::FIXED,
        Ty::Fixed(_) => form::BINARY,
        Ty::U128 | Ty::I128 => form::X16,
        Ty::Unit => form::UNIT,
    };
    // A null clears the `Seq` test, so a leaf that passes it holds no null.
    if matches!(t, Ty::U(_) | Ty::I(_) | Ty::Nat | Ty::Int) && !leb_form {
        let (lo, hi) = int_range(t);
        if let Some((base, step)) = seq_params(&acc, lo, hi) {
            pl.form = form::SEQ;
            // Both lie in the leaf's 64-bit range (`seq_params` checked): stored as u64 or i64 bits.
            #[allow(clippy::cast_possible_truncation)]
            {
                pl.seq_base = base as u64;
                pl.seq_step = step as u64;
            }
        }
    }
    pl.width = match pl.form {
        form::FIXED => match t {
            Ty::Principal => 30,
            Ty::Fixed(n) => u64::from(n),
            _ => 0,
        },
        f => group::cell_width(f, 0),
    };
    if acc.has(A_STORED_NULL) && pl.form != form::SEQ {
        pl.flags |= LF_VALIDITY;
    }
    let k = usize::from(leb_form);
    let sz = |i: usize| acc.sizes.get(i.wrapping_add(k)).copied().unwrap_or(0);
    if group::variable(pl.form) {
        pl.run = sz(0);
        pl.ool_k = sz(2);
        pl.ool_bytes = sz(4);
        pl.huge = sz(6);
    }
    if pl.ool_k > 0 {
        pl.flags |= LF_OOL;
    }
    store_plan(h, &mut ws, u64::from(l.ordinal), &pl)?;
    m.touch();
    Ok(())
}

// ---------------------------------------------------------------- layout

/// Lays out a leaf's cell segments one at a time: as many positions as fit in the rest of the image block, a
/// segment that cannot take one starting at the next block.
struct Packer {
    f: u8,
    w: u64,
    v: bool,
    n: u64,
    c: u64,
    first: u64,
}

impl Packer {
    fn next(&mut self) -> Result<Option<group::Seg>, Fault> {
        loop {
            if self.first >= self.n {
                return Ok(None);
            }
            let left = self.n.wrapping_sub(self.first);
            let room = PAGE.wrapping_sub(self.c % PAGE);
            let (mut lo, mut hi) = (0u64, left);
            while lo < hi {
                let mid = lo.wrapping_add(hi.wrapping_sub(lo).div_ceil(2));
                if group::cell_bytes(self.f, self.w, self.v, mid) <= room {
                    lo = mid;
                } else {
                    hi = mid.wrapping_sub(1);
                }
            }
            if lo == 0 {
                if room == PAGE {
                    return Err(rt(Corruption::Layout, self.c));
                }
                self.c = self.c.wrapping_add(room);
                continue;
            }
            let b = group::cell_bytes(self.f, self.w, self.v, lo);
            let s = group::Seg {
                off: self.c,
                first: u32::try_from(self.first).map_err(|_| rt(Corruption::Range, self.first))?,
                positions: u32::try_from(lo).map_err(|_| rt(Corruption::Range, lo))?,
                bytes: u32::try_from(b).map_err(|_| rt(Corruption::Range, b))?,
                kind: seg::CELLS,
            };
            self.c = self.c.checked_add(b).ok_or(rt(Corruption::Range, self.c))?;
            self.first = self.first.wrapping_add(lo);
            return Ok(Some(s));
        }
    }
}

/// Every segment of a leaf in table order (cells, `Seq` part, out-of-line directory, inline run), from the
/// leaf's cell cursor; returns the cursor after its parts.
fn leaf_segments<F>(pl: &Plan, n: u64, run_at: u64, mut f: F) -> Result<u64, Fault>
where
    F: FnMut(group::Seg) -> Result<bool, Fault>,
{
    let mut cursor = pl.cell_at;
    if group::has_cells(pl.form, pl.validity()) {
        let mut p = Packer { f: pl.form, w: pl.width, v: pl.validity(), n, c: cursor, first: 0 };
        while let Some(s) = p.next()? {
            if !f(s)? {
                return Ok(p.c);
            }
        }
        cursor = p.c;
    }
    tail_segments(pl, n, run_at, cursor, f)
}

/// A leaf's segments after its cells (`Seq` part, out-of-line directory, inline run), from `cursor`, where its
/// cells end; returns the cursor after them. At most three.
fn tail_segments<F>(pl: &Plan, n: u64, run_at: u64, mut cursor: u64, mut f: F) -> Result<u64, Fault>
where
    F: FnMut(group::Seg) -> Result<bool, Fault>,
{
    let u32of = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let mut part = |bytes: u64, kind: u8, positions: u64, f: &mut F| -> Result<bool, Fault> {
        let off = group::place(cursor, bytes);
        cursor = off.checked_add(bytes).ok_or(rt(Corruption::Range, off))?;
        f(group::Seg { off, first: 0, positions: u32of(positions)?, bytes: u32of(bytes)?, kind })
    };
    if pl.form == form::SEQ && !part(16, seg::SEQ, n, &mut f)? {
        return Ok(cursor);
    }
    if pl.flags & LF_OOL != 0 && !part(group::ool_dir_bytes(n, pl.ool_k), seg::OOL, n, &mut f)? {
        return Ok(cursor);
    }
    if group::variable(pl.form) {
        f(group::Seg { off: run_at, first: 0, positions: u32of(n)?, bytes: u32of(pl.run)?, kind: seg::RUN })?;
    }
    Ok(cursor)
}

/// The rank map's offset (0: none) and the front metadata's end.
fn front<H: PageHost + ?Sized>(h: &H, rw: &Rw) -> Result<(u64, u64), Fault> {
    let leaves = st_u32(h, rw.st, RW_LEAVES)?;
    let fr = group::HEADER.wrapping_add(leaves.wrapping_mul(group::LD_BYTES));
    let rank_at = if rw.compacted { fr } else { 0 };
    let end = if rw.compacted { fr.wrapping_add(group::RANK_MAP) } else { fr };
    Ok((rank_at, group::align8(end)))
}

/// The most cell segments one piece of the layout lays out.
const LAYOUT_SEGS: u64 = 4;

/// The layout, in pieces of a leaf or of `LAYOUT_SEGS` of its cell segments: each leaf's segments from the
/// cell cursor, its inline and out-of-line offsets; then the totals. Between the pieces of one leaf, the
/// cell segments laid out so far are counted at `RW_POS` and the layout's cursor (its next byte and its next
/// position) kept at `RW_VAT` and `RW_ACC`.
fn layout<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<(), Fault> {
    let mut ws = work_space(&*h, tbl, rw)?;
    if st_u64(&*h, rw.st, RW_SUB)? == 0 {
        // The leaves written are known now (`RW_LEAVES`), and so is the front metadata's end.
        m.spend(COST_LEAF);
        let (_, hh) = front(&*h, rw)?;
        let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
        put_u32(b, RW_H, u32::try_from(hh).map_err(|_| rt(Corruption::Range, hh))?)?;
        put_u64(b, RW_C, hh)?;
        put_u64(b, RW_SUB, 1)?;
        if m.full() {
            return Ok(());
        }
    }
    loop {
        if m.full() {
            return Ok(());
        }
        let o = st_u32(&*h, rw.st, RW_LEAF)?;
        let l = match next_leaf(&*h, shape, rw, o, m)? {
            Next::End => break,
            Next::From(k) => {
                set_u32(h, rw.st, RW_LEAF, k)?;
                continue;
            }
            Next::Leaf(l) => l,
        };
        let o = u64::from(l.ordinal);
        let mut pl = load_plan(&*h, &ws, o)?.ok_or(rt(Corruption::Link, o))?;
        let b = h.get(Region::Block(rw.st))?;
        let (c, s_total, run, oolt) =
            (rd_u64(b, RW_C)?, u64::from(rd_u32(b, RW_S)?), rd_u64(b, RW_RUN)?, rd_u64(b, RW_OOLT)?);
        let (x, v) = (u64::from(rd_u32(b, RW_X)?), u64::from(rd_u32(b, RW_V)?));
        pl.cell_at = c;
        pl.first_seg = s_total;
        pl.run_rel = run;
        pl.ool_rel = oolt;
        // The cell segments, at most `LAYOUT_SEGS` a piece, from where the last piece of the leaf left them.
        let mut count = u64::from(rd_u32(b, RW_POS)?);
        let mut cursor = c;
        if group::has_cells(pl.form, pl.validity()) {
            let (pc, first) = if count == 0 { (c, 0) } else { (rd_u64(b, RW_VAT)?, rd_u64(b, RW_ACC)?) };
            let mut p = Packer { f: pl.form, w: pl.width, v: pl.validity(), n: rw.n, c: pc, first };
            let mut k = 0u64;
            let mut done = false;
            while k < LAYOUT_SEGS {
                if p.next()?.is_none() {
                    done = true;
                    break;
                }
                k = k.wrapping_add(1);
            }
            count = count.wrapping_add(k);
            m.spend(COST_LEAF.wrapping_add(k.wrapping_mul(COST_CELL)));
            if !done {
                let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
                put_u32(b, RW_POS, u32::try_from(count).map_err(|_| rt(Corruption::Range, count))?)?;
                put_u64(b, RW_VAT, p.c)?;
                put_u64(b, RW_ACC, p.first)?;
                continue;
            }
            cursor = p.c;
        }
        let end = tail_segments(&pl, rw.n, 0, cursor, |_| {
            count = count.wrapping_add(1);
            Ok(true)
        })?;
        pl.segs = count;
        m.spend(COST_LEAF);
        store_plan(h, &mut ws, o, &pl)?;
        let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
        put_u32(b, RW_POS, 0)?;
        put_u64(b, RW_VAT, 0)?;
        put_u64(b, RW_ACC, 0)?;
        put_u64(b, RW_C, end)?;
        put_u32(b, RW_S, u32::try_from(s_total.wrapping_add(count)).map_err(|_| rt(Corruption::Range, count))?)?;
        put_u64(b, RW_RUN, run.checked_add(pl.run).ok_or(rt(Corruption::Range, run))?)?;
        put_u64(b, RW_OOLT, oolt.checked_add(pl.ool_bytes).ok_or(rt(Corruption::Range, oolt))?)?;
        put_u32(b, RW_X, u32::try_from(x.wrapping_add(pl.ool_k)).map_err(|_| rt(Corruption::Range, x))?)?;
        put_u32(b, RW_V, u32::try_from(v.wrapping_add(pl.huge)).map_err(|_| rt(Corruption::Range, v))?)?;
        put_u32(b, RW_LEAF, u32::try_from(o.wrapping_add(1)).map_err(|_| rt(Corruption::Range, o))?)?;
    }
    // The totals.
    let b = h.get(Region::Block(rw.st))?;
    let inline_at = rd_u64(b, RW_C)?;
    let ool_start = inline_at.checked_add(rd_u64(b, RW_RUN)?).ok_or(rt(Corruption::Range, inline_at))?;
    let ool_end = ool_start.checked_add(rd_u64(b, RW_OOLT)?).ok_or(rt(Corruption::Range, ool_start))?;
    let segs_at = ool_end.wrapping_add(31) & !31;
    let s = u64::from(rd_u32(b, RW_S)?);
    let total = segs_at.checked_add(s.wrapping_mul(group::SEG_BYTES)).ok_or(rt(Corruption::Range, segs_at))?;
    let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
    put_u64(b, RW_INLINE, inline_at)?;
    put_u64(b, RW_OOL, ool_start)?;
    put_u64(b, RW_SEGS, segs_at)?;
    put_u64(b, RW_TOTAL, total)?;
    next_step(h, rw.st, STEP_WRITE)
}

// ---------------------------------------------------------------- the image

/// Write phases (sub-step).
const W_HEADER: u64 = 0;
const W_DIR: u64 = 1;
const W_RANK: u64 = 2;
const W_CELLS: u64 = 3;
const W_INLINE: u64 = 4;
const W_OOL: u64 = 5;
const W_SEGTABLE: u64 = 6;
const W_DONE: u64 = 7;

/// How a variable value is stored in the image.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    None,
    Inline(u64),
    Medium(u64),
    Huge,
}

fn place_of(s: &SVal, t: Ty) -> Place {
    match *s {
        SVal::Null | SVal::Unit => Place::None,
        SVal::Huge { .. } => Place::Huge,
        SVal::B { len, .. } => {
            if len > LONG_VALUE {
                Place::Medium(len)
            } else {
                Place::Inline(len)
            }
        }
        SVal::V(v) => {
            let mut b = [0u8; 19];
            Place::Inline(cell::small_leb(&v, matches!(t, Ty::Int), &mut b) as u64)
        }
    }
}

/// Copies bytes [from, from + n) of a variable value (a small Nat or Int as its LEB128).
fn value_bytes<H: PageHost + ?Sized>(h: &H, s: &SVal, t: Ty, from: u64, out: &mut [u8]) -> Result<(), Fault> {
    match s {
        SVal::V(v) => {
            let mut b = [0u8; 19];
            let n = cell::small_leb(v, matches!(t, Ty::Int), &mut b);
            let src = b.get(..n).unwrap_or(&[]);
            let s0 = usize::try_from(from).unwrap_or(usize::MAX);
            let piece = src.get(s0..s0.saturating_add(out.len())).ok_or(rt(Corruption::Range, from))?;
            crate::mem::copy_bytes(out, piece);
            Ok(())
        }
        other => cur::read(h, other, from, out),
    }
}

/// The image's header.
fn header_bytes<H: PageHost + ?Sized>(h: &H, tbl: u64, shape: &Shape, rw: &Rw) -> Result<[u8; 128], Fault> {
    let b = h.get(Region::Block(rw.st))?;
    let (rank_at, hh) = front(h, rw)?;
    let total = rd_u64(b, RW_TOTAL)?;
    let x = u64::from(rd_u32(b, RW_X)?);
    let v = u64::from(rd_u32(b, RW_V)?);
    let s = u64::from(rd_u32(b, RW_S)?);
    let leaves = u64::from(rd_u32(b, RW_LEAVES)?);
    let ool_start = rd_u64(b, RW_OOL)?;
    let segs_at = rd_u64(b, RW_SEGS)?;
    let ool_end = ool_start.wrapping_add(rd_u64(b, RW_OOLT)?);
    let uid = root::u64_at(h, tbl, root::UID)?;
    let mut hd = [0u8; 128];
    crate::schema::put_common_header(&mut hd, group::KIND_GROUP, 0, total, uid, shape.hash)?;
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let w32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
    put_u64(&mut hd, group::G_GROUP, rw.g)?;
    put_u16(&mut hd, group::G_P, w16(rw.p)?)?;
    put_u16(&mut hd, group::G_N, w16(rw.n)?)?;
    put_u16(&mut hd, group::G_L, w16(leaves)?)?;
    let mut flags = 0u16;
    if rw.compacted {
        flags |= GF_COMPACTED;
    }
    if x > 0 {
        flags |= GF_OOL;
    }
    if v > 0 {
        flags |= GF_HUGE;
    }
    put_u16(&mut hd, group::G_FLAGS, flags)?;
    put_u32(&mut hd, group::G_VERSION, rw.version)?;
    put_u32(&mut hd, group::G_S, w32(s)?)?;
    put_u32(&mut hd, group::G_X, w32(x)?)?;
    put_u32(&mut hd, group::G_H, w32(hh)?)?;
    put_u32(&mut hd, group::G_RANK, w32(rank_at)?)?;
    put_u32(&mut hd, group::G_DIR, w32(group::HEADER)?)?;
    put_u64(&mut hd, group::G_OOL_AT, ool_start)?;
    put_u64(&mut hd, group::G_OOL_BYTES, ool_end.wrapping_sub(ool_start))?;
    put_u32(&mut hd, group::G_V, w32(v)?)?;
    put_u64(&mut hd, group::G_SEGS, segs_at)?;
    put_u64(&mut hd, group::G_FIRST_HUGE, 0)?;
    Ok(hd)
}

/// The cursor of the write phases, kept in the state block between units.
#[derive(Clone, Copy, Debug)]
struct WCur {
    phase: u64,
    leaf: u64,
    seg: u64,
    part: u64,
    slot: u64,
    q: u64,
    vat: u64,
    acc: u64,
    seg_q: u64,
}

impl WCur {
    fn load<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<WCur, Fault> {
        let b = h.get(Region::Block(st))?;
        let sub = rd_u64(b, RW_SUB)?;
        Ok(WCur {
            phase: sub & 0xFF,
            part: sub >> 8,
            leaf: u64::from(rd_u32(b, RW_LEAF)?),
            seg: u64::from(rd_u32(b, RW_SEG)?),
            slot: u64::from(rd_u32(b, RW_POS)?),
            q: u64::from(rd_u32(b, RW_Q)?),
            vat: rd_u64(b, RW_VAT)?,
            acc: rd_u64(b, RW_ACC)?,
            seg_q: rd_u64(b, RW_SEG_Q)?,
        })
    }

    fn store<H: PageHost>(&self, h: &mut H, st: u64) -> Result<(), Fault> {
        let b = h.get_mut(Region::Block(st), Owner(0))?;
        let w32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
        put_u64(b, RW_SUB, self.phase | (self.part << 8))?;
        put_u32(b, RW_LEAF, w32(self.leaf)?)?;
        put_u32(b, RW_SEG, w32(self.seg)?)?;
        put_u32(b, RW_POS, w32(self.slot)?)?;
        put_u32(b, RW_Q, w32(self.q)?)?;
        put_u64(b, RW_VAT, self.vat)?;
        put_u64(b, RW_ACC, self.acc)?;
        put_u64(b, RW_SEG_Q, self.seg_q)
    }

    /// The next leaf: its cursors reset.
    fn next_leaf(&mut self) {
        self.leaf = self.leaf.wrapping_add(1);
        self.seg = 0;
        self.part = 0;
        self.slot = 0;
        self.q = 0;
        self.vat = 0;
        self.acc = 0;
        self.seg_q = 0;
    }

    fn next_phase(&mut self) {
        self.phase = self.phase.wrapping_add(1);
        self.leaf = 0;
        self.seg = 0;
        self.part = 0;
        self.slot = 0;
        self.q = 0;
        self.vat = 0;
        self.acc = 0;
        self.seg_q = 0;
    }
}

/// The image, in order, from the write cursor until the unit is spent.
fn write<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<(), Fault> {
    // The output's stage and cursor are made after a reading of the meter: the share's state and the stage
    // are two pieces.
    if m.full() {
        return Ok(());
    }
    let img = Image::read_record(h.get(Region::Block(rw.st))?, RW_IMAGE)?;
    let mut out = Out { img, tbl, stage: [0; 4_096], staged: 0 };
    let mut c = WCur::load(&*h, rw.st)?;
    let src = source(&*h, rw)?;
    let r = write_phases(h, tbl, shape, rw, &src, &mut out, &mut c, m);
    out.flush(h)?;
    out.img.put_record(h.get_mut(Region::Block(rw.st), Owner(0))?, RW_IMAGE)?;
    r?;
    c.store(h, rw.st)?;
    if c.phase >= W_DONE {
        let total = st_u64(&*h, rw.st, RW_TOTAL)?;
        if out.img.length != total {
            return Err(rt(Corruption::Count, out.img.length));
        }
        next_step(h, rw.st, STEP_FLIP)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_phases<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    out: &mut Out,
    c: &mut WCur,
    m: &mut Meter,
) -> Result<(), Fault> {
    let ws = work_space(&*h, tbl, rw)?;
    let b = h.get(Region::Block(rw.st))?;
    let segs_at = rd_u64(b, RW_SEGS)?;
    let inline_at = rd_u64(b, RW_INLINE)?;
    let ool_start = rd_u64(b, RW_OOL)?;
    let hh = u64::from(rd_u32(b, RW_H)?);
    // No leaf with inline values, or none with values out of line: that phase writes nothing.
    let runs = ool_start.saturating_sub(inline_at);
    let ools = u64::from(rd_u32(b, RW_X)?);
    while c.phase < W_DONE {
        if (c.phase == W_INLINE && runs == 0) || (c.phase == W_OOL && ools == 0) {
            c.next_phase();
            continue;
        }
        if m.full() {
            return Ok(());
        }
        match c.phase {
            W_HEADER => {
                let hd = header_bytes(&*h, tbl, shape, rw)?;
                out.put(h, m, &hd)?;
                c.next_phase();
            }
            W_DIR => {
                let l = match next_leaf(&*h, shape, rw, c.leaf, m)? {
                    Next::End => {
                        c.next_phase();
                        continue;
                    }
                    Next::From(k) => {
                        c.leaf = k;
                        continue;
                    }
                    Next::Leaf(l) => l,
                };
                let pl = load_plan(&*h, &ws, u64::from(l.ordinal))?.ok_or(rt(Corruption::Link, c.leaf))?;
                let ld = group::LeafDir {
                    id: l.id,
                    ordinal: l.ordinal,
                    form: pl.form,
                    flags: pl.flags,
                    positions: u32::try_from(rw.p).map_err(|_| rt(Corruption::Range, rw.p))?,
                    width: u16::try_from(pl.width).map_err(|_| rt(Corruption::Range, pl.width))?,
                    first_seg: u32::try_from(pl.first_seg).map_err(|_| rt(Corruption::Range, pl.first_seg))?,
                    segs: u32::try_from(pl.segs).map_err(|_| rt(Corruption::Range, pl.segs))?,
                    ool: u32::try_from(pl.ool_k).map_err(|_| rt(Corruption::Range, pl.ool_k))?,
                };
                let mut e = [0u8; 32];
                ld.write(&mut e)?;
                out.put(h, m, &e)?;
                m.spend(COST_LEAF);
                c.leaf = u64::from(l.ordinal).wrapping_add(1);
            }
            W_RANK => {
                if rw.compacted {
                    let mut bm = [0u8; 512];
                    crate::mem::copy_bytes(&mut bm, crate::mem::sub(h.get(Region::Block(rw.work))?, RW_SNAP, 512)?);
                    out.put(h, m, &bm)?;
                    // The rank map, each word's rank a u16, made in place and written once.
                    let mut ranks = [0u8; 128];
                    let mut rank = 0u32;
                    for wd in 0..64u64 {
                        put_u16(&mut ranks, wd.wrapping_mul(2), u16::try_from(rank).unwrap_or(0))?;
                        let word = u64::from_le_bytes(crate::mem::rd_array::<8>(&bm, wd.wrapping_mul(8))?);
                        rank = rank.wrapping_add(word.count_ones());
                    }
                    out.put(h, m, &ranks)?;
                }
                // The padding to H.
                out.pad_to(h, m, hh)?;
                c.next_phase();
            }
            W_CELLS => {
                let l = match next_leaf(&*h, shape, rw, c.leaf, m)? {
                    Next::End => {
                        c.next_phase();
                        continue;
                    }
                    Next::From(k) => {
                        c.next_leaf();
                        c.leaf = k;
                        continue;
                    }
                    Next::Leaf(l) => l,
                };
                if u64::from(l.ordinal) != c.leaf {
                    c.leaf = u64::from(l.ordinal);
                    c.next_leaf();
                    c.leaf = u64::from(l.ordinal);
                }
                let pl = load_plan(&*h, &ws, c.leaf)?.ok_or(rt(Corruption::Link, c.leaf))?;
                // The plan's load leads to the leaf's cells: the cursor stays on the leaf if the share ends.
                m.lead(COST_LEAF);
                if m.full() {
                    return Ok(());
                }
                if cells_leaf(h, shape, rw, src, &l, &pl, out, c, m)? {
                    // A leaf's cells end where the layout placed the end of its last segment.
                    let end = leaf_segments(&pl, rw.n, 0, |_| Ok(true))?;
                    if out.pos() != end {
                        return Err(rt(Corruption::Layout, out.pos()));
                    }
                    c.next_leaf();
                    // The cursor past the leaf is saved: progress, also for a leaf with no cells (a snapshot
                    // with no live row), whose work so far only led to it.
                    m.touch();
                }
            }
            W_INLINE => {
                let l = match next_leaf(&*h, shape, rw, c.leaf, m)? {
                    Next::End => {
                        c.next_phase();
                        continue;
                    }
                    Next::From(k) => {
                        c.next_leaf();
                        c.leaf = k;
                        continue;
                    }
                    Next::Leaf(l) => l,
                };
                if u64::from(l.ordinal) != c.leaf {
                    c.next_leaf();
                    c.leaf = u64::from(l.ordinal);
                }
                let pl = load_plan(&*h, &ws, c.leaf)?.ok_or(rt(Corruption::Link, c.leaf))?;
                if !group::variable(pl.form) {
                    m.spend(COST_LEAF);
                    c.next_leaf();
                    continue;
                }
                if c.q == 0 && c.vat == 0 {
                    out.pad_to(h, m, inline_at.wrapping_add(pl.run_rel))?;
                }
                if values_leaf(h, tbl, shape, rw, src, &l, out, c, m, false)? {
                    c.next_leaf();
                }
            }
            W_OOL => {
                let l = match next_leaf(&*h, shape, rw, c.leaf, m)? {
                    Next::End => {
                        c.next_phase();
                        continue;
                    }
                    Next::From(k) => {
                        c.next_leaf();
                        c.leaf = k;
                        continue;
                    }
                    Next::Leaf(l) => l,
                };
                if u64::from(l.ordinal) != c.leaf {
                    c.next_leaf();
                    c.leaf = u64::from(l.ordinal);
                }
                let pl = load_plan(&*h, &ws, c.leaf)?.ok_or(rt(Corruption::Link, c.leaf))?;
                if pl.ool_k == 0 {
                    m.spend(COST_LEAF);
                    c.next_leaf();
                    continue;
                }
                if c.q == 0 && c.vat == 0 {
                    out.pad_to(h, m, ool_start.wrapping_add(pl.ool_rel))?;
                }
                if values_leaf(h, tbl, shape, rw, src, &l, out, c, m, true)? {
                    c.next_leaf();
                }
            }
            W_SEGTABLE => {
                if c.leaf == 0 && c.seg == 0 {
                    out.pad_to(h, m, segs_at)?;
                }
                let l = match next_leaf(&*h, shape, rw, c.leaf, m)? {
                    Next::End => {
                        c.next_phase();
                        continue;
                    }
                    Next::From(k) => {
                        c.leaf = k;
                        c.seg = 0;
                        continue;
                    }
                    Next::Leaf(l) => l,
                };
                let pl = load_plan(&*h, &ws, u64::from(l.ordinal))?.ok_or(rt(Corruption::Link, c.leaf))?;
                // Finding segment c.seg walks the leaf's segments from its first.
                m.spend(c.seg.wrapping_mul(COST_CELL));
                let mut segs = [[0u8; 32]; 1];
                let mut i = 0u64;
                let mut entries = 0u64;
                let mut err = Ok(());
                leaf_segments(&pl, rw.n, inline_at.wrapping_add(pl.run_rel), |s| {
                    if i >= c.seg {
                        match s.bytes32() {
                            Ok(b) => segs[0] = b,
                            Err(e) => err = Err(e),
                        }
                        entries = entries.wrapping_add(1);
                        i = i.wrapping_add(1);
                        return Ok(false);
                    }
                    i = i.wrapping_add(1);
                    Ok(true)
                })?;
                err?;
                if entries == 0 {
                    // A leaf with no segment (a snapshot with no live row): its plan loaded and its segments
                    // walked, a leaf's piece.
                    m.spend(COST_LEAF);
                    c.leaf = u64::from(l.ordinal).wrapping_add(1);
                    c.seg = 0;
                    continue;
                }
                out.put(h, m, &segs[0])?;
                m.spend(COST_LEAF);
                c.leaf = u64::from(l.ordinal);
                c.seg = c.seg.wrapping_add(1);
                if c.seg >= pl.segs {
                    c.leaf = c.leaf.wrapping_add(1);
                    c.seg = 0;
                }
            }
            _ => c.phase = W_DONE,
        }
    }
    Ok(())
}

/// The cell area of one leaf from the cursor: its segments in order. `true` when the leaf is done.
#[allow(clippy::too_many_arguments)]
fn cells_leaf<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    pl: &Plan,
    out: &mut Out,
    c: &mut WCur,
    m: &mut Meter,
) -> Result<bool, Fault> {
    let cut = cut_of(&*h, rw, l, m)?;
    loop {
        if m.full() {
            return Ok(false);
        }
        // Segment c.seg of the leaf, found by a walk over the leaf's segments from its first: it leads to the
        // segment's pieces.
        m.lead(c.seg.wrapping_add(1).wrapping_mul(COST_CELL));
        let mut found = None;
        let mut i = 0u64;
        leaf_segments(pl, rw.n, 0, |s| {
            if i == c.seg {
                found = Some(s);
                return Ok(false);
            }
            i = i.wrapping_add(1);
            Ok(true)
        })?;
        let Some(sg) = found else { return Ok(true) };
        if sg.kind == seg::RUN {
            return Ok(true);
        }
        if c.part == 0 && c.slot == 0 && c.vat == 0 {
            out.pad_to(h, m, sg.off)?;
        }
        let done = match sg.kind {
            seg::CELLS => cell_segment(h, shape, rw, src, l, pl, &sg, out, c, m, cut.as_ref())?,
            seg::SEQ => {
                out.put(h, m, &pl.seq_base.to_le_bytes())?;
                out.put(h, m, &pl.seq_step.to_le_bytes())?;
                true
            }
            seg::OOL => ool_dir(h, shape, rw, src, l, pl, &sg, out, c, m, cut.as_ref())?,
            _ => true,
        };
        if !done {
            return Ok(false);
        }
        c.seg = c.seg.wrapping_add(1);
        c.part = 0;
        c.slot = 0;
        c.vat = 0;
        c.seg_q = c.q;
    }
}

/// Slot `c.slot` onward of one cell segment: its validity words (part 0), then its cells (part 1).
#[allow(clippy::too_many_arguments)]
fn cell_segment<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    pl: &Plan,
    sg: &group::Seg,
    out: &mut Out,
    c: &mut WCur,
    m: &mut Meter,
    cut: Option<&walk::Cut>,
) -> Result<bool, Fault> {
    let t = l.ty;
    let k = u64::from(sg.positions);
    if c.part == 0 && c.slot == 0 {
        c.seg_q = c.q;
    }
    if c.part == 0 {
        if pl.validity() {
            // One value per piece; the word under construction waits in the cursor (`vat`, zero at the
            // segment's start) until its 64th position or the segment's end. The in-place walk writes the
            // slots it reads, the general path the others, one at a time.
            let mut back = Back::default();
            while c.slot < k {
                if m.full() {
                    return Ok(false);
                }
                if let Some(cut) = cut {
                    if back.walks(c.slot) {
                        let pc = walk_write(h, rw, cut, walk::Part::Validity, pl, c, k, out, m)?;
                        if pc.slow {
                            back.slow(c.slot, pc.read);
                        }
                        continue;
                    }
                }
                let q = next_stored(&*h, rw, c.q)?.ok_or(rt(Corruption::Count, c.slot))?;
                let Some(s) = live_value(&*h, shape, rw, src, l, m, q)? else { return Ok(false) };
                let present = !s.is_null();
                m.spend(COST_VALUE);
                c.vat |= u64::from(present) << (c.slot % 64);
                c.slot = c.slot.wrapping_add(1);
                c.q = q.wrapping_add(1);
                if c.slot % 64 == 0 || c.slot == k {
                    out.put(h, m, &c.vat.to_le_bytes())?;
                    c.vat = 0;
                }
            }
        }
        c.part = 1;
        c.slot = 0;
        c.q = c.seg_q;
        c.vat = out.pos();
    }
    let body = c.vat;
    let part = cut.and_then(|ct| walk_part(ct, pl));
    let mut back = Back::default();
    while c.slot < k {
        if m.full() {
            return Ok(false);
        }
        if let (Some(cut), Some(part)) = (cut, part) {
            if back.walks(c.slot) {
                let pc = walk_write(h, rw, cut, part, pl, c, k, out, m)?;
                if pc.slow {
                    back.slow(c.slot, pc.read);
                }
                continue;
            }
        }
        let q = next_stored(&*h, rw, c.q)?.ok_or(rt(Corruption::Count, c.slot))?;
        let Some(s) = live_value(&*h, shape, rw, src, l, m, q)? else { return Ok(false) };
        m.spend(COST_VALUE);
        let i = c.slot;
        match pl.form {
            form::BOOL => {
                // The word under construction waits in the cursor (`acc`, which a Bool leaf has no other
                // use for) until its 64th position or the segment's end.
                c.acc |= u64::from(matches!(s, SVal::V(Val::Bool(true)))) << (i % 64);
                if i % 64 == 63 || i.wrapping_add(1) == k {
                    out.put(h, m, &c.acc.to_le_bytes())?;
                    c.acc = 0;
                }
            }
            form::UNIT => {}
            f if group::variable(f) => {
                out.put(h, m, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
                if let Place::Inline(len) = place_of(&s, t) {
                    c.acc = c.acc.wrapping_add(len);
                }
                if i.wrapping_add(1) == k {
                    out.put(h, m, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
                }
            }
            form::FIXED => {
                // A null's cell is zeros; a value's bytes are staged 64 at a time.
                let w = pl.width;
                if s.is_null() {
                    let z = FIXED_ZEROS.get(..usize::try_from(w).unwrap_or(0)).ok_or(rt(Corruption::Range, w))?;
                    out.put(h, m, z)?;
                } else if matches!(t, Ty::Principal) {
                    out.put(h, m, &cur::principal_cell(&*h, &s)?)?;
                } else {
                    // The cell's bytes in one read: a `Fixed` form holds at most `FIXED_MAX` of them.
                    let mut b = [0u8; 1_016];
                    let bk = b.get_mut(..usize::try_from(w).unwrap_or(usize::MAX)).ok_or(rt(Corruption::Range, w))?;
                    cur::read(&*h, &s, 0, bk)?;
                    out.put(h, m, bk)?;
                }
            }
            _ => {
                let mut b = [0u8; 16];
                match s {
                    SVal::V(Val::U(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    SVal::V(Val::I(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    SVal::V(Val::F32(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    SVal::V(Val::F64(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    SVal::V(Val::U128(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    SVal::V(Val::I128(x)) => crate::mem::copy_bytes(&mut b, &x.to_le_bytes()),
                    _ => {}
                }
                out.put(h, m, b.get(..usize::try_from(pl.width).unwrap_or(0)).unwrap_or(&[]))?;
            }
        }
        c.slot = c.slot.wrapping_add(1);
        c.q = q.wrapping_add(1);
    }
    out.pad_to(h, m, body.wrapping_add(group::cell_bytes(pl.form, pl.width, false, k)))?;
    Ok(true)
}

/// Leaf `l`'s value at stored position q: `Null` for a position dead at the snapshot; `None` as `value`.
fn live_value<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    m: &mut Meter,
    q: u64,
) -> Result<Option<SVal>, Fault> {
    if snap_live(h, rw.work, q)? {
        value(h, shape, src, l, m, q)
    } else {
        Ok(Some(SVal::Null))
    }
}

/// What the in-place walk writes of a leaf's cell segment's body, when it reads the leaf's form: a fixed-width
/// form's cells or a `Bool` form's bits (`Cells`), a Text's or Blob's offsets (`Offsets`).
fn walk_part(cut: &walk::Cut, pl: &Plan) -> Option<walk::Part> {
    use walk::Kind as K;
    let ok = match (cut.kind, pl.form) {
        (K::Bool, form::BOOL) => true,
        (K::U(_) | K::I(_) | K::Nat | K::Int, form::U8 | form::U16 | form::U32 | form::U64) => true,
        (K::U(_) | K::I(_) | K::Nat | K::Int, form::I8 | form::I16 | form::I32 | form::I64) => true,
        (K::Char, form::U32) | (K::F32, form::F32) | (K::F64, form::F64) => true,
        (K::Bytes, form::UTF8 | form::BINARY) => return Some(walk::Part::Offsets),
        (K::Principal, form::FIXED) => true,
        _ => false,
    };
    ok.then_some(walk::Part::Cells)
}

/// One piece of the in-place walk writing `part` of leaf `pl`'s segment, slots `c.slot` to k − 1, through the
/// output's stage, which is written out when the piece found it full.
#[allow(clippy::too_many_arguments)]
fn walk_write<H: PageHost>(
    h: &mut H,
    rw: &Rw,
    cut: &walk::Cut,
    part: walk::Part,
    pl: &Plan,
    c: &mut WCur,
    k: u64,
    out: &mut Out,
    m: &mut Meter,
) -> Result<walk::Piece, Fault> {
    let from = out.pos();
    let mut cur = walk::Cur { slot: c.slot, q: c.q, word: c.vat, acc: c.acc };
    let width = usize::try_from(pl.width).unwrap_or(0);
    let mut st = walk::Stage { buf: &mut out.stage, at: out.staged };
    let limit = m.walk_limit();
    let stop = m.walk_stop();
    let pc = walk::write(&*h, rw, cut, part, pl.form, width, &mut cur, k, &mut st, limit, &stop);
    out.staged = st.at;
    let moved = cur.slot != c.slot;
    c.slot = cur.slot;
    c.q = cur.q;
    match part {
        walk::Part::Validity | walk::Part::OolBits => c.vat = cur.word,
        walk::Part::Cells | walk::Part::Offsets => c.acc = cur.acc,
    }
    let to = out.pos();
    if moved || to != from {
        m.touch();
        m.spend(pc.work);
        m.bytes(to.wrapping_sub(from));
    } else {
        // A walk that wrote no slot leads to the general path's piece.
        m.lead(pc.work);
    }
    if pc.full {
        out.flush(h)?;
    }
    Ok(pc)
}

/// The out-of-line directory: its bitmap over the stored slots (part 0), its rank (part 1), its offsets
/// (part 2).
#[allow(clippy::too_many_arguments)]
fn ool_dir<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    pl: &Plan,
    sg: &group::Seg,
    out: &mut Out,
    c: &mut WCur,
    m: &mut Meter,
    cut: Option<&walk::Cut>,
) -> Result<bool, Fault> {
    let n = rw.n;
    let t = l.ty;
    let ool = |s: &SVal| matches!(place_of(s, t), Place::Medium(_) | Place::Huge);
    // The in-place walk reads a Text or Blob leaf's slots; the general path the others, one at a time.
    let cut = cut.filter(|ct| ct.kind == walk::Kind::Bytes && matches!(pl.form, form::UTF8 | form::BINARY));
    let mut back = Back::default();
    if c.part == 0 {
        if c.slot == 0 {
            // The directory covers every stored slot from the first.
            c.q = 0;
        }
        // One value per piece; the word under construction waits in the cursor (`vat`, zero at the
        // segment's start) until its 64th slot or the last.
        while c.slot < n {
            if m.full() {
                return Ok(false);
            }
            if let Some(cut) = cut {
                if back.walks(c.slot) {
                    let pc = walk_write(h, rw, cut, walk::Part::OolBits, pl, c, n, out, m)?;
                    if pc.slow {
                        back.slow(c.slot, pc.read);
                    }
                    continue;
                }
            }
            let q = next_stored(&*h, rw, c.q)?.ok_or(rt(Corruption::Count, c.slot))?;
            let Some(s) = live_value(&*h, shape, rw, src, l, m, q)? else { return Ok(false) };
            c.vat |= u64::from(ool(&s)) << (c.slot % 64);
            m.spend(COST_VALUE);
            c.slot = c.slot.wrapping_add(1);
            c.q = q.wrapping_add(1);
            if c.slot % 64 == 0 || c.slot == n {
                out.put(h, m, &c.vat.to_le_bytes())?;
                c.vat = 0;
            }
        }
        c.part = 1;
        c.slot = 0;
        c.q = 0;
    }
    if c.part == 1 {
        // One rank per 4,096 slots: the out-of-line values before them. A group has at most 4,096 slots, so
        // the one rank is 0; written generally all the same.
        let mut before = 0u32;
        for chunk in 0..n.div_ceil(4_096) {
            out.put(h, m, &before.to_le_bytes())?;
            let _ = chunk;
            before = before.wrapping_add(0);
        }
        c.part = 2;
        c.slot = 0;
        c.q = 0;
        c.acc = pl.ool_rel;
        out.put(h, m, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
    }
    back = Back::default();
    while c.slot < n {
        if m.full() {
            return Ok(false);
        }
        if let Some(cut) = cut {
            if back.walks(c.slot) {
                // The slots whose values lie inline write nothing: the walk passes them.
                let mut cur = walk::Cur { slot: c.slot, q: c.q, word: 0, acc: 0 };
                let stop = m.walk_stop();
                let pc = walk::ool_offsets(&*h, rw, cut, &mut cur, n, m.walk_limit(), &stop);
                if cur.slot != c.slot {
                    m.spend(pc.work);
                } else {
                    m.lead(pc.work);
                }
                c.slot = cur.slot;
                c.q = cur.q;
                if pc.slow {
                    back.slow(c.slot, pc.read);
                }
                continue;
            }
        }
        let q = next_stored(&*h, rw, c.q)?.ok_or(rt(Corruption::Count, c.slot))?;
        let Some(s) = live_value(&*h, shape, rw, src, l, m, q)? else { return Ok(false) };
        m.spend(COST_VALUE);
        let len = match place_of(&s, t) {
            Place::Medium(len) => Some(len),
            Place::Huge => Some(8),
            _ => None,
        };
        if let Some(len) = len {
            c.acc = c.acc.wrapping_add(len);
            out.put(h, m, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
        }
        c.slot = c.slot.wrapping_add(1);
        c.q = q.wrapping_add(1);
    }
    out.pad_to(h, m, sg.off.wrapping_add(u64::from(sg.bytes)))?;
    Ok(true)
}

/// A leaf's values in the inline data region (`ool` false) or the out-of-line area: each copied in pieces
/// from the cursor's byte offset; a huge value's reference is copied into the new long-value list and its
/// index written. `true` when the leaf is done.
#[allow(clippy::too_many_arguments)]
fn values_leaf<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    out: &mut Out,
    c: &mut WCur,
    m: &mut Meter,
    ool: bool,
) -> Result<bool, Fault> {
    let t = l.ty;
    // The in-place walk copies a Text or Blob leaf's inline values whole, and passes them in the out-of-line
    // area; the general path the others, and a value it copies in pieces, one at a time.
    let cut = cut_of(&*h, rw, l, m)?.filter(|ct| ct.kind == walk::Kind::Bytes);
    let mut back = Back::default();
    loop {
        if m.full() {
            return Ok(false);
        }
        if let Some(cut) = cut.as_ref() {
            if c.vat == 0 && back.walks(c.q) {
                let mut cur = walk::Cur { slot: 0, q: c.q, word: 0, acc: 0 };
                let mut st = walk::Stage { buf: &mut out.stage, at: out.staged };
                let stop = m.walk_stop();
                let pc = walk::run(&*h, rw, cut, &mut cur, &mut st, ool, m.walk_limit(), &stop);
                out.staged = st.at;
                let moved = cur.q != c.q || pc.bytes > 0;
                c.q = cur.q;
                if moved {
                    m.touch();
                    m.spend(pc.work);
                    m.bytes(pc.bytes);
                } else {
                    m.lead(pc.work);
                }
                if pc.full {
                    out.flush(h)?;
                }
                if pc.end {
                    return Ok(true);
                }
                if pc.slow {
                    back.slow(c.q, pc.read);
                }
                continue;
            }
        }
        // A dead position holds no value: on to the next live one, a bitmap word at a time.
        let Some(q) = next_live(&*h, rw, c.q)? else { return Ok(true) };
        let Some(s) = value(&*h, shape, src, l, m, q)? else { return Ok(false) };
        m.lead(COST_VALUE);
        match (place_of(&s, t), ool) {
            (Place::Inline(len), false) | (Place::Medium(len), true) => {
                let mut piece = [0u8; 1_024];
                while c.vat < len {
                    if m.full() {
                        return Ok(false);
                    }
                    let k = len.wrapping_sub(c.vat).min(1_024);
                    let p = piece.get_mut(..usize::try_from(k).unwrap_or(0)).unwrap_or(&mut []);
                    value_bytes(&*h, &s, t, c.vat, p)?;
                    out.put(h, m, p)?;
                    m.spend(COST_CELL);
                    c.vat = c.vat.wrapping_add(k);
                }
            }
            (Place::Huge, true) => {
                let SVal::Huge { img, .. } = s else { return Err(rt(Corruption::Link, q)) };
                // The push is a piece of its own (it may start a list block): the cursor stays at q.
                if m.full() {
                    return Ok(false);
                }
                let i = lv_push(h, tbl, rw, &img)?;
                // The list's last block, and a new one when the push starts it.
                m.touch();
                m.touch();
                out.put(h, m, &i.to_le_bytes())?;
            }
            _ => {}
        }
        c.vat = 0;
        c.q = q.wrapping_add(1);
        m.touch();
    }
}

/// Appends an image reference to the new long-value list; returns its index.
fn lv_push<H: PageHost>(h: &mut H, tbl: u64, rw: &Rw, img: &Image) -> Result<u64, Fault> {
    let (head, count, tail) = {
        let b = h.get(Region::Block(rw.st))?;
        (rd_u64(b, RW_LV)?, u64::from(rd_u32(b, RW_LV_COUNT)?), rd_u64(b, RW_LV_TAIL)?)
    };
    let mut lv = page::LvBuilder::resume(head, tail, count);
    let i = lv.push(h, tbl, img)?;
    let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
    put_u64(b, RW_LV, lv.head)?;
    put_u32(b, RW_LV_COUNT, u32::try_from(lv.count).map_err(|_| rt(Corruption::Range, lv.count))?)?;
    put_u64(b, RW_LV_TAIL, lv.tail())?;
    Ok(i)
}

// ---------------------------------------------------------------- flip and the last units

/// The flip, O(1): the directory names the pending page, which takes the new image, its long-value list, the
/// post-snapshot patches and the group's extremes block; a seal pops the frozen queue and takes the group's
/// rows off the buffered live rows.
fn flip<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<(), Fault> {
    m.spend(COST_LEAF);
    let (img, lv, lv_count, v_total) = {
        let b = h.get(Region::Block(rw.st))?;
        (Image::read_record(b, RW_IMAGE)?, rd_u64(b, RW_LV)?, rd_u32(b, RW_LV_COUNT)?, rd_u32(b, RW_V)?)
    };
    let live = buffer::u16_of(&*h, rw.pending, page::P_LIVE)?;
    let stored = rw.n;
    let dead = stored.saturating_sub(live);
    {
        let b = h.get_mut(Region::Block(rw.pending), Owner(0))?;
        let post = rd_u64(b, page::P_POST_PATCH)?;
        let mut flags = 0u16;
        if rw.compacted {
            flags |= page::PF_COMPACTED;
        }
        if v_total > 0 {
            flags |= page::PF_HUGE;
        }
        if post != 0 {
            flags |= page::PF_PATCHES;
        }
        put_u16(b, page::P_FLAGS, flags)?;
        img.put_record(b, page::P_IMAGE)?;
        put_u16(b, page::P_DEAD, u16::try_from(dead).map_err(|_| rt(Corruption::Range, dead))?)?;
        put_u16(b, 102, 0)?;
        put_u32(b, page::P_VERSION, rw.version)?;
        put_u32(b, page::P_LV_COUNT, lv_count)?;
        put_u64(b, page::P_LV_LIST, lv)?;
        put_u64(b, page::P_PATCH_DIR, post)?;
        put_u64(b, page::P_POST_PATCH, 0)?;
    }
    // The directory names the pending page only while the group is still the one the snapshot was taken of.
    let entry = root::group(&*h, tbl, rw.g)?;
    let ours = if rw.kind == SEAL { entry == buffer::DESC_BIT | rw.source } else { entry == rw.source };
    if ours {
        root::set_group(h, tbl, rw.g, rw.pending)?;
        if super::extremes::hand_over(h, rw.source, rw.kind == SEAL, rw.pending)? {
            let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
            let f = rd_u16(b, RW_FLAGS)?;
            put_u16(b, RW_FLAGS, f | RWF_EXT)?;
        }
        if rw.kind == SEAL && generation_of(&*h, rw.st)? < latest_drop(&*h, shape)? {
            // A seal whose snapshot came before the latest drop wrote the dropped leaves: the column release's
            // walk, which passed the group while it was buffered, goes back to it.
            super::evolve::stale_seal_flipped(h, tbl, rw.g)?;
        }
    }
    if rw.kind == SEAL && ours {
        let next = buffer::u64_of(&*h, rw.source, buffer::D_NEXT)?;
        root::set_u64(h, tbl, root::FROZEN_HEAD, next)?;
        if next == 0 {
            root::set_u64(h, tbl, root::FROZEN_TAIL, 0)?;
        }
        let waiting = root::u32_at(&*h, tbl, root::FROZEN_WAITING)?;
        root::set_u32(h, tbl, root::FROZEN_WAITING, waiting.saturating_sub(1))?;
        // The buffered live rows lose the group's (the integer leaves' totals count its rows still).
        let tail = root::u64_at(&*h, tbl, root::TAIL)?;
        let t = h.get_mut(Region::Block(tail), Owner(0))?;
        let buffered = rd_u64(t, buffer::H_BUFFERED)?;
        put_u64(t, buffer::H_BUFFERED, buffered.wrapping_sub(live))?;
    }
    next_step(h, rw.st, STEP_LAST)
}

/// The last units: every huge value's old referent is cleared (part 0), then the old structures, the work
/// page and the state block go to the drain (part 1). `true` when done.
fn last<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw, m: &mut Meter) -> Result<bool, Fault> {
    let ws = work_space(&*h, tbl, rw)?;
    if st_u64(&*h, rw.st, RW_SUB)? == 0 {
        // The source is read only here: once its structures go to the drain (sub-step 1), the drain may free
        // them before the next piece.
        let src = source_at_last(&*h, rw)?;
        loop {
            if m.full() {
                return Ok(false);
            }
            let o = st_u32(&*h, rw.st, RW_LEAF)?;
            let l = match next_leaf(&*h, shape, rw, o, m)? {
                Next::End => break,
                Next::From(k) => {
                    let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
                    put_u32(b, RW_LEAF, u32::try_from(k).map_err(|_| rt(Corruption::Range, k))?)?;
                    put_u32(b, RW_Q, 0)?;
                    continue;
                }
                Next::Leaf(l) => l,
            };
            let o = u64::from(l.ordinal);
            m.lead(COST_LEAF);
            let pl = load_plan(&*h, &ws, o)?.ok_or(rt(Corruption::Link, o))?;
            let mut q = if o == st_u32(&*h, rw.st, RW_LEAF)? { st_u32(&*h, rw.st, RW_Q)? } else { 0 };
            let mut done = true;
            if pl.huge > 0 {
                // The in-place walk passes the rows whose item is not a long item; the general path takes
                // the others, one at a time.
                let cut = cut_of(&*h, rw, &l, m)?.filter(|ct| ct.kind == walk::Kind::Bytes);
                let mut back = Back::default();
                loop {
                    if m.full() {
                        done = false;
                        break;
                    }
                    if let Some(cut) = cut.as_ref() {
                        if back.walks(q) {
                            let mut cur = walk::Cur { slot: 0, q, word: 0, acc: 0 };
                            let stop = m.walk_stop();
                            let pc = walk::no_huge(&*h, rw, cut, &mut cur, m.walk_limit(), &stop);
                            if cur.q != q {
                                m.spend(pc.work);
                            } else {
                                m.lead(pc.work);
                            }
                            q = cur.q;
                            if pc.end {
                                break;
                            }
                            if pc.slow {
                                back.slow(q, pc.read);
                            }
                            continue;
                        }
                    }
                    let Some(qq) = next_stored(&*h, rw, q)? else { break };
                    q = qq;
                    if snap_live(&*h, rw.work, q)? {
                        let owner = huge_owner(&*h, shape, &src, &l, m, q)?;
                        if m.pending != 0 {
                            done = false;
                            break;
                        }
                        m.lead(COST_VALUE);
                        // The owner's clearing is a piece of its own: the next unit finds it again.
                        if owner.is_some() && m.full() {
                            done = false;
                            break;
                        }
                        if let Some(owner) = owner {
                            forget(h, owner)?;
                            m.spend(COST_LEAF);
                            m.touch();
                        }
                    } else {
                        m.spend(COST_SKIP);
                    }
                    // The position is passed, which the cursor saves: progress.
                    q = q.wrapping_add(1);
                    m.touch();
                }
            }
            let b = h.get_mut(Region::Block(rw.st), Owner(0))?;
            if done {
                put_u32(b, RW_LEAF, u32::try_from(o.wrapping_add(1)).map_err(|_| rt(Corruption::Range, o))?)?;
                put_u32(b, RW_Q, 0)?;
                m.touch();
            } else {
                put_u32(b, RW_LEAF, u32::try_from(o).map_err(|_| rt(Corruption::Range, o))?)?;
                put_u32(b, RW_Q, u32::try_from(q).map_err(|_| rt(Corruption::Range, q))?)?;
                return Ok(false);
            }
        }
        set_u64(h, rw.st, RW_SUB, 1)?;
        if m.full() {
            return Ok(false);
        }
    }
    // The old structures, then the rewrite's own, in two pieces.
    if st_u64(&*h, rw.st, RW_SUB)? == 1 {
        // The source no longer names the extremes block the flip gave the group's page.
        if rd_u16(h.get(Region::Block(rw.st))?, RW_FLAGS)? & RWF_EXT != 0 {
            super::extremes::forget(h, rw.source, rw.kind == SEAL)?;
            m.touch();
        }
        if rw.kind == SEAL {
            drain::push_descriptor(h, tbl, rw.source)?;
            // The descriptor's block and the last waiting descriptor's.
            m.touch();
            m.touch();
        } else {
            drain::push(h, tbl, &drain::Rec::new(drain::K_GROUP_PAGE, rw.source, 0))?;
            m.touch();
        }
        m.spend(COST_LEAF);
        set_u64(h, rw.st, RW_SUB, 2)?;
        if m.full() {
            return Ok(false);
        }
    }
    if ws.blocks.root != 0 {
        drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK_LIST, ws.blocks.root, ws.blocks.height))?;
    }
    drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK, rw.work, 0))?;
    drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK, rw.st, 0))?;
    // The installed page is written only while the directory still names it.
    if root::group(&*h, tbl, rw.g)? == rw.pending {
        put_u64(h.get_mut(Region::Block(rw.pending), Owner(0))?, page::P_REWRITE, 0)?;
    }
    set_running(h, tbl, rw.kind, 0)?;
    super::jobs::after_rewrite(h, tbl, rw.g, rw.pending)?;
    Ok(true)
}

/// Where the snapshot keeps leaf `l`'s huge value at position q, found without reading the value (a write
/// after the flip may have released it): a row's long item, a patch entry's long value, or a long-value
/// list entry.
fn huge_owner<H: PageHost + ?Sized>(
    h: &H,
    shape: &Shape,
    src: &Source,
    l: &LeafInfo,
    m: &mut Meter,
    q: u64,
) -> Result<Option<cur::Owner>, Fault> {
    let _ = shape;
    match *src {
        Source::Buffered { desc } => {
            let Some((block, slot)) = buffer::locate_in(h, desc, q)? else { return Ok(None) };
            let Some(loc) = buffer::slot_row(h, block, slot)? else { return Ok(None) };
            let r = RowRef::new(h, loc)?;
            let v = r.version()?;
            if !l.ty.variable() || !l.at(v) {
                return Ok(None);
            }
            // A version with no row layout yet leaves `m` pending; the caller ends its share.
            let Some((_, vi, _)) = m.layout(h, v, u64::from(l.ordinal))? else { return Ok(None) };
            let Some(at) = r.long_ref_in(&m.lay_head, vi)? else { return Ok(None) };
            // A long item the rewrite did not adopt stays the row's, and goes to the drain with its group's
            // buffer blocks.
            let mut b = [0u8; 24];
            r.read(at, &mut b)?;
            Ok(adopts(&Image::read_ref(&b, 0)?).then_some(cur::Owner::Item { loc, at }))
        }
        Source::Sealed { meta, dir } => {
            if dir != 0 {
                if let Some(e) = patch::lookup(h, dir, u64::from(l.ordinal), q)? {
                    if e.flags & patch::F_LONG != 0 {
                        return Ok(Some(cur::Owner::Patch { block: e.block, data: e.data }));
                    }
                    return Ok(None);
                }
            }
            let gv = GroupView::from_meta(h, &meta);
            let Some(ld) = gv.leaf_by_ordinal(u64::from(l.ordinal))? else { return Ok(None) };
            let Some(s) = gv.slot(q)? else { return Ok(None) };
            Ok(gv.huge_index(&ld, s)?.map(|index| cur::Owner::List { page: meta.page, index }))
        }
    }
}

/// The snapshot's source after the flip, when its old patch directory is still the one of the snapshot.
fn source_at_last<H: PageHost + ?Sized>(h: &H, rw: &Rw) -> Result<Source, Fault> {
    source(h, rw)
}

/// Clears a huge value's old referent: the new long-value list is its one referent now.
fn forget<H: PageHost>(h: &mut H, owner: cur::Owner) -> Result<(), Fault> {
    match owner {
        cur::Owner::Item { loc, at } => match loc {
            RowLoc::Inline { block, off, .. } => {
                fill(h.get_mut(Region::Block(block), Owner(0))?, off.wrapping_add(at), 24, 0)
            }
            RowLoc::Long(img) => super::long::write_at(h, img, at, &[0u8; 24]),
        },
        cur::Owner::List { page, index } => {
            let (b, off) = page::lv_entry(&*h, page, index)?;
            fill(h.get_mut(Region::Block(b), Owner(0))?, off, crate::image::REF_BYTES, 0)
        }
        cur::Owner::Patch { block, data } => patch::forget_long(h, block, data),
    }
}

// ---------------------------------------------------------------- abandon

/// Abandons a rewrite that has not flipped: its new image, its pending page with the post-snapshot patches, its
/// work page and its state block go to the drain; the new long-value list's
/// blocks go without their entries, whose values the old structures still own.
pub fn abandon<H: PageHost>(h: &mut H, tbl: u64, st: u64) -> Result<(), Fault> {
    let rw = Rw::read(&*h, st)?;
    let (img, lv) = {
        let b = h.get(Region::Block(st))?;
        (Image::read_record(b, RW_IMAGE)?, rd_u64(b, RW_LV)?)
    };
    {
        let b = h.get_mut(Region::Block(rw.pending), Owner(0))?;
        img.put_record(b, page::P_IMAGE)?;
        let post = rd_u64(b, page::P_POST_PATCH)?;
        put_u64(b, page::P_PATCH_DIR, post)?;
        put_u64(b, page::P_POST_PATCH, 0)?;
        put_u64(b, page::P_LV_LIST, 0)?;
        put_u32(b, page::P_LV_COUNT, 0)?;
        put_u64(b, page::P_REWRITE, 0)?;
    }
    if lv != 0 {
        drain::push(h, tbl, &drain::Rec { height: 1, ..drain::Rec::new(drain::K_LV_LIST, lv, 0) })?;
    }
    drain::push(h, tbl, &drain::Rec::new(drain::K_GROUP_PAGE, rw.pending, 0))?;
    let ws = work_space(&*h, tbl, &rw)?;
    if ws.blocks.root != 0 {
        drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK_LIST, ws.blocks.root, ws.blocks.height))?;
    }
    drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK, rw.work, 0))?;
    drain::push(h, tbl, &drain::Rec::new(drain::K_BLOCK, st, 0))?;
    set_running(h, tbl, rw.kind, 0)
}

/// The estimated instructions of a rewrite's first unit: the blocks it allocates and fills.
pub fn snapshot_work() -> u64 {
    COST_LEAF.wrapping_mul(2)
}

/// The estimated instructions of a rewrite's last unit.
pub fn work_of<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u64, Fault> {
    st_u64(h, st, RW_WORK_LAST)
}

/// The step of a rewrite.
pub fn step_of<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u8, Fault> {
    st_u8(h, st, RW_STEP)
}

/// The kind of a rewrite.
pub fn kind_of<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u8, Fault> {
    st_u8(h, st, RW_KIND)
}

/// Units a rewrite has run (status replies).
pub fn units_of<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u64, Fault> {
    st_u64(h, st, RW_UNITS)
}

/// The latest drop a rewrite's snapshot saw.
pub fn generation_of<H: PageHost + ?Sized>(h: &H, st: u64) -> Result<u32, Fault> {
    Ok(u32::try_from(st_u32(h, st, RW_GEN)?).unwrap_or(0))
}
