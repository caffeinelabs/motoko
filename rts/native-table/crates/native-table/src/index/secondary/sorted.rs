//! The sorted walk of a secondary index: its rows in key order, ascending or descending, from an end or from a
//! key, at most a given number at a time; and the index's smallest and largest keys, from its ends.
//!
//! **Order.** Keys rise, or fall, by their encoding; the rows of one key come in rising rid order in either
//! direction, as OQL hands out the rows of one value of an index in a range walk whichever its direction, which
//! is also the order a sort keeps ties in.
//!
//! **Reads.** One forward cursor reads the tree's rows of the key walked. Going forward it runs on from key to
//! key, so a walk is one descent, then one step per entry; going backward, each key takes two descents, one to
//! find it (its count entry, the last of its entries) and one to its first row.
//!
//! The walk keeps no state between calls: a walk stopped by its limit resumes after the row it handed out last
//! (`Start::After`).

use core::cmp::Ordering;
use core::ops::Bound;

use super::{entry_key, exact, field_at, rt, take, Secondary};
use crate::fault::{Corruption, Fault};
use crate::index::area_nodes::AreaView;
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

impl Secondary {
    /// Hands the index's rows to `row` in key order, from `start` in direction `dir`, at most `limit` of them:
    /// each as its key, the full encoding, and its rid; returns how many it handed out. `spec` is the index's key
    /// spec. `key` takes the key walked and `next` the next key, so each must hold the index's longest key, and
    /// at least W bytes: a key that does not fit is `Scratch { need }`, the bytes each must hold, before any row
    /// of that key is handed out, and the walk resumes after the last row it handed out.
    #[allow(clippy::too_many_arguments)]
    pub fn walk<H, F>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        dir: Dir,
        start: Start<'_>,
        limit: u64,
        key: &mut [u8],
        next: &mut [u8],
        mut row: F,
    ) -> Result<u64, Fault>
    where
        H: PageHost + ?Sized,
        F: FnMut(&[u8], u64) -> Result<(), Fault>,
    {
        if limit == 0 {
            return Ok(0);
        }
        let (s, v) = (self.shape, self.view(h));
        // The length of the key walked, at the start of `key`, and the rid of its row handed out last.
        let (mut len, mut after) = match start {
            Start::After(k, rid) => (take(key, k)?, Some(rid)),
            Start::End | Start::Key(_) => {
                let from = if let Start::Key(k) = start { Bound::Included(k) } else { Bound::Unbounded };
                let Some(n) = self.tree_key(h, spec, from, dir, next)? else { return Ok(0) };
                (take(key, next.get(..n).unwrap_or(&[]))?, None)
            }
        };
        let mut done = 0u64;
        // In the key walked; going forward it stays placed from key to key.
        let mut c: Cursor<Fwd> = Cursor::empty();
        let mut placed = false;
        loop {
            let k = key.get(..len).ok_or(rt(Corruption::Bounds, len as u64))?;
            if dir == Dir::Rev || !placed {
                if self.root != 0 {
                    let p = KeyProbe { key: k, kind: after.map_or(Kind::Low, Kind::Row) };
                    let b = if after.is_some() { Bound::Excluded(&p) } else { Bound::Included(&p) };
                    c.seek_here(s, &v, self.root, b)?;
                }
                placed = true;
            }
            while let Some((rid, n, i)) = tree_row(s, &v, &mut c, k)? {
                c.advance(s, &v, n, i)?;
                row(k, rid)?;
                done = done.wrapping_add(1);
                if done >= limit {
                    return Ok(done);
                }
            }
            // The next key: going forward, the one where the cursor stands.
            let found = match dir {
                Dir::Fwd => cursor_key(s, &v, spec, &mut c, k, next)?,
                Dir::Rev => self.tree_key(h, spec, Bound::Excluded(k), Dir::Rev, next)?,
            };
            let Some(n) = found else { return Ok(done) };
            (len, after) = (take(key, next.get(..n).unwrap_or(&[]))?, None);
        }
    }

    /// The index's smallest key whose first part is not null, whole at the start of `out`; its length. `None`
    /// when no row holds such a key: the smallest value of a column, which skips nulls, as OQL's `min` does. A
    /// key that does not fit `out` is `Scratch { need }`, the bytes it must hold.
    pub fn min_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        out: &mut [u8],
    ) -> Result<Option<usize>, Fault> {
        // A null first part encodes as its tag 00, below the tag 01 every value of the part starts with.
        let from = if first_nullable(spec) { Bound::Included(&[1u8][..]) } else { Bound::Unbounded };
        self.tree_key(h, spec, from, Dir::Fwd, out)
    }

    /// The index's largest key whose first part is not null, whole at the start of `out`; its length. `None`
    /// when no row holds such a key, as OQL's `max` of a column.
    pub fn max_key<H: PageHost + ?Sized>(
        &self,
        h: &H,
        spec: &KeySpec<'_>,
        out: &mut [u8],
    ) -> Result<Option<usize>, Fault> {
        let Some(len) = self.tree_key(h, spec, Bound::Unbounded, Dir::Rev, out)? else { return Ok(None) };
        // Every key whose first part is null lies below every other.
        if first_nullable(spec) && out.first() == Some(&0) {
            return Ok(None);
        }
        Ok(Some(len))
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
