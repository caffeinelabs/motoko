//! Rewrites: the seal of a full group, and the compaction of a sealed group. Each runs
//! whole in the call that starts it: it reads the group's rows, builds the group's new image with one writer,
//! names the image on a new group page, which the directory then names, and frees what the group held before.
//! No read or write ever meets a rewrite half done, so the image's bytes depend on the group's live rows alone.
//!
//! **Steps.** (1) The group's liveness: a seal reads its buffer blocks' slots, a compaction its group page's
//! live bitmap; the compacted form follows (`compacted_form`). (2) *Pass 1*, leaf by leaf over
//! the stored positions: the leaf's form and sizes, its plan in the work space. (3) The *layout*, leaf by leaf,
//! from the sizes alone. (4) The *image*, written in order (header, leaf directory, rank map, cell area, inline
//! data region, out-of-line area, segment table); a huge value's reference moves into the new long-value list.
//! (5) The new group page takes the image, the long-value list and the group's extremes block, and the
//! directory names it; the caller frees the old structures.
//!
//! **Work space** (`stats::space`), on a work block allocated for the rewrite and freed at its end: leaf o's
//! locator names leaf o's plan; the row layouts a seal reads its rows in follow it (`versions`).

use crate::consts::{magic, BLOCK, BUFFER_BLOCKS_MAX, COMPACT_DEAD, FIXED_MAX, FORMAT_TABLE, HUGE_VALUE, LONG_VALUE};
use crate::fault::{rt, Corruption, Fault};
use crate::image::Image;
use crate::mem::{fill, put_u16, put_u32, put_u64, put_u8, rd_u16, rd_u32, rd_u64, rd_u8, PageHost, Region};
use crate::stats::space::Space;

use super::buffer::{self, RowLoc, SLOT_DEAD};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::group::{self, form, seg, GroupMeta, GroupView, GF_COMPACTED, GF_HUGE, GF_OOL, LF_OOL, LF_VALIDITY};
use super::layout::{LeafInfo, RowHead, Shape, Ty};
use super::page;
use super::patch;
use super::root;
use super::row::{self, RowRef};
use super::versions::{self, Versions};

mod walk;

/// Rewrite kinds.
pub const SEAL: u8 = 1;
pub const COMPACTION: u8 = 2;

/// A rewrite's facts and its group's liveness.
pub struct Rw {
    pub kind: u8,
    pub g: u64,
    /// The group's buffer descriptor (a seal), or its group page.
    pub source: u64,
    /// The group's positions, the positions its image stores and its live rows.
    pub p: u64,
    pub n: u64,
    pub live: u64,
    pub compacted: bool,
    /// A seal's block records rise strictly, each first position its block's, and every block holds a slot:
    /// a walk over the group's positions takes the blocks in record order.
    pub ordered: bool,
    /// The schema version the image is written at: the table's.
    pub version: u32,
    /// The patch directory a compaction reads (0: none).
    pub dir: u64,
    /// The live bitmap by position.
    pub bits: [u64; 64],
}

impl Rw {
    fn word(&self, w: u64) -> u64 {
        usize::try_from(w).ok().and_then(|i| self.bits.get(i)).copied().unwrap_or(0)
    }

    fn is_live(&self, q: u64) -> bool {
        self.word(q / 64) >> (q % 64) & 1 != 0
    }

    /// The first stored position at or after q: every position of an image that is not compacted, else the
    /// live ones. `None` past the last.
    fn next_stored(&self, q: u64) -> Option<u64> {
        if q >= self.p {
            return None;
        }
        if !self.compacted {
            return Some(q);
        }
        self.next_live(q)
    }

    /// The first live position at or after q, a word of the bitmap at a time. `None` past the last.
    fn next_live(&self, q: u64) -> Option<u64> {
        if q >= self.p {
            return None;
        }
        let mut w = q / 64;
        let mut word = self.word(w) & (u64::MAX << (q % 64));
        loop {
            if word != 0 {
                let r = w.wrapping_mul(64).wrapping_add(u64::from(word.trailing_zeros()));
                return if r < self.p { Some(r) } else { None };
            }
            w = w.wrapping_add(1);
            if w.wrapping_mul(64) >= self.p {
                return None;
            }
            word = self.word(w);
        }
    }
}

/// Whether a rewrite of a group of p positions, `live` of them live, writes the compacted form: the one rule
/// of every rewrite (seal, compaction), so the same live rows give the same bytes whatever
/// rewrite writes them. Dead positions reaching `COMPACT_DEAD` are left out with a rank map; fewer are written
/// as nulls.
pub fn compacted_form(p: u64, live: u64) -> bool {
    p.saturating_sub(live) >= COMPACT_DEAD
}

// ---------------------------------------------------------------- the rows' values

/// A null's `Fixed` cell: zeros, as long as the longest (`FIXED_MAX` bytes).
static FIXED_ZEROS: [u8; 1_016] = [0; 1_016];
const _: () = assert!(FIXED_MAX == 1_016);

/// Where the group's rows are read from.
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
/// bytes, told by the reference's image length. A shorter long item is placed by its length and copied like the
/// same value inline, so the image does not depend on the rows' history.
fn adopts(img: &Image) -> bool {
    img.length > HUGE_IMAGE
}

/// A leaf's offsets in a row image: its fixed cell, its item table index and its bitmap bit.
type Offsets = (u64, u64, u64);

/// What a rewrite's reads keep between values: its work space and the row layouts built there, the last
/// row layout looked up and the last leaf's offsets in it, and a sealed source's leaf (its patch unit and its
/// directory entry).
struct Cx {
    ws: Space,
    vs: Versions,
    lay: Option<versions::Entry>,
    lay_head: RowHead,
    lay_at: Option<(u64, Offsets)>,
    src_leaf: Option<(u64, Option<patch::Unit>, group::LeafDir)>,
}

impl Cx {
    /// Leaf `o`'s offsets in a row of schema version v, the version's layout then in `lay_head`; `None` when
    /// the version's row layout is not built yet (`entry` builds it).
    #[inline(always)]
    fn offsets<H: PageHost + ?Sized>(&mut self, h: &H, v: u32, o: u64) -> Result<Option<Offsets>, Fault> {
        if let (Some(e), Some((k, at))) = (&self.lay, self.lay_at) {
            if e.version == v && k == o {
                return Ok(Some(at));
            }
        }
        let e = match self.lay {
            Some(e) if e.version == v => e,
            _ => {
                self.lay_at = None;
                let Some(e) = self.vs.find(v) else { return Ok(None) };
                self.lay = Some(e);
                self.lay_head = e.head();
                e
            }
        };
        let at = versions::offsets(h, &self.ws, &e, o)?;
        self.lay_at = Some((o, at));
        Ok(Some(at))
    }

    /// The row layout of schema version v, built when the rewrite has none yet.
    fn entry<H: PageHost>(&mut self, h: &mut H, shape: &Shape, v: u32) -> Result<versions::Entry, Fault> {
        if let Some(e) = self.vs.find(v) {
            return Ok(e);
        }
        // A build may take the room of another version's table.
        self.lay = None;
        self.lay_at = None;
        self.vs.build(h, shape, &mut self.ws, v)
    }
}

/// Leaf `l`'s value at position q (`Null` for a position the source holds no row at).
fn value<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    src: &Source,
    l: &LeafInfo,
    cx: &mut Cx,
    q: u64,
) -> Result<SVal, Fault> {
    let o = u64::from(l.ordinal);
    match *src {
        Source::Buffered { desc } => {
            let Some((block, slot)) = buffer::locate_in(&*h, desc, q)? else { return Ok(SVal::Null) };
            let Some(loc) = buffer::slot_row(&*h, block, slot)? else { return Ok(SVal::Null) };
            loop {
                let r = RowRef::new(&*h, loc)?;
                let v = r.version()?;
                if let Some((f, vi, b)) = cx.offsets(&*h, v, o)? {
                    let head = cx.lay_head;
                    let s = row::stored_at(&head, &r, l, f, vi, b)?;
                    let s = cur::from_stored_in(&r, &head, l, &s, vi)?;
                    // A long item a rewrite does not adopt (`adopts`) is read as bytes of its long value, so it
                    // is sized and placed by its length as the same value inline would be.
                    return Ok(match s {
                        SVal::Huge { img, len, .. } if !adopts(&img) => SVal::B { at: cur::At::Long { img }, len },
                        s => s,
                    });
                }
                cx.entry(h, shape, v)?;
            }
        }
        Source::Sealed { meta, dir } => {
            let gv = GroupView::from_meta(&*h, &meta);
            let (unit, ld) = match cx.src_leaf {
                Some((k, u, d)) if k == o => (u, d),
                _ => {
                    let u = patch::unit(&*h, dir, o)?;
                    let d = gv.leaf(o)?;
                    cx.src_leaf = Some((o, u, d));
                    (u, d)
                }
            };
            if let Some(u) = unit {
                if let Some(e) = patch::lookup_in(&*h, &u, q)? {
                    return cur::from_entry(&*h, &e, l);
                }
            }
            let Some(s) = gv.slot(q)? else { return Ok(SVal::Null) };
            cur::from_cell(&gv, &ld, l, s, q)
        }
    }
}

/// Leaf `l`'s value at stored position q: `Null` for a dead position.
fn live_value<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    l: &LeafInfo,
    cx: &mut Cx,
    q: u64,
) -> Result<SVal, Fault> {
    if rw.is_live(q) {
        value(h, shape, src, l, cx, q)
    } else {
        Ok(SVal::Null)
    }
}

/// Leaf `l`'s place in the rows of a seal, for the in-place walks: a leaf of a kind they read, in rows of the
/// version the seal writes. `None` when the walks do not read the leaf.
fn cut_of<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    cx: &mut Cx,
    l: &LeafInfo,
) -> Result<Option<walk::Cut>, Fault> {
    if rw.kind != SEAL || walk::Kind::of(l.ty).is_none() {
        return Ok(None);
    }
    let e = cx.entry(h, shape, rw.version)?;
    let Ok((f, vi, bit)) = versions::offsets(&*h, &cx.ws, &e, u64::from(l.ordinal)) else { return Ok(None) };
    Ok(walk::Cut::new(l.ty, &e.head(), f, vi, bit))
}

/// Where the in-place walk takes up again after positions it left to the general path: the general path
/// reads the positions (or slots) up to `until`. A walk that stopped before reading any doubles that stretch,
/// up to `BACK_MAX`, so that rows the walk cannot read (moved rows, long items) cost the walk little; a walk
/// that read some sets it back to one.
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

// ---------------------------------------------------------------- plans in the work space

const PLAN: u64 = 80;

#[derive(Clone, Copy, Debug, Default)]
struct Plan {
    form: u8,
    flags: u8,
    width: u64,
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

fn load_plan<H: PageHost + ?Sized>(h: &H, ws: &Space, o: u64) -> Result<Plan, Fault> {
    let at = ws.locator(h, o)?;
    if at == 0 {
        return Err(rt(Corruption::Link, o));
    }
    let mut b = [0u8; 80];
    ws.read_bytes(h, at, &mut b)?;
    Plan::from_bytes(&b)
}

fn store_plan<H: PageHost>(h: &mut H, ws: &mut Space, o: u64, pl: &Plan) -> Result<(), Fault> {
    let mut at = ws.locator(&*h, o)?;
    if at == 0 {
        at = ws.append(h, PLAN)?;
        ws.set_locator(h, o, at)?;
    }
    ws.write_bytes(h, at, &pl.encode()?)
}

/// The leaves a rewrite writes, in ordinal order: `f(ordinal, leaf)` for each.
fn for_each_leaf<H: PageHost, F>(h: &mut H, shape: &Shape, mut f: F) -> Result<(), Fault>
where
    F: FnMut(&mut H, u64, &LeafInfo) -> Result<(), Fault>,
{
    for o in 0..shape.leaves {
        f(h, o, &shape.leaf(&*h, o)?)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- pass 1

/// Pass 1's accumulators of one leaf: what its form and sizes follow from. An integer leaf's largest value
/// (unsigned) and its smallest and largest (signed), and its `Seq` test; the sizes of a variable leaf's values.
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

impl Acc {
    fn new() -> Acc {
        Acc { flags: A_SEQ_OK, imin: i64::MAX, imax: i64::MIN, ..Acc::default() }
    }

    fn has(&self, f: u64) -> bool {
        self.flags & f != 0
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

/// Pass 1, leaf by leaf: each written leaf's plan. Returns the leaves written.
fn pass1<H: PageHost>(h: &mut H, shape: &Shape, rw: &Rw, src: &Source, cx: &mut Cx) -> Result<u64, Fault> {
    let mut leaves = 0u64;
    for_each_leaf(h, shape, |h, o, l| {
        let acc = scan(h, shape, rw, src, cx, l)?;
        store_plan(h, &mut cx.ws, o, &plan_of(l, &acc))?;
        leaves = leaves.wrapping_add(1);
        Ok(())
    })?;
    Ok(leaves)
}

/// Pass 1's scan of leaf `l` over the stored positions: each value read and fed to the accumulators (its
/// leaf's range, `Seq` test and sizes), by the in-place walk where it reads the leaf.
fn scan<H: PageHost>(h: &mut H, shape: &Shape, rw: &Rw, src: &Source, cx: &mut Cx, l: &LeafInfo) -> Result<Acc, Fault> {
    let t = l.ty;
    let mut acc = Acc::new();
    let cut = cut_of(h, shape, rw, cx, l)?;
    let int = matches!(t, Ty::U(_) | Ty::I(_) | Ty::Nat | Ty::Int);
    let mut back = Back::default();
    let mut q = 0u64;
    loop {
        if let Some(cut) = cut.as_ref() {
            if back.walks(q) {
                let (pc, qq) = walk::scan(&*h, rw, cut, &mut acc, q);
                q = qq;
                if pc.end {
                    break;
                }
                back.slow(q, pc.read);
                continue;
            }
        }
        let Some(qq) = rw.next_stored(q) else { break };
        q = qq;
        if !rw.is_live(q) {
            acc.flags |= A_STORED_NULL;
            q = q.wrapping_add(1);
            continue;
        }
        let s = value(h, shape, src, l, cx, q)?;
        q = q.wrapping_add(1);
        if s.is_null() {
            acc.flags |= A_STORED_NULL;
            acc.flags &= !A_SEQ_OK;
            continue;
        }
        let qv = q.wrapping_sub(1);
        if int {
            match s {
                SVal::V(Val::U(x)) => {
                    acc.umax = acc.umax.max(x);
                    seq_add(&mut acc, qv, i128::from(x));
                    if let Ok(y) = i64::try_from(x) {
                        acc.imin = acc.imin.min(y);
                        acc.imax = acc.imax.max(y);
                    }
                }
                SVal::V(Val::I(x)) => {
                    acc.imin = acc.imin.min(x);
                    acc.imax = acc.imax.max(x);
                    seq_add(&mut acc, qv, i128::from(x));
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
    }
    Ok(acc)
}

/// A leaf's form and plan, from its accumulators.
fn plan_of(l: &LeafInfo, acc: &Acc) -> Plan {
    let t = l.ty;
    let leb_form = matches!(t, Ty::Nat | Ty::Int) && acc.has(A_ANY_BIG);
    let f = match t {
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
        // A dropped leaf keeps its directory entry, with no cells: it reads as null.
        Ty::Dropped => return Plan { form: form::UNIT, ..Plan::default() },
    };
    let mut pl = Plan { form: f, ..Plan::default() };
    // A null clears the `Seq` test, so a leaf that passes it holds no null.
    if matches!(t, Ty::U(_) | Ty::I(_) | Ty::Nat | Ty::Int) && !leb_form {
        let (lo, hi) = int_range(t);
        if let Some((base, step)) = seq_params(acc, lo, hi) {
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
    pl
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
            let room = crate::consts::PAGE.wrapping_sub(self.c % crate::consts::PAGE);
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
                if room == crate::consts::PAGE {
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
/// leaf's cell cursor, while `f` returns `true`; returns the cursor after its parts.
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

/// Where the image's parts lie, and its totals.
#[derive(Clone, Copy, Default)]
struct Lay {
    leaves: u64,
    rank_at: u64,
    hh: u64,
    inline_at: u64,
    ool_start: u64,
    ool_end: u64,
    segs_at: u64,
    total: u64,
    s: u64,
    x: u64,
    v: u64,
}

/// The layout: each leaf's segments from the cell cursor, its inline and out-of-line offsets; then the totals.
fn layout<H: PageHost>(h: &mut H, shape: &Shape, rw: &Rw, cx: &mut Cx, leaves: u64) -> Result<Lay, Fault> {
    let fr = group::HEADER.wrapping_add(leaves.wrapping_mul(group::LD_BYTES));
    let mut lay = Lay { leaves, ..Lay::default() };
    lay.rank_at = if rw.compacted { fr } else { 0 };
    lay.hh = group::align8(if rw.compacted { fr.wrapping_add(group::RANK_MAP) } else { fr });
    let (mut c, mut run, mut oolt) = (lay.hh, 0u64, 0u64);
    for_each_leaf(h, shape, |h, o, _| {
        let mut pl = load_plan(&*h, &cx.ws, o)?;
        pl.cell_at = c;
        pl.first_seg = lay.s;
        pl.run_rel = run;
        pl.ool_rel = oolt;
        let mut count = 0u64;
        let end = leaf_segments(&pl, rw.n, 0, |_| {
            count = count.wrapping_add(1);
            Ok(true)
        })?;
        pl.segs = count;
        store_plan(h, &mut cx.ws, o, &pl)?;
        c = end;
        lay.s = lay.s.checked_add(count).ok_or(rt(Corruption::Range, count))?;
        run = run.checked_add(pl.run).ok_or(rt(Corruption::Range, run))?;
        oolt = oolt.checked_add(pl.ool_bytes).ok_or(rt(Corruption::Range, oolt))?;
        lay.x = lay.x.checked_add(pl.ool_k).ok_or(rt(Corruption::Range, lay.x))?;
        lay.v = lay.v.checked_add(pl.huge).ok_or(rt(Corruption::Range, lay.v))?;
        Ok(())
    })?;
    lay.inline_at = c;
    lay.ool_start = c.checked_add(run).ok_or(rt(Corruption::Range, c))?;
    lay.ool_end = lay.ool_start.checked_add(oolt).ok_or(rt(Corruption::Range, lay.ool_start))?;
    lay.segs_at = lay.ool_end.checked_add(31).ok_or(rt(Corruption::Range, lay.ool_end))? & !31;
    lay.total =
        lay.segs_at.checked_add(lay.s.wrapping_mul(group::SEG_BYTES)).ok_or(rt(Corruption::Range, lay.segs_at))?;
    Ok(lay)
}

// ---------------------------------------------------------------- the image

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

/// The image being written in order, staged 4 KiB at a time.
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

    fn put<H: PageHost>(&mut self, h: &mut H, b: &[u8]) -> Result<(), Fault> {
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

    /// Zeros up to image offset `off`.
    fn pad_to<H: PageHost>(&mut self, h: &mut H, off: u64) -> Result<(), Fault> {
        let at = self.pos();
        if off < at {
            return Err(rt(Corruption::Layout, off));
        }
        let mut left = off.wrapping_sub(at);
        let zeros = [0u8; 512];
        while left > 0 {
            let n = left.min(512);
            self.put(h, zeros.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))?;
            left = left.wrapping_sub(n);
        }
        Ok(())
    }
}

/// The image's header.
fn header_bytes<H: PageHost + ?Sized>(h: &H, tbl: u64, shape: &Shape, rw: &Rw, lay: &Lay) -> Result<[u8; 128], Fault> {
    let uid = root::u64_at(h, tbl, root::UID)?;
    let mut hd = [0u8; 128];
    crate::schema::put_common_header(&mut hd, group::KIND_GROUP, 0, lay.total, uid, shape.hash)?;
    let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
    let w32 = |x: u64| u32::try_from(x).map_err(|_| rt(Corruption::Range, x));
    put_u64(&mut hd, group::G_GROUP, rw.g)?;
    put_u16(&mut hd, group::G_P, w16(rw.p)?)?;
    put_u16(&mut hd, group::G_N, w16(rw.n)?)?;
    put_u16(&mut hd, group::G_L, w16(lay.leaves)?)?;
    let mut flags = 0u16;
    if rw.compacted {
        flags |= GF_COMPACTED;
    }
    if lay.x > 0 {
        flags |= GF_OOL;
    }
    if lay.v > 0 {
        flags |= GF_HUGE;
    }
    put_u16(&mut hd, group::G_FLAGS, flags)?;
    put_u32(&mut hd, group::G_VERSION, rw.version)?;
    put_u32(&mut hd, group::G_S, w32(lay.s)?)?;
    put_u32(&mut hd, group::G_X, w32(lay.x)?)?;
    put_u32(&mut hd, group::G_H, w32(lay.hh)?)?;
    put_u32(&mut hd, group::G_RANK, w32(lay.rank_at)?)?;
    put_u32(&mut hd, group::G_DIR, w32(group::HEADER)?)?;
    put_u64(&mut hd, group::G_OOL_AT, lay.ool_start)?;
    put_u64(&mut hd, group::G_OOL_BYTES, lay.ool_end.wrapping_sub(lay.ool_start))?;
    put_u32(&mut hd, group::G_V, w32(lay.v)?)?;
    put_u64(&mut hd, group::G_SEGS, lay.segs_at)?;
    put_u64(&mut hd, group::G_FIRST_HUGE, 0)?;
    Ok(hd)
}

/// The image, in order; returns it and the new long-value list.
#[allow(clippy::too_many_arguments)]
fn write<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    cx: &mut Cx,
    lay: &Lay,
) -> Result<(Image, page::LvBuilder), Fault> {
    let mut out = Out { img: Image::EMPTY, tbl, stage: [0; 4_096], staged: 0 };
    let mut lv = page::LvBuilder::new();
    out.put(h, &header_bytes(&*h, tbl, shape, rw, lay)?)?;
    // The leaf directory.
    for_each_leaf(h, shape, |h, o, l| {
        let pl = load_plan(&*h, &cx.ws, o)?;
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
        out.put(h, &e)
    })?;
    // The rank map: the live bitmap, then each word's rank a u16.
    if rw.compacted {
        let mut bm = [0u8; 512];
        let mut ranks = [0u8; 128];
        let mut rank = 0u32;
        for (wd, word) in (0u64..).zip(rw.bits.iter()) {
            put_u64(&mut bm, wd.wrapping_mul(8), *word)?;
            put_u16(&mut ranks, wd.wrapping_mul(2), u16::try_from(rank).unwrap_or(0))?;
            rank = rank.wrapping_add(word.count_ones());
        }
        out.put(h, &bm)?;
        out.put(h, &ranks)?;
    }
    out.pad_to(h, lay.hh)?;
    // The cell area, each leaf's ending where the layout placed the end of its last segment.
    for_each_leaf(h, shape, |h, o, l| {
        let pl = load_plan(&*h, &cx.ws, o)?;
        cells_leaf(h, shape, rw, src, cx, l, &pl, &mut out)?;
        if out.pos() != leaf_segments(&pl, rw.n, 0, |_| Ok(true))? {
            return Err(rt(Corruption::Layout, out.pos()));
        }
        Ok(())
    })?;
    // The inline data region, then the out-of-line area.
    for ool in [false, true] {
        if (!ool && lay.ool_start == lay.inline_at) || (ool && lay.x == 0) {
            continue;
        }
        for_each_leaf(h, shape, |h, o, l| {
            let pl = load_plan(&*h, &cx.ws, o)?;
            if (!ool && !group::variable(pl.form)) || (ool && pl.ool_k == 0) {
                return Ok(());
            }
            out.pad_to(
                h,
                if ool { lay.ool_start.wrapping_add(pl.ool_rel) } else { lay.inline_at.wrapping_add(pl.run_rel) },
            )?;
            values_leaf(h, tbl, shape, rw, src, cx, l, &mut out, &mut lv, ool)
        })?;
    }
    // The segment table.
    out.pad_to(h, lay.segs_at)?;
    for_each_leaf(h, shape, |h, o, _| {
        let pl = load_plan(&*h, &cx.ws, o)?;
        leaf_segments(&pl, rw.n, lay.inline_at.wrapping_add(pl.run_rel), |s| {
            out.put(h, &s.bytes32()?)?;
            Ok(true)
        })?;
        Ok(())
    })?;
    out.flush(h)?;
    if out.img.length != lay.total {
        return Err(rt(Corruption::Count, out.img.length));
    }
    Ok((out.img, lv))
}

/// The cell area of one leaf: its segments in order.
#[allow(clippy::too_many_arguments)]
fn cells_leaf<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    cx: &mut Cx,
    l: &LeafInfo,
    pl: &Plan,
    out: &mut Out,
) -> Result<(), Fault> {
    let cut = cut_of(h, shape, rw, cx, l)?;
    // The positions read so far, and a variable leaf's running offset in its run (`acc`), across segments.
    let mut c = walk::Cur { slot: 0, q: 0, word: 0, acc: 0 };
    leaf_segments(pl, rw.n, 0, |sg| {
        if sg.kind == seg::RUN {
            return Ok(false);
        }
        out.pad_to(h, sg.off)?;
        match sg.kind {
            seg::CELLS => cell_segment(h, shape, rw, src, cx, l, pl, &sg, out, &mut c, cut.as_ref())?,
            seg::SEQ => {
                out.put(h, &pl.seq_base.to_le_bytes())?;
                out.put(h, &pl.seq_step.to_le_bytes())?;
            }
            seg::OOL => ool_dir(h, shape, rw, src, cx, l, pl, &sg, out, cut.as_ref())?,
            _ => {}
        }
        Ok(true)
    })?;
    Ok(())
}

/// One cell segment: its validity words, then its cells, each slot by the in-place walk where it reads the
/// leaf's form, else by the general path.
#[allow(clippy::too_many_arguments)]
fn cell_segment<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    cx: &mut Cx,
    l: &LeafInfo,
    pl: &Plan,
    sg: &group::Seg,
    out: &mut Out,
    c: &mut walk::Cur,
    cut: Option<&walk::Cut>,
) -> Result<(), Fault> {
    let t = l.ty;
    let k = u64::from(sg.positions);
    let seg_q = c.q;
    if pl.validity() {
        // The word under construction waits in the cursor until its 64th slot or the segment's end.
        c.slot = 0;
        c.word = 0;
        let mut back = Back::default();
        while c.slot < k {
            if let Some(cut) = cut {
                if back.walks(c.slot) {
                    let pc = walk_write(h, rw, cut, walk::Part::Validity, pl, c, k, out)?;
                    if pc.slow {
                        back.slow(c.slot, pc.read);
                    }
                    continue;
                }
            }
            let q = rw.next_stored(c.q).ok_or(rt(Corruption::Count, c.slot))?;
            let s = live_value(h, shape, rw, src, l, cx, q)?;
            c.word |= u64::from(!s.is_null()) << (c.slot % 64);
            c.slot = c.slot.wrapping_add(1);
            c.q = q.wrapping_add(1);
            if c.slot % 64 == 0 || c.slot == k {
                out.put(h, &c.word.to_le_bytes())?;
                c.word = 0;
            }
        }
    }
    c.slot = 0;
    c.q = seg_q;
    let body = out.pos();
    let part = cut.and_then(|ct| walk_part(ct, pl));
    let mut back = Back::default();
    while c.slot < k {
        if let (Some(cut), Some(part)) = (cut, part) {
            if back.walks(c.slot) {
                let pc = walk_write(h, rw, cut, part, pl, c, k, out)?;
                if pc.slow {
                    back.slow(c.slot, pc.read);
                }
                continue;
            }
        }
        let q = rw.next_stored(c.q).ok_or(rt(Corruption::Count, c.slot))?;
        let s = live_value(h, shape, rw, src, l, cx, q)?;
        let i = c.slot;
        match pl.form {
            form::BOOL => {
                // The word under construction waits in the cursor until its 64th position or the segment's end.
                c.acc |= u64::from(matches!(s, SVal::V(Val::Bool(true)))) << (i % 64);
                if i % 64 == 63 || i.wrapping_add(1) == k {
                    out.put(h, &c.acc.to_le_bytes())?;
                    c.acc = 0;
                }
            }
            form::UNIT => {}
            f if group::variable(f) => {
                out.put(h, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
                if let Place::Inline(len) = place_of(&s, t) {
                    c.acc = c.acc.wrapping_add(len);
                }
                if i.wrapping_add(1) == k {
                    out.put(h, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
                }
            }
            form::FIXED => {
                // A null's cell is zeros.
                let w = pl.width;
                if s.is_null() {
                    let z = FIXED_ZEROS.get(..usize::try_from(w).unwrap_or(0)).ok_or(rt(Corruption::Range, w))?;
                    out.put(h, z)?;
                } else if matches!(t, Ty::Principal) {
                    out.put(h, &cur::principal_cell(&*h, &s)?)?;
                } else {
                    // The cell's bytes in one read: a `Fixed` form holds at most `FIXED_MAX` of them.
                    let mut b = [0u8; 1_016];
                    let bk = b.get_mut(..usize::try_from(w).unwrap_or(usize::MAX)).ok_or(rt(Corruption::Range, w))?;
                    cur::read(&*h, &s, 0, bk)?;
                    out.put(h, bk)?;
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
                out.put(h, b.get(..usize::try_from(pl.width).unwrap_or(0)).unwrap_or(&[]))?;
            }
        }
        c.slot = c.slot.wrapping_add(1);
        c.q = q.wrapping_add(1);
    }
    out.pad_to(h, body.wrapping_add(group::cell_bytes(pl.form, pl.width, false, k)))
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

/// The in-place walk writing `part` of leaf `pl`'s segment, slots `c.slot` to k − 1, through the output's
/// stage, which is written out when the walk found it full.
#[allow(clippy::too_many_arguments)]
fn walk_write<H: PageHost>(
    h: &mut H,
    rw: &Rw,
    cut: &walk::Cut,
    part: walk::Part,
    pl: &Plan,
    c: &mut walk::Cur,
    k: u64,
    out: &mut Out,
) -> Result<walk::Piece, Fault> {
    let width = usize::try_from(pl.width).unwrap_or(0);
    let mut st = walk::Stage { buf: &mut out.stage, at: out.staged };
    let pc = walk::write(&*h, rw, cut, part, pl.form, width, c, k, &mut st);
    out.staged = st.at;
    if pc.full {
        out.flush(h)?;
    }
    Ok(pc)
}

/// The out-of-line directory: its bitmap over the stored slots, its rank, its offsets.
#[allow(clippy::too_many_arguments)]
fn ool_dir<H: PageHost>(
    h: &mut H,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    cx: &mut Cx,
    l: &LeafInfo,
    pl: &Plan,
    sg: &group::Seg,
    out: &mut Out,
    cut: Option<&walk::Cut>,
) -> Result<(), Fault> {
    let n = rw.n;
    let t = l.ty;
    let ool = |s: &SVal| matches!(place_of(s, t), Place::Medium(_) | Place::Huge);
    // The in-place walk reads a Text or Blob leaf's slots; the general path the others.
    let cut = cut.filter(|ct| ct.kind == walk::Kind::Bytes && matches!(pl.form, form::UTF8 | form::BINARY));
    // The bitmap, every stored slot from the first: the word under construction waits in the cursor until its
    // 64th slot or the last.
    let mut c = walk::Cur { slot: 0, q: 0, word: 0, acc: 0 };
    let mut back = Back::default();
    while c.slot < n {
        if let Some(cut) = cut {
            if back.walks(c.slot) {
                let pc = walk_write(h, rw, cut, walk::Part::OolBits, pl, &mut c, n, out)?;
                if pc.slow {
                    back.slow(c.slot, pc.read);
                }
                continue;
            }
        }
        let q = rw.next_stored(c.q).ok_or(rt(Corruption::Count, c.slot))?;
        let s = live_value(h, shape, rw, src, l, cx, q)?;
        c.word |= u64::from(ool(&s)) << (c.slot % 64);
        c.slot = c.slot.wrapping_add(1);
        c.q = q.wrapping_add(1);
        if c.slot % 64 == 0 || c.slot == n {
            out.put(h, &c.word.to_le_bytes())?;
            c.word = 0;
        }
    }
    // One rank per 4,096 slots: the out-of-line values before them. A group has at most 4,096 slots, so the one
    // rank is 0.
    for _ in 0..n.div_ceil(4_096) {
        out.put(h, &0u32.to_le_bytes())?;
    }
    // The offsets: the first, then one per value out of line, from the leaf's place in the out-of-line area.
    c = walk::Cur { slot: 0, q: 0, word: 0, acc: pl.ool_rel };
    out.put(h, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
    back = Back::default();
    while c.slot < n {
        if let Some(cut) = cut {
            if back.walks(c.slot) {
                // The slots whose values lie inline write nothing: the walk passes them.
                let pc = walk::ool_offsets(&*h, rw, cut, &mut c, n);
                if pc.slow {
                    back.slow(c.slot, pc.read);
                }
                continue;
            }
        }
        let q = rw.next_stored(c.q).ok_or(rt(Corruption::Count, c.slot))?;
        let s = live_value(h, shape, rw, src, l, cx, q)?;
        let len = match place_of(&s, t) {
            Place::Medium(len) => Some(len),
            Place::Huge => Some(8),
            _ => None,
        };
        if let Some(len) = len {
            c.acc = c.acc.wrapping_add(len);
            out.put(h, &u32::try_from(c.acc).map_err(|_| rt(Corruption::Range, c.acc))?.to_le_bytes())?;
        }
        c.slot = c.slot.wrapping_add(1);
        c.q = q.wrapping_add(1);
    }
    out.pad_to(h, sg.off.wrapping_add(u64::from(sg.bytes)))
}

/// A leaf's values in the inline data region (`ool` false) or the out-of-line area, each live position's in
/// order: a value copied in pieces, or a huge value's reference moved into the new long-value list (`lv`), its
/// index written and its old referent cleared.
#[allow(clippy::too_many_arguments)]
fn values_leaf<H: PageHost>(
    h: &mut H,
    tbl: u64,
    shape: &Shape,
    rw: &Rw,
    src: &Source,
    cx: &mut Cx,
    l: &LeafInfo,
    out: &mut Out,
    lv: &mut page::LvBuilder,
    ool: bool,
) -> Result<(), Fault> {
    let t = l.ty;
    // The in-place walk copies a Text or Blob leaf's inline values whole, and passes them in the out-of-line
    // area; the general path takes the others.
    let cut = cut_of(h, shape, rw, cx, l)?.filter(|ct| ct.kind == walk::Kind::Bytes);
    let mut back = Back::default();
    let mut q = 0u64;
    loop {
        if let Some(cut) = cut.as_ref() {
            if back.walks(q) {
                let mut cur = walk::Cur { slot: 0, q, word: 0, acc: 0 };
                let mut st = walk::Stage { buf: &mut out.stage, at: out.staged };
                let pc = walk::run(&*h, rw, cut, &mut cur, &mut st, ool);
                out.staged = st.at;
                q = cur.q;
                if pc.full {
                    out.flush(h)?;
                }
                if pc.end {
                    return Ok(());
                }
                if pc.slow {
                    back.slow(q, pc.read);
                }
                continue;
            }
        }
        // A dead position holds no value: on to the next live one, a bitmap word at a time.
        let Some(qq) = rw.next_live(q) else { return Ok(()) };
        q = qq;
        let s = value(h, shape, src, l, cx, q)?;
        match (place_of(&s, t), ool) {
            (Place::Inline(len), false) | (Place::Medium(len), true) => {
                let mut piece = [0u8; 1_024];
                let mut at = 0u64;
                while at < len {
                    let k = len.wrapping_sub(at).min(1_024);
                    let p = piece.get_mut(..usize::try_from(k).unwrap_or(0)).unwrap_or(&mut []);
                    value_bytes(&*h, &s, t, at, p)?;
                    out.put(h, p)?;
                    at = at.wrapping_add(k);
                }
            }
            (Place::Huge, true) => {
                let SVal::Huge { img, owner, .. } = s else { return Err(rt(Corruption::Link, q)) };
                let i = lv.push(h, tbl, &img)?;
                out.put(h, &i.to_le_bytes())?;
                // The new list is the value's one referent now: what the group's old structures free leaves it.
                forget(h, owner)?;
            }
            _ => {}
        }
        q = q.wrapping_add(1);
    }
}

/// Clears a huge value's old referent.
fn forget<H: PageHost>(h: &mut H, owner: cur::Owner) -> Result<(), Fault> {
    match owner {
        cur::Owner::Item { loc, at } => match loc {
            RowLoc::Inline { block, off, .. } => fill(h.get_mut(Region::Block(block))?, off.wrapping_add(at), 24, 0),
            RowLoc::Long(img) => super::long::write_at(h, img, at, &[0u8; 24]),
        },
        cur::Owner::List { page, index } => {
            let (b, off) = page::lv_entry(&*h, page, index)?;
            fill(h.get_mut(Region::Block(b))?, off, crate::image::REF_BYTES, 0)
        }
        cur::Owner::Patch { block, data } => patch::forget_long(h, block, data),
    }
}

// ---------------------------------------------------------------- the rewrite

// The liveness reads slot entries of 8 bytes, flags at 4.
const _: () = assert!(buffer::SLOT == 8);

/// A seal's liveness: each block record's slots in order, a live slot's bit set; the live count, the compacted
/// form, and whether the records rise strictly, each first position its block's.
fn liveness<H: PageHost + ?Sized>(h: &H, rw: &mut Rw) -> Result<(), Fault> {
    let desc = rw.source;
    let blocks = buffer::u16_of(h, desc, buffer::D_BLOCKS)?.min(BUFFER_BLOCKS_MAX);
    let mut slots = 0u64;
    let mut apart = false;
    for k in 0..blocks {
        let rec = buffer::D_RECORDS.wrapping_add(k.wrapping_mul(16));
        let blk_ref = buffer::u64_of(h, desc, rec)?;
        let blk = h.get(Region::Block(blk_ref))?;
        if rd_u32(blk, 0)? != magic::NTBF || rd_u64(blk, buffer::B_GROUP)? != rw.g {
            return Err(rt(Corruption::Header, blk_ref));
        }
        if u64::from(rd_u16(blk, buffer::B_FIRST)?) != slots {
            return Err(rt(Corruption::Order, blk_ref));
        }
        // The slot array grows down from the block's end: slot 0 is its last entry.
        let n = u64::from(rd_u16(blk, buffer::B_SLOTS)?);
        if buffer::u16_of(h, desc, rec.wrapping_add(8))? != slots || n == 0 {
            apart = true;
        }
        let array =
            crate::mem::sub(blk, BLOCK.wrapping_sub(n.wrapping_mul(buffer::SLOT)), n.wrapping_mul(buffer::SLOT))?;
        let mut q = slots;
        for e in array.chunks_exact(8).rev() {
            if e.get(4).copied().unwrap_or(SLOT_DEAD) & SLOT_DEAD == 0 {
                if let Some(w) = rw.bits.get_mut(usize::try_from(q / 64).unwrap_or(usize::MAX)) {
                    *w |= 1 << (q % 64);
                }
            }
            q = q.wrapping_add(1);
        }
        slots = slots.wrapping_add(n);
    }
    if slots != rw.p {
        return Err(rt(Corruption::Count, desc));
    }
    rw.live = rw.bits.iter().map(|w| u64::from(w.count_ones())).sum::<u64>();
    rw.compacted = compacted_form(rw.p, rw.live);
    rw.n = if rw.compacted { rw.live } else { rw.p };
    rw.ordered = !apart;
    Ok(())
}

/// Seals group `desc`, a buffer descriptor the open group has just left (`write::freeze`): its rows' image,
/// written at the table's schema version, on a new group page that the directory names; then the descriptor,
/// its buffer blocks and the long values its rows held that the image does not keep are freed.
pub fn seal<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, desc: u64) -> Result<(), Fault> {
    let (g, p) = {
        let d = h.get(Region::Block(desc))?;
        if rd_u32(d, 0)? != magic::NTBD || rd_u16(d, 4)? > FORMAT_TABLE {
            return Err(rt(Corruption::Header, desc));
        }
        (rd_u64(d, buffer::D_GROUP)?, u64::from(rd_u16(d, buffer::D_POSITIONS)?))
    };
    let mut rw = Rw {
        kind: SEAL,
        g,
        source: desc,
        p,
        n: 0,
        live: 0,
        compacted: false,
        ordered: false,
        version: shape.version,
        dir: 0,
        bits: [0; 64],
    };
    liveness(&*h, &mut rw)?;
    let pg = run(h, tbl, shape, &rw)?;
    // The group's rows leave the buffered live rows (the integer leaves' totals count them still).
    let tail = root::u64_at(&*h, tbl, root::TAIL)?;
    let buffered = buffer::u64_of(&*h, tail, buffer::H_BUFFERED)?;
    put_u64(h.get_mut(Region::Block(tail))?, buffer::H_BUFFERED, buffered.wrapping_sub(rw.live))?;
    super::drain::free_descriptor(h, tbl, desc)?;
    super::jobs::after_rewrite(h, tbl, g, pg)
}

/// Compacts (`COMPACTION`) the sealed group whose page is `pg`:
/// its live rows' image, written at the table's schema version without its dead positions' rows, on a new group
/// page that the directory names; then the old page, its image, its long-value list and its patches are freed.
pub fn rewrite<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, pg: u64, kind: u8) -> Result<(), Fault> {
    let mut rw = Rw {
        kind,
        g: 0,
        source: pg,
        p: 0,
        n: 0,
        live: 0,
        compacted: false,
        ordered: false,
        version: shape.version,
        dir: 0,
        bits: [0; 64],
    };
    {
        let b = h.get(Region::Block(pg))?;
        if rd_u32(b, 0)? != magic::NTGP {
            return Err(rt(Corruption::Header, pg));
        }
        rw.g = rd_u64(b, page::P_GROUP)?;
        rw.p = u64::from(rd_u16(b, page::P_POSITIONS)?);
        rw.live = u64::from(rd_u16(b, page::P_LIVE)?);
        rw.dir = rd_u64(b, page::P_PATCH_DIR)?;
        for (i, w) in (0u64..).zip(rw.bits.iter_mut()) {
            *w = rd_u64(b, page::P_BITMAP.wrapping_add(i.wrapping_mul(8)))?;
        }
    }
    rw.compacted = compacted_form(rw.p, rw.live);
    rw.n = if rw.compacted { rw.live } else { rw.p };
    let new = run(h, tbl, shape, &rw)?;
    super::drain::free_group_page(h, tbl, pg)?;
    super::jobs::after_rewrite(h, tbl, rw.g, new)
}

/// The rewrite itself: pass 1, the layout and the image in a work space of its own, then the new group page,
/// which takes the group's extremes block from the source and which the directory names. Returns the page.
fn run<H: PageHost>(h: &mut H, tbl: u64, shape: &Shape, rw: &Rw) -> Result<u64, Fault> {
    let work = crate::area::alloc_block(h, tbl, 0)?;
    let mut ws = Space::begin(h, tbl, work, rw.g, shape.leaves)?;
    ws.zero_step(h, u64::MAX)?;
    let mut cx = Cx { ws, vs: Versions::new(), lay: None, lay_head: RowHead::default(), lay_at: None, src_leaf: None };
    let src = source(&*h, rw)?;
    let leaves = pass1(h, shape, rw, &src, &mut cx)?;
    let lay = layout(h, shape, rw, &mut cx, leaves)?;
    let (img, lv) = write(h, tbl, shape, rw, &src, &mut cx, &lay)?;
    super::drain::free_list(h, tbl, cx.ws.blocks, 0)?;
    crate::area::free_block(h, tbl, 0, work)?;
    // The new group page.
    let pg = crate::area::alloc_block(h, tbl, 0)?;
    let ext_at = if rw.kind == SEAL { buffer::D_EXT } else { page::P_EXT };
    let ext = rd_u64(h.get(Region::Block(rw.source))?, ext_at)?;
    {
        let w16 = |x: u64| u16::try_from(x).map_err(|_| rt(Corruption::Range, x));
        let b = h.get_mut(Region::Block(pg))?;
        fill(b, 0, page::P_FIELDS, 0)?;
        put_u32(b, 0, magic::NTGP)?;
        put_u16(b, 4, FORMAT_TABLE)?;
        let mut flags = 0u16;
        if rw.compacted {
            flags |= page::PF_COMPACTED;
        }
        if lay.v > 0 {
            flags |= page::PF_HUGE;
        }
        put_u16(b, page::P_FLAGS, flags)?;
        put_u64(b, page::P_GROUP, rw.g)?;
        img.put_record(b, page::P_IMAGE)?;
        put_u16(b, page::P_POSITIONS, w16(rw.p)?)?;
        put_u16(b, page::P_LIVE, w16(rw.live)?)?;
        put_u16(b, page::P_DEAD, w16(rw.n.saturating_sub(rw.live))?)?;
        put_u32(b, page::P_VERSION, rw.version)?;
        put_u32(b, page::P_LV_COUNT, u32::try_from(lv.count).map_err(|_| rt(Corruption::Range, lv.count))?)?;
        put_u64(b, page::P_LV_LIST, lv.head)?;
        for (i, w) in (0u64..).zip(rw.bits.iter()) {
            put_u64(b, page::P_BITMAP.wrapping_add(i.wrapping_mul(8)), *w)?;
        }
        put_u64(b, page::P_EXT, ext)?;
    }
    // The source no longer names the extremes block.
    put_u64(h.get_mut(Region::Block(rw.source))?, ext_at, 0)?;
    root::set_group(h, tbl, rw.g, pg)?;
    Ok(pg)
}
