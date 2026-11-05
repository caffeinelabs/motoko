//! Filtered scans: the live rows whose value of one leaf meets a condition (equal to a value, null, or within
//! two ends, each included, excluded or open), in rid order. A row's value of the leaf is read inside the group
//! the scan has open, under the overlay, and only a matching row is read further.
//!
//! A sealed group is opened once for all its rows. For an integer leaf, the group's extremes record
//! (`extremes`) skips a group whose minimum and maximum the condition rules out, and passes a group whose
//! minimum and maximum both meet it when the leaf cannot be null: a passed group's live rows all match, and
//! their values of the leaf are not read. A loose minimum or maximum still bounds the group's values, so it
//! skips and passes as an exact one does. A group that a value beyond i128 joined, or whose block does not
//! cover the leaf, is tested row by row, as are the groups of every other leaf and every buffered row.
//!
//! Four answers: the matching rows handed to a sink, at most a given number a call, with the rid to go on
//! from (`scan`); the same with only the leaves asked for, every row when no filter is given (`cells`); how
//! many rows match (`count`); and the exact sum and the non-null count of an integer leaf over the matching
//! rows (`sum`). A count and a sum read no other cell of a row, and `cells` reads no leaf it does not hand out
//! besides the filter's.

use core::cmp::Ordering;
use core::ops::Bound;

use crate::consts::GROUP_SLOTS;
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{rd_array, PageHost, Region};
use crate::stats::I192;

use super::buffer::{self, DESC_BIT};
use super::cell::{self, Val};
use super::cur::{self, SVal};
use super::extremes::{self, BIG, HAS};
use super::group::{GroupView, LeafDir};
use super::layout::{LeafInfo, Shape, Ty};
use super::overlay::{self, Buffered, Sealed};
use super::read::{self, Live};
use super::totals::{self, Total};
use super::{page, root, CellIn, RowSink};

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// What a filtered scan asks of its leaf's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cond<'a> {
    /// Equal to the cell; `CellIn::Null` asks for null (an option leaf).
    Eq(CellIn<'a>),
    /// Not null, and within the low and the high end, each included, excluded or open (`Unbounded`).
    Range(Bound<CellIn<'a>>, Bound<CellIn<'a>>),
}

/// The rows whose value of leaf `leaf` meets `cond`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filter<'a> {
    pub leaf: u32,
    pub cond: Cond<'a>,
}

// ---------------------------------------------------------------- values as a filter compares them

/// A non-null value as a filter compares it.
#[derive(Clone, Copy, Debug)]
enum Pt<'a> {
    /// An integer within i128 (any u128), or a Char: its rank (`totals::rank_of_int`), which orders a leaf's
    /// values as the extremes records do.
    Int(u128),
    /// A Nat or Int beyond i128: its canonical LEB128 form, and whether it is negative.
    Big(&'a [u8], bool),
    Bool(bool),
    Float(f64),
    /// Text, Blob, Principal and `[u8; N]`, in byte order.
    Bytes(&'a [u8]),
    Unit,
}

/// Whether the canonical LEB128 form `b` of a value of leaf `l` is negative.
fn negative(l: &LeafInfo, b: &[u8]) -> bool {
    l.ty == Ty::Int && b.last().is_some_and(|&x| x & 0x40 != 0)
}

/// A checked non-null value of leaf `l` as a filter compares it.
fn pt<'a>(l: &LeafInfo, v: Val<'a>) -> Result<Pt<'a>, Fault> {
    Ok(match v {
        Val::U(x) => Pt::Int(totals::rank_of_int(l.ty, i128::from(x))),
        Val::I(x) => Pt::Int(totals::rank_of_int(l.ty, i128::from(x))),
        Val::Wide(x) | Val::I128(x) => Pt::Int(totals::rank_of_int(l.ty, x)),
        Val::U128(x) => Pt::Int(x),
        Val::Big(b) => Pt::Big(b, negative(l, b)),
        Val::Bool(b) => Pt::Bool(b),
        Val::F32(b) => Pt::Float(f64::from(f32::from_bits(b))),
        Val::F64(b) => Pt::Float(f64::from_bits(b)),
        Val::Bytes(b) => Pt::Bytes(b),
        Val::Unit => Pt::Unit,
        Val::Null | Val::Arg(_) => return Err(Fault::BadValue { leaf: l.id }),
    })
}

/// The order of two canonical LEB128 values beyond i128 of `la` and `lb` bytes, negative or not, byte j of
/// each read by `a` and `b`: by sign, then by length (a longer one is further from zero), then by their 7-bit
/// groups from the most significant.
fn big_order(
    (la, na): (u64, bool),
    (lb, nb): (u64, bool),
    mut a: impl FnMut(u64) -> Result<u8, Fault>,
    mut b: impl FnMut(u64) -> Result<u8, Fault>,
) -> Result<Ordering, Fault> {
    if na != nb {
        return Ok(if na { Ordering::Less } else { Ordering::Greater });
    }
    if la != lb {
        let o = la.cmp(&lb);
        return Ok(if na { o.reverse() } else { o });
    }
    let mut j = la;
    while j > 0 {
        j = j.wrapping_sub(1);
        let (x, y) = (a(j)? & 0x7F, b(j)? & 0x7F);
        if x != y {
            return Ok(x.cmp(&y));
        }
    }
    Ok(Ordering::Equal)
}

/// The order of two values of one leaf; `None` when they do not compare (a NaN).
fn order(a: &Pt<'_>, b: &Pt<'_>) -> Result<Option<Ordering>, Fault> {
    let byte = |s: &[u8], j: u64| -> Result<u8, Fault> {
        s.get(usize::try_from(j).unwrap_or(usize::MAX)).copied().ok_or(rt(Corruption::Range, j))
    };
    Ok(match (*a, *b) {
        (Pt::Int(x), Pt::Int(y)) => Some(x.cmp(&y)),
        (Pt::Int(_), Pt::Big(_, neg)) => Some(if neg { Ordering::Greater } else { Ordering::Less }),
        (Pt::Big(_, neg), Pt::Int(_)) => Some(if neg { Ordering::Less } else { Ordering::Greater }),
        (Pt::Big(x, nx), Pt::Big(y, ny)) => {
            Some(big_order((x.len() as u64, nx), (y.len() as u64, ny), |j| byte(x, j), |j| byte(y, j))?)
        }
        (Pt::Bool(x), Pt::Bool(y)) => Some(x.cmp(&y)),
        (Pt::Float(x), Pt::Float(y)) => x.partial_cmp(&y),
        (Pt::Bytes(x), Pt::Bytes(y)) => Some(x.cmp(y)),
        (Pt::Unit, Pt::Unit) => Some(Ordering::Equal),
        _ => return Err(rt(Corruption::Encoding, 0)),
    })
}

/// The order of leaf `l`'s non-null stored value `v` against `p`; `None` when they do not compare.
fn order_of<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, v: &SVal, p: &Pt<'_>) -> Result<Option<Ordering>, Fault> {
    match *v {
        SVal::Null => Ok(None),
        SVal::Unit => order(&Pt::Unit, p),
        SVal::V(x) => order(&pt(l, x)?, p),
        _ => {
            let big = matches!(l.ty, Ty::Nat | Ty::Int);
            if let Some(s) = cur::piece_out(h, v)? {
                let a = if big { Pt::Big(s, negative(l, s)) } else { Pt::Bytes(s) };
                return order(&a, p);
            }
            split_order(h, l, v, p)
        }
    }
}

/// `order_of` for a byte string or a value beyond i128 that does not lie in one piece.
#[inline(never)]
fn split_order<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, v: &SVal, p: &Pt<'_>) -> Result<Option<Ordering>, Fault> {
    let n = v.len();
    match *p {
        Pt::Int(_) => {
            let last = cur::byte(h, v, n.checked_sub(1).ok_or(rt(Corruption::Encoding, 0))?)?;
            let neg = l.ty == Ty::Int && last & 0x40 != 0;
            Ok(Some(if neg { Ordering::Less } else { Ordering::Greater }))
        }
        Pt::Big(b, nb) => {
            let last = cur::byte(h, v, n.checked_sub(1).ok_or(rt(Corruption::Encoding, 0))?)?;
            let neg = l.ty == Ty::Int && last & 0x40 != 0;
            let at = |j: u64| b.get(usize::try_from(j).unwrap_or(usize::MAX)).copied().ok_or(rt(Corruption::Range, j));
            Ok(Some(big_order((n, neg), (b.len() as u64, nb), |j| cur::byte(h, v, j), at)?))
        }
        Pt::Bytes(b) => {
            let m = n.min(b.len() as u64);
            let mut piece = [0u8; 256];
            let mut at = 0u64;
            while at < m {
                let k = m.wrapping_sub(at).min(piece.len() as u64);
                let ku = usize::try_from(k).map_err(|_| rt(Corruption::Range, k))?;
                let d = piece.get_mut(..ku).ok_or(rt(Corruption::Range, k))?;
                cur::read(h, v, at, d)?;
                let from = usize::try_from(at).map_err(|_| rt(Corruption::Range, at))?;
                let e = b.get(from..from.wrapping_add(ku)).ok_or(rt(Corruption::Range, at))?;
                match (*d).cmp(e) {
                    Ordering::Equal => {}
                    o => return Ok(Some(o)),
                }
                at = at.wrapping_add(k);
            }
            Ok(Some(n.cmp(&(b.len() as u64))))
        }
        _ => Err(rt(Corruption::Encoding, u64::from(l.ordinal))),
    }
}

/// Whether a value whose order against end `b`'s value is `o` meets that end, a low end when `low`.
fn meets(b: &Bound<Pt<'_>>, o: Option<Ordering>, low: bool) -> bool {
    let beyond = if low { Ordering::Greater } else { Ordering::Less };
    match (b, o) {
        (Bound::Unbounded, _) => true,
        (_, None) => false,
        (Bound::Included(_), Some(o)) => o == Ordering::Equal || o == beyond,
        (Bound::Excluded(_), Some(o)) => o == beyond,
    }
}

// ---------------------------------------------------------------- the test

/// What a scan does with a sealed group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plan {
    /// No live row matches.
    Skip,
    /// Every live row matches.
    Pass,
    /// Each live row is tested.
    Rows,
}

/// A filter checked against its leaf.
struct Test<'a> {
    l: LeafInfo,
    /// The filter asks for null.
    null: bool,
    /// The filter is an equality: both ends included at one value.
    eq: bool,
    lo: Bound<Pt<'a>>,
    hi: Bound<Pt<'a>>,
}

impl<'a> Test<'a> {
    /// `BadRef` for a leaf the table does not have live; `BadValue` for a cell that is not of the leaf's type, a
    /// null end, a null asked of a leaf that cannot be null, or a value still in the message's argument.
    fn new<H: PageHost + ?Sized>(h: &H, shape: &Shape, f: &Filter<'a>) -> Result<Test<'a>, Fault> {
        let o = shape.ordinal_of(h, f.leaf)?.ok_or(Fault::BadRef)?;
        let l = shape.leaf(h, o)?;
        let end = |b: Bound<CellIn<'a>>| -> Result<Bound<Pt<'a>>, Fault> {
            Ok(match b {
                Bound::Included(c) => Bound::Included(pt(&l, cell::check(&l, c)?)?),
                Bound::Excluded(c) => Bound::Excluded(pt(&l, cell::check(&l, c)?)?),
                Bound::Unbounded => Bound::Unbounded,
            })
        };
        let open = Bound::Unbounded;
        Ok(match f.cond {
            Cond::Eq(c) => match cell::check(&l, c)? {
                Val::Null => Test { l, null: true, eq: false, lo: open, hi: open },
                v => {
                    let p = pt(&l, v)?;
                    Test { l, null: false, eq: true, lo: Bound::Included(p), hi: Bound::Included(p) }
                }
            },
            Cond::Range(a, b) => Test { l, null: false, eq: false, lo: end(a)?, hi: end(b)? },
        })
    }

    /// Whether the leaf's current value `v` meets the filter.
    fn matches<H: PageHost + ?Sized>(&self, h: &H, v: &SVal) -> Result<bool, Fault> {
        if self.null || v.is_null() {
            return Ok(self.null && v.is_null());
        }
        if let (true, Bound::Included(p)) = (self.eq, &self.lo) {
            return Ok(order_of(h, &self.l, v, p)? == Some(Ordering::Equal));
        }
        for (b, low) in [(&self.lo, true), (&self.hi, false)] {
            if let Bound::Included(p) | Bound::Excluded(p) = b {
                if !meets(b, order_of(h, &self.l, v, p)?, low) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Whether a value of rank `x` meets end `b`, a low end when `low`.
    fn rank_meets(b: &Bound<Pt<'_>>, x: u128, low: bool) -> Result<bool, Fault> {
        Ok(match b {
            Bound::Included(p) | Bound::Excluded(p) => meets(b, order(&Pt::Int(x), p)?, low),
            Bound::Unbounded => true,
        })
    }

    /// What to do with the sealed group whose page is `pg`, from its extremes record for the leaf.
    fn plan<H: PageHost + ?Sized>(&self, h: &H, pg: u64) -> Result<Plan, Fault> {
        if self.null || self.l.ty.running().is_none() {
            return Ok(Plan::Rows);
        }
        let blk = extremes::of_page(h, pg)?;
        if blk == 0 {
            return Ok(Plan::Rows);
        }
        let Some(r) = extremes::record(h, blk, &self.l)? else { return Ok(Plan::Rows) };
        if r.flags & BIG != 0 {
            return Ok(Plan::Rows);
        }
        // No value ever joined: every live row's value is null.
        if r.flags & HAS == 0 {
            return Ok(Plan::Skip);
        }
        if !Self::rank_meets(&self.lo, r.max, true)? || !Self::rank_meets(&self.hi, r.min, false)? {
            return Ok(Plan::Skip);
        }
        if !self.l.nullable && Self::rank_meets(&self.lo, r.min, true)? && Self::rank_meets(&self.hi, r.max, false)? {
            return Ok(Plan::Pass);
        }
        Ok(Plan::Rows)
    }
}

// ---------------------------------------------------------------- the scan

/// What a scan does with the matching rows.
trait Visit<H: PageHost + ?Sized> {
    /// Whether it takes no more rows.
    fn full(&self) -> bool {
        false
    }

    /// A sealed group's live positions [from, to), all matching: `true` when it took them at once, else they
    /// come one by one.
    fn passed(&mut self, _h: &H, _s: &Sealed, _from: u64, _to: u64) -> Result<bool, Fault> {
        Ok(false)
    }

    /// A sealed group opened (`gv`), before its rows.
    fn open(&mut self, _gv: &GroupView<'_, H>) -> Result<(), Fault> {
        Ok(())
    }

    /// A matching row at position `pos` of sealed group `s` (viewed by `gv`; the position's stored slot
    /// `slot`), with its value of the filter's leaf when that was read.
    #[allow(clippy::too_many_arguments)]
    fn sealed(
        &mut self,
        gv: &GroupView<'_, H>,
        shape: &Shape,
        s: &Sealed,
        pos: u64,
        slot: Option<u64>,
        rid: u64,
        v: Option<&SVal>,
    ) -> Result<(), Fault>;

    /// A matching buffered row, with its value of the filter's leaf when there is a filter.
    fn buffered(&mut self, h: &H, shape: &Shape, b: &Buffered, rid: u64, v: Option<&SVal>) -> Result<(), Fault>;
}

/// Runs `t` over the live rows from rid `from` on, handing the matching ones to `v` (every live row when `t` is
/// `None`); the rid to go on from when `v` became full before the last row, else `None`.
fn run<H: PageHost + ?Sized, V: Visit<H>>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    t: Option<&Test<'_>>,
    from: u64,
    v: &mut V,
) -> Result<Option<u64>, Fault> {
    let (open, _, tail) = buffer::open_group(h, tbl)?;
    let end = open.wrapping_mul(GROUP_SLOTS).wrapping_add(buffer::u16_of(h, tail, buffer::H_OPEN_POS)?);
    let mut rid = from;
    while rid < end {
        if v.full() {
            return Ok(Some(rid));
        }
        let g = rid / GROUP_SLOTS;
        let base = g.wrapping_mul(GROUP_SLOTS);
        let g_end = base.wrapping_add(GROUP_SLOTS).min(end);
        let entry = root::group(h, tbl, g)?;
        if entry == 0 {
            rid = g_end;
            continue;
        }
        if entry & DESC_BIT != 0 {
            while rid < g_end {
                if v.full() {
                    return Ok(Some(rid));
                }
                if let Some(Live::Buffered(at)) = read::live(h, tbl, rid)? {
                    let b = overlay::buffered(h, &at)?;
                    match t {
                        Some(t) => {
                            let x = overlay::buffered_leaf(h, shape, &b, &t.l)?;
                            if t.matches(h, &x)? {
                                v.buffered(h, shape, &b, rid, Some(&x))?;
                            }
                        }
                        None => v.buffered(h, shape, &b, rid, None)?,
                    }
                }
                rid = rid.wrapping_add(1);
            }
            continue;
        }
        let plan = match t {
            Some(t) => t.plan(h, entry)?,
            None => Plan::Pass,
        };
        if plan == Plan::Skip {
            rid = g_end;
            continue;
        }
        let s = overlay::sealed(h, entry)?;
        let to = base.wrapping_add(s.meta.p).min(g_end);
        if plan == Plan::Pass && v.passed(h, &s, rid.wrapping_sub(base), to.saturating_sub(base))? {
            rid = g_end;
            continue;
        }
        let gv = GroupView::from_meta(h, &s.meta);
        v.open(&gv)?;
        // The group's live bitmap, found once for its rows (each below the group's positions).
        let live = h.get(Region::Block(s.live_page))?;
        // The test each row takes: none in a passed group.
        let rows = if plan == Plan::Rows { t } else { None };
        let ld = match rows {
            Some(t) => gv.leaf_by_ordinal(u64::from(t.l.ordinal))?,
            None => None,
        };
        while rid < to {
            if v.full() {
                return Ok(Some(rid));
            }
            let pos = rid.wrapping_sub(base);
            if page::live_bit(live, pos)? {
                let slot = gv.slot(pos)?;
                match rows {
                    None => v.sealed(&gv, shape, &s, pos, slot, rid, None)?,
                    Some(t) => {
                        let x = overlay::leaf_value(&gv, shape, &s, &t.l, ld.as_ref(), slot, pos)?;
                        if t.matches(h, &x)? {
                            v.sealed(&gv, shape, &s, pos, slot, rid, Some(&x))?;
                        }
                    }
                }
            }
            rid = rid.wrapping_add(1);
        }
        rid = g_end;
    }
    Ok(None)
}

/// Hands the matching rows to a sink, at most `left` more.
struct Rows<'o, S: RowSink> {
    out: &'o mut S,
    left: u64,
}

impl<H: PageHost + ?Sized, S: RowSink> Visit<H> for Rows<'_, S> {
    fn full(&self) -> bool {
        self.left == 0
    }

    fn sealed(
        &mut self,
        gv: &GroupView<'_, H>,
        shape: &Shape,
        s: &Sealed,
        pos: u64,
        _slot: Option<u64>,
        rid: u64,
        _v: Option<&SVal>,
    ) -> Result<(), Fault> {
        read::emit_sealed(gv.h, shape, s, pos, self.out)?;
        self.out.row_end(rid)?;
        self.left = self.left.saturating_sub(1);
        Ok(())
    }

    fn buffered(&mut self, h: &H, shape: &Shape, b: &Buffered, rid: u64, _v: Option<&SVal>) -> Result<(), Fault> {
        read::emit_buffered(h, shape, b, self.out)?;
        self.out.row_end(rid)?;
        self.left = self.left.saturating_sub(1);
        Ok(())
    }
}

/// How many of the leaves a row hands out a cell scan finds once a call, and their leaf directory entries once a
/// group; a leaf past them is found again for each cell.
const COLS: usize = 64;

/// A leaf a cell scan hands out, and its leaf directory entry in the open group (`None`: the image lacks it).
#[derive(Clone, Copy)]
struct Col {
    l: LeafInfo,
    ld: Option<LeafDir>,
}

/// Hands the leaves `ids` names of each matching row to a sink, at most `left` more rows.
struct Cells<'o, 'i, S: RowSink> {
    ids: &'i [u32],
    /// The first `COLS` leaves of `ids`, found.
    cols: [Option<Col>; COLS],
    /// The filter's leaf, whose value the test has read.
    tested: Option<u16>,
    out: &'o mut S,
    left: u64,
}

impl<'o, 'i, S: RowSink> Cells<'o, 'i, S> {
    /// `BadRef` for a leaf of `ids` the table does not have live.
    fn new<H: PageHost + ?Sized>(
        h: &H,
        shape: &Shape,
        ids: &'i [u32],
        tested: Option<u16>,
        out: &'o mut S,
        left: u64,
    ) -> Result<Self, Fault> {
        let mut cols = [None; COLS];
        for (i, &id) in ids.iter().enumerate() {
            let o = shape.ordinal_of(h, id)?.ok_or(Fault::BadRef)?;
            if let Some(x) = cols.get_mut(i) {
                *x = Some(Col { l: shape.leaf(h, o)?, ld: None });
            }
        }
        Ok(Cells { ids, cols, tested, out, left })
    }

    /// Leaf i of `ids`.
    fn leaf<H: PageHost + ?Sized>(&self, h: &H, shape: &Shape, i: usize) -> Result<LeafInfo, Fault> {
        let id = *self.ids.get(i).ok_or(rt(Corruption::Range, i as u64))?;
        shape.leaf(h, shape.ordinal_of(h, id)?.ok_or(Fault::BadRef)?)
    }

    /// The value `v` the filter's test read, when leaf `l` is the filter's.
    fn reuse(&self, l: &LeafInfo, v: Option<&SVal>) -> Option<SVal> {
        v.filter(|_| self.tested == Some(l.ordinal)).copied()
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        self.out.row_end(rid)?;
        self.left = self.left.saturating_sub(1);
        Ok(())
    }
}

impl<H: PageHost + ?Sized, S: RowSink> Visit<H> for Cells<'_, '_, S> {
    fn full(&self) -> bool {
        self.left == 0
    }

    fn open(&mut self, gv: &GroupView<'_, H>) -> Result<(), Fault> {
        for c in self.cols.iter_mut().flatten() {
            c.ld = gv.leaf_by_ordinal(u64::from(c.l.ordinal))?;
        }
        Ok(())
    }

    fn sealed(
        &mut self,
        gv: &GroupView<'_, H>,
        shape: &Shape,
        s: &Sealed,
        pos: u64,
        slot: Option<u64>,
        rid: u64,
        v: Option<&SVal>,
    ) -> Result<(), Fault> {
        for i in 0..self.ids.len() {
            let found;
            let c = match self.cols.get(i) {
                Some(Some(c)) => c,
                _ => {
                    let l = self.leaf(gv.h, shape, i)?;
                    found = Col { l, ld: gv.leaf_by_ordinal(u64::from(l.ordinal))? };
                    &found
                }
            };
            let x = match self.reuse(&c.l, v) {
                Some(x) => x,
                None => overlay::leaf_value(gv, shape, s, &c.l, c.ld.as_ref(), slot, pos)?,
            };
            read::emit_sval(gv.h, self.out, &c.l, &x)?;
        }
        self.row_end(rid)
    }

    fn buffered(&mut self, h: &H, shape: &Shape, b: &Buffered, rid: u64, v: Option<&SVal>) -> Result<(), Fault> {
        for i in 0..self.ids.len() {
            let l = match self.cols.get(i).copied().flatten() {
                Some(c) => c.l,
                None => self.leaf(h, shape, i)?,
            };
            let x = match self.reuse(&l, v) {
                Some(x) => x,
                None => overlay::buffered_leaf(h, shape, b, &l)?,
            };
            read::emit_sval(h, self.out, &l, &x)?;
        }
        self.row_end(rid)
    }
}

/// The live positions in [from, to) of a sealed group, from its live bitmap.
fn live_in<H: PageHost + ?Sized>(h: &H, s: &Sealed, from: u64, to: u64) -> Result<u64, Fault> {
    let b = h.get(Region::Block(s.live_page))?;
    let mut n = 0u64;
    let mut p = from;
    while p < to {
        let w = p / 64;
        let word = u64::from_le_bytes(rd_array::<8>(b, page::P_BITMAP.wrapping_add(w.wrapping_mul(8)))?);
        let hi = to.wrapping_sub(w.wrapping_mul(64)).min(64);
        let below = if hi == 64 { u64::MAX } else { (1u64 << hi).wrapping_sub(1) };
        n = n.wrapping_add(u64::from((word & below & (u64::MAX << (p % 64))).count_ones()));
        p = w.wrapping_add(1).wrapping_mul(64);
    }
    Ok(n)
}

/// Counts the matching rows.
struct Count(u64);

impl<H: PageHost + ?Sized> Visit<H> for Count {
    fn passed(&mut self, h: &H, s: &Sealed, from: u64, to: u64) -> Result<bool, Fault> {
        self.0 = self.0.wrapping_add(live_in(h, s, from, to)?);
        Ok(true)
    }

    fn sealed(
        &mut self,
        _gv: &GroupView<'_, H>,
        _shape: &Shape,
        _s: &Sealed,
        _pos: u64,
        _slot: Option<u64>,
        _rid: u64,
        _v: Option<&SVal>,
    ) -> Result<(), Fault> {
        self.0 = self.0.wrapping_add(1);
        Ok(())
    }

    fn buffered(&mut self, _h: &H, _shape: &Shape, _b: &Buffered, _rid: u64, _v: Option<&SVal>) -> Result<(), Fault> {
        self.0 = self.0.wrapping_add(1);
        Ok(())
    }
}

/// The two's-complement bytes, little-endian and sign-extended without end, of a Nat's or Int's canonical
/// LEB128 value `v` of leaf `l`, read in pieces.
struct LebBytes<'v> {
    v: &'v SVal,
    n: u64,
    /// The 7-bit group a value's groups extend with past its last.
    ext: u8,
    next: u64,
    acc: u32,
    bits: u32,
    piece: [u8; 64],
    piece_at: u64,
    piece_len: u64,
}

impl<'v> LebBytes<'v> {
    fn new<H: PageHost + ?Sized>(h: &H, l: &LeafInfo, v: &'v SVal) -> Result<LebBytes<'v>, Fault> {
        let n = v.len();
        let last = cur::byte(h, v, n.checked_sub(1).ok_or(rt(Corruption::Encoding, u64::from(l.ordinal)))?)?;
        let ext = if l.ty == Ty::Int && last & 0x40 != 0 { 0x7F } else { 0 };
        Ok(LebBytes { v, n, ext, next: 0, acc: 0, bits: 0, piece: [0; 64], piece_at: 0, piece_len: 0 })
    }

    fn group<H: PageHost + ?Sized>(&mut self, h: &H) -> Result<u8, Fault> {
        let j = self.next;
        self.next = j.wrapping_add(1);
        if j >= self.n {
            return Ok(self.ext);
        }
        if j < self.piece_at || j >= self.piece_at.wrapping_add(self.piece_len) {
            let k = self.n.wrapping_sub(j).min(self.piece.len() as u64);
            let d = self.piece.get_mut(..usize::try_from(k).unwrap_or(0)).ok_or(rt(Corruption::Range, k))?;
            cur::read(h, self.v, j, d)?;
            (self.piece_at, self.piece_len) = (j, k);
        }
        let i = usize::try_from(j.wrapping_sub(self.piece_at)).unwrap_or(usize::MAX);
        Ok(self.piece.get(i).copied().ok_or(rt(Corruption::Range, j))? & 0x7F)
    }

    fn byte<H: PageHost + ?Sized>(&mut self, h: &H) -> Result<u8, Fault> {
        while self.bits < 8 {
            let g = self.group(h)?;
            self.acc |= u32::from(g) << self.bits;
            self.bits = self.bits.wrapping_add(7);
        }
        let x = self.acc.to_le_bytes()[0];
        self.acc >>= 8;
        self.bits = self.bits.wrapping_sub(8);
        Ok(x)
    }
}

/// Adds a two's-complement number, its bytes given in order by `next` (sign-extended), to the one of `len`
/// bytes at the start of `acc` (0 bytes: zero), over `need` bytes, which hold the sum; the sum's length
/// without its redundant sign bytes.
fn add_bytes(
    acc: &mut [u8],
    len: usize,
    need: usize,
    mut next: impl FnMut() -> Result<u8, Fault>,
) -> Result<usize, Fault> {
    let s = acc.get_mut(..need).ok_or(Fault::Scratch { need: need as u64 })?;
    let top = len.checked_sub(1).and_then(|i| s.get(i)).copied().unwrap_or(0);
    if let Some(above) = s.get_mut(len..) {
        above.fill(if top & 0x80 != 0 { 0xFF } else { 0 });
    }
    let mut carry = 0u16;
    for a in s.iter_mut() {
        let t = u16::from(*a).wrapping_add(u16::from(next()?)).wrapping_add(carry);
        *a = t.to_le_bytes()[0];
        carry = t >> 8;
    }
    Ok(totals::trimmed(s))
}

/// The sum and the non-null count of an integer leaf's values: those within 64 bits in an i128 (fewer than
/// 2^61 rows of them cannot pass it), u128s, i128s and Nats and Ints up to i128 in an i192, and the values
/// beyond i128 in the caller's buffer.
struct Sum<'b> {
    l: LeafInfo,
    /// The leaf is the filter's.
    same: bool,
    /// Its leaf directory entry in the open group.
    ld: Option<LeafDir>,
    small: i128,
    wide: I192,
    buf: &'b mut [u8],
    /// Bytes of the sum of the values beyond i128 at the start of `buf` (0: none).
    big: usize,
    /// The bytes the longest value beyond i128 that `buf` could not take needs (0: none).
    short: usize,
    n: u64,
}

impl Sum<'_> {
    fn add<H: PageHost + ?Sized>(&mut self, h: &H, v: &SVal) -> Result<(), Fault> {
        match *v {
            SVal::Null => return Ok(()),
            SVal::V(Val::U(x)) => self.small = self.small.wrapping_add(i128::from(x)),
            SVal::V(Val::I(x)) => self.small = self.small.wrapping_add(i128::from(x)),
            SVal::V(Val::Wide(x) | Val::I128(x)) => self.wide.apply_i128(x, false),
            SVal::V(Val::U128(x)) => self.wide.apply_u128(x, false),
            SVal::B { .. } | SVal::Huge { .. } if matches!(self.l.ty, Ty::Nat | Ty::Int) => self.add_big(h, v)?,
            _ => return Err(rt(Corruption::Encoding, u64::from(self.l.ordinal))),
        }
        self.n = self.n.wrapping_add(1);
        Ok(())
    }

    /// A value beyond i128 added to the sum in `buf`; when `buf` cannot hold it, only its size is noted.
    #[inline(never)]
    fn add_big<H: PageHost + ?Sized>(&mut self, h: &H, v: &SVal) -> Result<(), Fault> {
        let bytes = usize::try_from(v.len().saturating_mul(7).div_ceil(8)).unwrap_or(usize::MAX).saturating_add(1);
        let need = self.big.max(bytes).saturating_add(1);
        if need > self.buf.len() || self.short != 0 {
            self.short = self.short.max(bytes);
            return Ok(());
        }
        let mut src = LebBytes::new(h, &self.l, v)?;
        self.big = add_bytes(self.buf, self.big, need, || src.byte(h))?;
        Ok(())
    }

    /// The sum, written to `buf` when it is past i128, and the count. `Scratch { need }` when `buf` was too short
    /// for a value beyond i128 or for the sum: `need` holds the sum of any count of values as long as the
    /// longest.
    fn finish(mut self) -> Result<(Total, u64), Fault> {
        let room = |b: usize| b.max(25).saturating_add(10) as u64;
        if self.short != 0 {
            return Err(Fault::Scratch { need: room(self.short.max(self.big)) });
        }
        self.wide.apply_i128(self.small, false);
        let mut w = [0u8; 24];
        for (d, limb) in w.chunks_exact_mut(8).zip(self.wide.0) {
            crate::mem::copy_bytes(d, &limb.to_le_bytes());
        }
        if self.big == 0 {
            return Ok((totals::bytes_total(&w, self.buf)?, self.n));
        }
        let need = self.big.max(w.len()).saturating_add(1);
        if need > self.buf.len() {
            return Err(Fault::Scratch { need: room(self.big) });
        }
        let mut i = 0usize;
        let ext = if w[23] & 0x80 != 0 { 0xFF } else { 0 };
        let n = add_bytes(self.buf, self.big, need, || {
            let x = w.get(i).copied().unwrap_or(ext);
            i = i.wrapping_add(1);
            Ok(x)
        })?;
        let t = match totals::small_of(self.buf.get(..n).unwrap_or(&[])) {
            Some(x) => Total::Small(x),
            None => Total::Big(n),
        };
        Ok((t, self.n))
    }
}

impl<H: PageHost + ?Sized> Visit<H> for Sum<'_> {
    fn open(&mut self, gv: &GroupView<'_, H>) -> Result<(), Fault> {
        self.ld = gv.leaf_by_ordinal(u64::from(self.l.ordinal))?;
        Ok(())
    }

    fn sealed(
        &mut self,
        gv: &GroupView<'_, H>,
        shape: &Shape,
        s: &Sealed,
        pos: u64,
        slot: Option<u64>,
        _rid: u64,
        v: Option<&SVal>,
    ) -> Result<(), Fault> {
        if let (true, Some(x)) = (self.same, v) {
            return self.add(gv.h, x);
        }
        let x = overlay::leaf_value(gv, shape, s, &self.l, self.ld.as_ref(), slot, pos)?;
        self.add(gv.h, &x)
    }

    fn buffered(&mut self, h: &H, shape: &Shape, b: &Buffered, _rid: u64, v: Option<&SVal>) -> Result<(), Fault> {
        if let (true, Some(x)) = (self.same, v) {
            return self.add(h, x);
        }
        let x = overlay::buffered_leaf(h, shape, b, &self.l)?;
        self.add(h, &x)
    }
}

/// The live rows meeting `f`, from rid `next` on in rid order, at most `limit` of them, handed to `out`; the
/// rid to go on from, or `None` when no row is left.
pub fn scan<H: PageHost + ?Sized, S: RowSink>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    f: &Filter<'_>,
    next: u64,
    limit: u64,
    out: &mut S,
) -> Result<Option<u64>, Fault> {
    let t = Test::new(h, shape, f)?;
    run(h, tbl, shape, Some(&t), next, &mut Rows { out, left: limit })
}

/// The live rows meeting `f` (every live row when it is `None`), from rid `next` on in rid order, at most `limit`
/// of them, each handed to `out` as the values of the leaves `leaves` names in the order named; the rid to go on
/// from, or `None` when no row is left.
#[allow(clippy::too_many_arguments)]
pub fn cells<H: PageHost + ?Sized, S: RowSink>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    leaves: &[u32],
    f: Option<&Filter<'_>>,
    next: u64,
    limit: u64,
    out: &mut S,
) -> Result<Option<u64>, Fault> {
    let t = match f {
        Some(f) => Some(Test::new(h, shape, f)?),
        None => None,
    };
    let mut c = Cells::new(h, shape, leaves, t.as_ref().map(|t| t.l.ordinal), out, limit)?;
    run(h, tbl, shape, t.as_ref(), next, &mut c)
}

/// How many live rows meet `f`.
pub fn count<H: PageHost + ?Sized>(h: &H, tbl: u64, shape: &Shape, f: &Filter<'_>) -> Result<u64, Fault> {
    let t = Test::new(h, shape, f)?;
    let mut c = Count(0);
    run(h, tbl, shape, Some(&t), 0, &mut c)?;
    Ok(c.0)
}

/// The exact sum of integer leaf `leaf`'s non-null values over the live rows meeting `f`, and their count.
pub fn sum<H: PageHost + ?Sized>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    f: &Filter<'_>,
    leaf: u32,
    buf: &mut [u8],
) -> Result<(Total, u64), Fault> {
    let t = Test::new(h, shape, f)?;
    let o = shape.ordinal_of(h, leaf)?.ok_or(Fault::BadRef)?;
    let l = shape.leaf(h, o)?;
    if l.ty.running().is_none() {
        return Err(Fault::BadRef);
    }
    let same = l.ordinal == t.l.ordinal;
    let mut s = Sum { l, same, ld: None, small: 0, wide: I192::default(), buf, big: 0, short: 0, n: 0 };
    run(h, tbl, shape, Some(&t), 0, &mut s)?;
    s.finish()
}
