//! The sorted walk of a secondary index: its rows in key order, ascending or descending, from an end or from a
//! key, at most a given number at a time, over the tree and the uploaded segments; and the index's smallest
//! and largest keys, from its ends.
//!
//! **Order.** Keys rise, or fall, by their encoding; the rows of one key come in rising rid order in either
//! direction, as OQL hands out the rows of one value of an index in a range walk whichever its direction, which
//! is also the order a sort keeps ties in. A row the tree and a segment both hold is handed out once.
//!
//! **Reads.** One forward cursor reads the tree's rows of the key walked. Going forward it runs on from key to
//! key, so a walk of the tree alone is one descent, then one step per entry; going backward, each key takes two
//! descents, one to find it (its count entry, the last of its entries) and one to its first row. Each uploaded
//! segment adds one key step per key; for a key the segments hold, each segment adds one descent to find its
//! first row of the key, and each segment holding the key one descent per row it hands out: the walk keeps the
//! next row of each of the first `HOLDERS` segments holding the key. A key held by more segments reads the
//! segments past the last one kept again for each row they hand out.
//!
//! The walk keeps no state between calls: a walk stopped by its limit resumes after the row it handed out last
//! (`Start::After`).

use core::cmp::Ordering;
use core::ops::Bound;

use super::{entry_key, exact, field_at, key_run_len, rt, take, Secondary};
use crate::fault::{Corruption, Fault};
use crate::index::area_nodes::AreaView;
use crate::index::runlist::LoadedRecord;
use crate::key::probe::{is_count, is_ovf, payload, KeyProbe, Kind, ResolveKey};
use crate::key::KeySpec;
use crate::mem::PageHost;
use crate::tree::node::{is_leaf, ko};
use crate::tree::{Cursor, Dir, Fwd, IndexShape, Probe, Rev, Shape};

/// Where a walk starts, in its direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start<'k> {
    /// At the index's first key in the walk's direction: its smallest going forward, its largest backward.
    End,
    /// At the key (its full encoding), or at the first key past it in the walk's direction when no row holds it.
    Key(&'k [u8]),
    /// After the row with this rid under the key: where a walk that handed that row out last stopped.
    After(&'k [u8], u64),
}

/// A key found: its length, and whether the tree and the uploaded segments may hold it.
#[derive(Clone, Copy, Debug)]
struct Found {
    len: usize,
    tree: bool,
    segments: bool,
}

/// The uploaded segments whose next rows of one key a walk keeps, at most.
const HOLDERS: usize = 64;

/// The uploaded segments holding rows of the key walked, each with its next row: the first `HOLDERS` of them
/// in upload order, each read again only when it hands out a row. The segments from `rest` on are read again
/// whenever one of them hands out a row, and fill the places of exhausted ones then.
struct Holders {
    /// The segments kept (the first `n`), each as (its ordinal, its next row).
    kept: [(u64, u64); HOLDERS],
    n: usize,
    /// The first segment not kept that may hold a row of the key: those between the last kept one and it hold
    /// none.
    rest: u64,
    /// The smallest next row among the segments from `rest` on.
    rest_next: Option<u64>,
}

impl Holders {
    fn new() -> Holders {
        Holders { kept: [(0, 0); HOLDERS], n: 0, rest: 0, rest_next: None }
    }

    /// Holds no segment: the key walked is one no segment holds.
    fn clear(&mut self) {
        (self.n, self.rest_next) = (0, None);
    }

    /// Starts the segments' rows of `key` above `after` (from its first row when `None`).
    fn start<H: PageHost + ?Sized>(
        &mut self,
        ix: &Secondary,
        h: &H,
        loaded: &LoadedRecord,
        key: &[u8],
        after: Option<u64>,
    ) -> Result<(), Fault> {
        self.clear();
        self.rest = 0;
        self.read_rest(ix, h, loaded, key, after)
    }

    /// Reads the next row above `after` of each segment from `rest` on: kept while a place is free and every
    /// segment before it holding a row is kept; the others give `rest` and `rest_next`.
    fn read_rest<H: PageHost + ?Sized>(
        &mut self,
        ix: &Secondary,
        h: &H,
        loaded: &LoadedRecord,
        key: &[u8],
        after: Option<u64>,
    ) -> Result<(), Fault> {
        let mut j = self.rest;
        self.rest = loaded.runs;
        while j < loaded.runs {
            if let Some(r) = ix.segment(h, loaded, j)?.row_after(key, after)? {
                match self.kept.get_mut(self.n).filter(|_| self.rest == loaded.runs) {
                    Some(slot) => {
                        *slot = (j, r);
                        self.n = self.n.wrapping_add(1);
                    }
                    None => {
                        self.rest = self.rest.min(j);
                        self.rest_next = Some(self.rest_next.map_or(r, |b| b.min(r)));
                    }
                }
            }
            j = j.wrapping_add(1);
        }
        Ok(())
    }

    /// The smallest next row of the segments.
    fn first(&self) -> Option<u64> {
        let kept = self.kept.get(..self.n).unwrap_or(&[]).iter().map(|&(_, x)| x).min();
        match (kept, self.rest_next) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// After the row `rid` was handed out: the next row of each segment that held it, read from that segment
    /// alone when it is kept; an exhausted one's place is taken by the last kept one.
    fn pass<H: PageHost + ?Sized>(
        &mut self,
        ix: &Secondary,
        h: &H,
        loaded: &LoadedRecord,
        key: &[u8],
        rid: u64,
    ) -> Result<(), Fault> {
        let mut i = self.n;
        while i > 0 {
            i = i.wrapping_sub(1);
            let Some(&(j, x)) = self.kept.get(i) else { break };
            if x != rid {
                continue;
            }
            let moved = match ix.segment(h, loaded, j)?.row_after(key, Some(rid))? {
                Some(r) => (j, r),
                None => {
                    self.n = self.n.wrapping_sub(1);
                    self.kept.get(self.n).copied().unwrap_or((j, x))
                }
            };
            if let Some(slot) = self.kept.get_mut(i) {
                *slot = moved;
            }
        }
        if self.rest_next == Some(rid) {
            self.rest_next = None;
            self.read_rest(ix, h, loaded, key, Some(rid))?;
        }
        Ok(())
    }
}

impl Secondary {
    /// Hands the index's rows to `row` in key order, from `start` in direction `dir`, at most `limit` of them:
    /// each as its key, the full encoding, and its rid; returns how many it handed out. `spec` is the index's key
    /// spec. `key` takes the key walked and `scratch`, in two halves, the next key and each segment's, so `key`
    /// and each half must hold the index's longest key, and at least W bytes (a segment's key, its shared prefix
    /// and W more): a key that does not fit is `Scratch { need }`, the bytes each must hold, before any row of
    /// that key is handed out, and the walk resumes after the last row it handed out.
    #[allow(clippy::too_many_arguments)]
    pub fn walk<H, F>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        dir: Dir,
        start: Start<'_>,
        limit: u64,
        key: &mut [u8],
        scratch: &mut [u8],
        mut row: F,
    ) -> Result<u64, Fault>
    where
        H: PageHost + ?Sized,
        F: FnMut(&[u8], u64) -> Result<(), Fault>,
    {
        if limit == 0 {
            return Ok(0);
        }
        let half = scratch.len() / 2;
        let (next, seg) = scratch.split_at_mut_checked(half).ok_or(Fault::Scratch { need: 0 })?;
        let (s, v) = (self.shape, self.view(h));
        // The key walked, at the start of `key`, and the rid of its row handed out last.
        let (mut at, mut after) = match start {
            Start::After(k, rid) => (Found { len: take(key, k)?, tree: true, segments: true }, Some(rid)),
            Start::End | Start::Key(_) => {
                let from = if let Start::Key(k) = start { Bound::Included(k) } else { Bound::Unbounded };
                let Some(f) = self.first_key(h, spec, from, dir, next, seg)? else { return Ok(0) };
                take(key, next.get(..f.len).unwrap_or(&[]))?;
                (f, None)
            }
        };
        let mut done = 0u64;
        // In the key walked; going forward it stays placed from key to key.
        let mut c: Cursor<Fwd> = Cursor::empty();
        let mut placed = false;
        let loaded = self.loaded(h)?;
        let mut hold = Holders::new();
        loop {
            let k = key.get(..at.len).ok_or(rt(Corruption::Bounds, at.len as u64))?;
            match dir {
                Dir::Fwd if placed => {}
                Dir::Rev if !at.tree => c.clear(),
                _ => {
                    if self.root != 0 {
                        let p = KeyProbe { key: k, kind: after.map_or(Kind::Low, Kind::Row) };
                        let b = if after.is_some() { Bound::Excluded(&p) } else { Bound::Included(&p) };
                        c.seek_here(s, &v, self.root, b)?;
                    }
                    placed = true;
                }
            }
            // The key's rows, the tree's and the segments' merged in rid order.
            let mut t = tree_row(s, &v, &mut c, k)?;
            let segs = loaded.as_ref().filter(|_| at.segments);
            match segs {
                Some(l) => hold.start(self, h, l, k, after)?,
                None => hold.clear(),
            }
            loop {
                let g = hold.first();
                let rid = match (t, g) {
                    (Some((a, ..)), Some(b)) => a.min(b),
                    (Some((a, ..)), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => break,
                };
                if let Some((a, n, i)) = t {
                    if a == rid {
                        c.advance(s, &v, n, i)?;
                        t = tree_row(s, &v, &mut c, k)?;
                    }
                }
                if let (Some(l), true) = (segs, g == Some(rid)) {
                    hold.pass(self, h, l, k, rid)?;
                }
                row(k, rid)?;
                done = done.wrapping_add(1);
                if done >= limit {
                    return Ok(done);
                }
            }
            // The next key: going forward, the tree's is where the cursor stands.
            let from = Bound::Excluded(k);
            let tree = match dir {
                Dir::Fwd => cursor_key(s, &v, spec, &mut c, k, next)?,
                Dir::Rev => self.tree_key(h, spec, from, Dir::Rev, next)?,
            };
            let Some(f) = self.segment_key(h, spec, from, dir, tree, next, seg)? else { return Ok(done) };
            take(key, next.get(..f.len).unwrap_or(&[]))?;
            (at, after) = (f, None);
        }
    }

    /// The index's smallest key whose first part is not null, whole at the start of `out`; its length. `None`
    /// when no row holds such a key: the smallest value of a column, which skips nulls, as OQL's `min` does.
    /// `scratch` takes each segment's key; a key that does not fit either is `Scratch { need }`, the bytes each
    /// must hold.
    pub fn min_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        out: &mut [u8],
        scratch: &mut [u8],
    ) -> Result<Option<usize>, Fault> {
        // A null first part encodes as its tag 00, below the tag 01 every value of the part starts with.
        let from = if first_nullable(spec) { Bound::Included(&[1u8][..]) } else { Bound::Unbounded };
        Ok(self.first_key(h, spec, from, Dir::Fwd, out, scratch)?.map(|f| f.len))
    }

    /// The index's largest key whose first part is not null, whole at the start of `out`; its length. `None`
    /// when no row holds such a key, as OQL's `max` of a column. `scratch` takes each segment's key.
    pub fn max_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        out: &mut [u8],
        scratch: &mut [u8],
    ) -> Result<Option<usize>, Fault> {
        let Some(f) = self.first_key(h, spec, Bound::Unbounded, Dir::Rev, out, scratch)? else { return Ok(None) };
        // Every key whose first part is null lies below every other.
        if first_nullable(spec) && out.first() == Some(&0) {
            return Ok(None);
        }
        Ok(Some(f.len))
    }

    /// The first key from `from` in direction `dir` that the tree or an uploaded segment holds, whole at the
    /// start of `out`. `scratch` takes each segment's key.
    fn first_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        from: Bound<&[u8]>,
        dir: Dir,
        out: &mut [u8],
        scratch: &mut [u8],
    ) -> Result<Option<Found>, Fault> {
        let tree = self.tree_key(h, spec, from, dir, out)?;
        self.segment_key(h, spec, from, dir, tree, out, scratch)
    }

    /// The tree's first key from `from` in direction `dir`, whole at the start of `out`; its length.
    fn tree_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        from: Bound<&[u8]>,
        dir: Dir,
        out: &mut [u8],
    ) -> Result<Option<usize>, Fault> {
        if self.root == 0 {
            return Ok(None);
        }
        let (s, v) = (self.shape, self.view(h));
        // A key's entries run from its first row to its count entry: going forward a bound key's entries start at
        // its first row, going backward at its count entry.
        let (first, last) = match dir {
            Dir::Fwd => (Kind::Low, Kind::Count),
            Dir::Rev => (Kind::Count, Kind::Low),
        };
        let probe = match from {
            Bound::Included(key) => Bound::Included(KeyProbe { key, kind: first }),
            Bound::Excluded(key) => Bound::Excluded(KeyProbe { key, kind: last }),
            Bound::Unbounded => Bound::Unbounded,
        };
        let found = match dir {
            Dir::Fwd => Cursor::<Fwd>::seek(s, &v, self.root, probe.as_ref())?.peek(s, &v)?,
            Dir::Rev => Cursor::<Rev>::seek(s, &v, self.root, probe.as_ref())?.peek(s, &v)?,
        };
        let Some((n, i)) = found else { return Ok(None) };
        let len = entry_key(&v, n, i, out)?;
        Ok(Some(exact(spec, out, len)?))
    }

    /// Adds the uploaded segments to the tree's first key from `from` in direction `dir` (`tree`, its length,
    /// at the start of `out`): the first key from `from` that the tree or a segment holds, whole at the start of
    /// `out`, and which of them hold it. `scratch` takes each segment's key.
    #[allow(clippy::too_many_arguments)]
    fn segment_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        from: Bound<&[u8]>,
        dir: Dir,
        tree: Option<usize>,
        out: &mut [u8],
        scratch: &mut [u8],
    ) -> Result<Option<Found>, Fault> {
        let mut best = tree.map(|len| Found { len, tree: true, segments: false });
        let key_run_len = key_run_len(spec);
        let after = match from {
            Bound::Included(k) | Bound::Excluded(k) => Some(k),
            Bound::Unbounded => None,
        };
        // The order of a key that comes first.
        let sooner = if dir == Dir::Fwd { Ordering::Less } else { Ordering::Greater };
        self.segments(h, |run| {
            let len = match from {
                // A segment that holds the bound itself has it first.
                Bound::Included(k) if run.count_key(k)? > 0 => held_bound(spec, k, run.hdr.w, scratch)?,
                _ => match run.key_step(after, dir, key_run_len, scratch)? {
                    Some((len, _)) => exact(spec, scratch, len)?,
                    None => return Ok(()),
                },
            };
            let key = scratch.get(..len).ok_or(Fault::Scratch { need: len as u64 })?;
            match best.as_mut().map(|b| (key.cmp(out.get(..b.len).unwrap_or(&[])), b)) {
                Some((Ordering::Equal, b)) => b.segments = true,
                Some((o, _)) if o != sooner => {}
                _ => best = Some(Found { len: take(out, key)?, tree: false, segments: true }),
            }
            Ok(())
        })?;
        Ok(best)
    }
}

/// The key a segment holds for an included bound `k` its count finds, whole at the start of `out`; its length.
/// A segment's count compares key areas, which pad a key with zero bytes, so a bound that is not a whole key
/// (`min_key`'s, a first part's tag alone) finds the key that is the bound followed by zero bytes: the bound,
/// zero-padded to the end the key spec gives, which lies within the segment's W bytes past the bound (the key
/// area a key that fits W pads). A whole key is itself.
fn held_bound(spec: &KeySpec<'_>, k: &[u8], w: u64, out: &mut [u8]) -> Result<usize, Fault> {
    let need = (k.len() as u64).saturating_add(w);
    let end = usize::try_from(need).map_err(|_| Fault::Scratch { need })?;
    let out = out.get_mut(..end).ok_or(Fault::Scratch { need })?;
    let n = take(out, k)?;
    out.get_mut(n..).unwrap_or(&mut []).fill(0);
    exact(spec, out, end)
}

/// Whether the index's first part is nullable, so that a key starting with the null tag 00 is a null.
fn first_nullable(spec: &KeySpec<'_>) -> bool {
    spec.parts.first().is_some_and(|p| p.nullable)
}

/// Entry i of node n against the end of `key`'s entries, its count entry: `Less` for a row of `key` when the
/// entry lies at or after the key's first row, `Equal` for its count entry, `Greater` past it.
fn against_key<H: PageHost + ?Sized>(
    s: IndexShape,
    v: &AreaView<'_, H>,
    n: u64,
    i: u64,
    key: &[u8],
) -> Result<Ordering, Fault> {
    let koff = ko(s, &s.shape(is_leaf(v, n)?), i);
    KeyProbe { key, kind: Kind::Count }.cmp_at(v, n, koff)
}

/// The row of `key` a forward cursor standing at or after the key's first row stands at: its rid, node and
/// index; `None` at the key's count entry or past it.
fn tree_row<H: PageHost + ?Sized>(
    s: IndexShape,
    v: &AreaView<'_, H>,
    c: &mut Cursor<Fwd>,
    key: &[u8],
) -> Result<Option<(u64, u64, u64)>, Fault> {
    let Some((n, i)) = c.peek(s, v)? else { return Ok(None) };
    if against_key(s, v, n, i, key)? != Ordering::Less {
        return Ok(None);
    }
    let (_, field) = field_at(s, v, n, i)?;
    if is_count(field) {
        return Err(rt(Corruption::Order, n));
    }
    let rid = if is_ovf(field) { v.record(field)?.0 } else { payload(field) };
    Ok(Some((rid, n, i)))
}

/// The tree's next key after `key`, whole at the start of `out`, from a forward cursor standing past the
/// key's rows: at its count entry, which it passes, or past it.
fn cursor_key<H: PageHost + ?Sized>(
    s: IndexShape,
    v: &AreaView<'_, H>,
    spec: &KeySpec<'_>,
    c: &mut Cursor<Fwd>,
    key: &[u8],
    out: &mut [u8],
) -> Result<Option<usize>, Fault> {
    let Some((n, i)) = c.peek(s, v)? else { return Ok(None) };
    if against_key(s, v, n, i, key)? == Ordering::Equal {
        c.advance(s, v, n, i)?;
    }
    let Some((n, i)) = c.peek(s, v)? else { return Ok(None) };
    let len = entry_key(v, n, i, out)?;
    Ok(Some(exact(spec, out, len)?))
}
