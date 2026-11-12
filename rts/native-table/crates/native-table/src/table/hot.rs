//! The hot state's running records, reached leaf by leaf in ordinal order (with the totals the totals space
//! holds), and the keeping of an integer leaf's total for one stored value.

use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::mem::{rd_u16, PageHost, Region};
use crate::stats::running::{RKind, LIST_AT};

use super::group::align8;
use super::layout::{LeafInfo, Shape};
use super::totals::{self, At};

/// Walks the running-record leaf list alongside the leaves, and the totals space's records with them.
pub struct RunCursor {
    idx: u64,
    off: u64,
    r: u64,
    space: u64,
}

impl RunCursor {
    /// The cursor of a hot state whose running-record leaf list has `r` entries.
    pub fn of_list(r: u64) -> Self {
        RunCursor { idx: 0, off: LIST_AT.wrapping_add(align8(r.wrapping_mul(2))), r, space: 0 }
    }

    /// The cursor of the hot state in tail block `tail`: its running-record leaf list's length from the hot
    /// state itself.
    pub fn open<H: crate::mem::PageHost + ?Sized>(h: &H, tail: u64) -> Result<Self, Fault> {
        let b = h.get(crate::mem::Region::Block(tail))?;
        Ok(RunCursor::of_list(u64::from(rd_u16(b, super::buffer::H_RUN_R)?)))
    }

    /// Where leaf `l`'s running record lies: in the hot state, in the totals space (an integer leaf's total
    /// the hot state has no room for), or nowhere (a leaf that is not an integer); leaves come in ordinal
    /// order, each once.
    pub fn place(&mut self, hot: &[u8], l: &LeafInfo) -> Result<At, Fault> {
        let Some(k) = l.ty.running() else { return Ok(At::None) };
        if self.idx < self.r {
            let o = rd_u16(hot, LIST_AT.wrapping_add(self.idx.wrapping_mul(2)))?;
            if o == l.ordinal {
                let at = self.off;
                self.off = self.off.wrapping_add(k.bytes());
                self.idx = self.idx.wrapping_add(1);
                return Ok(At::Hot(at));
            }
            if o < l.ordinal {
                return Err(corrupt(ImageId::Runtime, Corruption::Order, u64::from(o)));
            }
        }
        let at = self.space;
        self.space = self.space.wrapping_add(k.bytes());
        Ok(At::Space(at))
    }

    /// `place` for the leaf of ordinal `ordinal` and running kind `k`. It keeps its own copy of `place`'s body:
    /// with one body shared, every insert and delete costs a few instructions more.
    pub fn place_kind(&mut self, hot: &[u8], ordinal: u16, k: Option<RKind>) -> Result<At, Fault> {
        let Some(k) = k else { return Ok(At::None) };
        if self.idx < self.r {
            let o = rd_u16(hot, LIST_AT.wrapping_add(self.idx.wrapping_mul(2)))?;
            if o == ordinal {
                let at = self.off;
                self.off = self.off.wrapping_add(k.bytes());
                self.idx = self.idx.wrapping_add(1);
                return Ok(At::Hot(at));
            }
            if o < ordinal {
                return Err(corrupt(ImageId::Runtime, Corruption::Order, u64::from(o)));
            }
        }
        let at = self.space;
        self.space = self.space.wrapping_add(k.bytes());
        Ok(At::Space(at))
    }
}

/// How a stored row's values change the totals: +1 for an insert, −1 for a delete.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Add,
    Remove,
}

/// Where leaf `l`'s running record lies, the hot state in tail block `tail`: the records of the leaves before
/// it are skipped by their types. O(L).
pub fn place_of<H: PageHost + ?Sized>(h: &H, shape: &Shape, tail: u64, l: &LeafInfo) -> Result<At, Fault> {
    let mut rc = RunCursor::open(h, tail)?;
    let b = h.get(Region::Block(tail))?;
    let p = h.get(Region::Page(shape.page))?;
    for o in 0..l.ordinal {
        let _ = rc.place_kind(b, o, shape.running_of(p, u64::from(o))?)?;
    }
    rc.place(b, l)
}

/// Moves the total of leaf `l` (its record at `at`, the hot state in tail block `tail`) and the extremes of its
/// row's group (block `ext`) by one value `v`; nothing for a leaf without a total.
#[allow(clippy::too_many_arguments)]
pub fn apply_at<H: PageHost>(
    h: &mut H,
    tbl: u64,
    tail: u64,
    at: At,
    l: &LeafInfo,
    v: &super::cur::SVal,
    sign: Sign,
    ext: u64,
) -> Result<(), Fault> {
    match at {
        At::None => Ok(()),
        at => totals::apply(h, tbl, tail, at, l, totals::TSrc::Cur(v), sign, ext),
    }
}
