//! Path cursors. The only state is the path; "fused" means an empty path. `seek` and
//! `next` read counts, flags, child words and, for a bounded end, one key part through the probe; they never
//! reach a value part.

use super::node::{child, count, is_leaf, ko};
use super::search::{search, Shared};
use super::shape::Shape;
use super::{Path, Probe};
use crate::fault::Fault;
use crate::host::NodeMem;
use core::cmp::Ordering;
use core::ops::Bound;

/// A cursor's direction, a compile-time parameter: the forward cursor's machine code has no direction
/// branch.
pub trait Direction: Copy {
    const REV: bool;
}

#[derive(Clone, Copy, Debug)]
pub struct Fwd;

#[derive(Clone, Copy, Debug)]
pub struct Rev;

impl Direction for Fwd {
    const REV: bool = false;
}

impl Direction for Rev {
    const REV: bool = true;
}

/// The run-time direction of a walk over an index and of a run's key steps; a tree cursor takes its direction
/// as a type instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Fwd,
    Rev,
}

/// A path cursor in direction D.
#[derive(Clone)]
pub struct Cursor<D: Direction> {
    path: Path,
    dir: core::marker::PhantomData<D>,
}

impl<D: Direction> Cursor<D> {
    /// A fused cursor.
    pub fn empty() -> Self {
        Cursor { path: Path::new(), dir: core::marker::PhantomData }
    }

    /// The path (for tokens and tests).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A cursor resumed from a path saved by an earlier `path()`. The frames are not checked here: every
    /// later `next` reads them through the node memory, which checks each reference.
    pub fn from_path(path: Path) -> Self {
        Cursor { path, dir: core::marker::PhantomData }
    }

    /// A cursor at the first entry not before `start` in its direction.
    #[inline(always)]
    pub fn seek<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<Cursor<D>, Fault> {
        let mut c = Self::empty();
        c.seek_here(s, m, root, start)?;
        Ok(c)
    }

    /// Positions this cursor at the first entry not before `start` in its direction, in place: `seek`
    /// without moving the cursor out of a `Result`, which costs a copy of its path per seek. The path is
    /// cleared first; a front end that starts from `empty()` has a zero-filled path, as today's typed
    /// iterators do.
    ///
    /// A typed seek is always inlined into its caller: out of line, its `Result` would come back through
    /// memory, where today's typed seek returns nothing. A typed front end calls it from its own seek, which
    /// traps on a fault and which the optimizer places as it places today's. Index forms keep an outlined
    /// body.
    #[inline(always)]
    pub fn seek_here<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        if S::TYPED {
            self.seek_body(s, m, root, start)
        } else {
            self.seek_outlined(s, m, root, start)
        }
    }

    #[inline]
    fn seek_outlined<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        self.seek_body(s, m, root, start)
    }

    #[inline(always)]
    fn seek_body<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        self.path.clear();
        if root == 0 {
            return Ok(());
        }
        if D::REV {
            self.seek_rev(s, m, root, start)
        } else {
            self.seek_fwd(s, m, root, start)
        }
    }

    /// The entry the cursor stands at, without moving past it: finished frames are popped first; None
    /// once the cursor is exhausted (its path is then empty). With `advance`, the two halves of `next`: a
    /// typed front end that forms the stored key's reference itself checks its end bound between them, on
    /// the same reference it hands out, as today's typed iterators do. Always inlined, like `advance`: out of
    /// line, their `Result`s would come back through memory.
    #[inline(always)]
    pub fn peek<S: Shape, M: NodeMem + ?Sized>(&mut self, s: S, m: &M) -> Result<Option<(u64, u64)>, Fault> {
        while let Some((n, j)) = self.path.last() {
            if D::REV {
                if j == 0 {
                    self.path.pop();
                    continue;
                }
                let i = j.wrapping_sub(1);
                if S::CHECKED && i >= count(s, m, n)? {
                    self.path.set_last_index(i);
                    continue;
                }
                return Ok(Some((n, i)));
            }
            if j >= count(s, m, n)? {
                self.path.pop();
                continue;
            }
            return Ok(Some((n, j)));
        }
        Ok(None)
    }

    /// Moves past entry (n, i), which `peek` returned.
    #[inline(always)]
    pub fn advance<S: Shape, M: NodeMem + ?Sized>(&mut self, s: S, m: &M, n: u64, i: u64) -> Result<(), Fault> {
        if D::REV {
            self.path.set_last_index(i);
            if !is_leaf(m, n)? {
                let c = child(s, m, n, i)?;
                self.rightmost(s, m, c)?;
            }
        } else {
            self.path.set_last_index(i.wrapping_add(1));
            if !is_leaf(m, n)? {
                let c = child(s, m, n, i.wrapping_add(1))?;
                self.leftmost(s, m, c)?;
            }
        }
        Ok(())
    }

    /// Fuses the cursor: it returns nothing more.
    #[inline]
    pub fn clear(&mut self) {
        self.path.clear();
    }

    /// The next (node, index), or None; beyond the bound or the last entry the path is cleared.
    #[inline]
    pub fn next<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        end: Bound<&P>,
    ) -> Result<Option<(u64, u64)>, Fault> {
        if D::REV {
            self.next_rev(s, m, end)
        } else {
            self.next_fwd(s, m, end)
        }
    }

    // ---- forward: a frame (n, i) means entry i of n is next ----

    /// The frames from n down its leftmost path. Always inlined, as today's is: out of line, its `Result`
    /// would come back through memory.
    #[inline(always)]
    fn leftmost<S: Shape, M: NodeMem + ?Sized>(&mut self, s: S, m: &M, mut n: u64) -> Result<(), Fault> {
        loop {
            self.path.push(n, 0)?;
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, 0)?;
        }
    }

    #[inline(always)]
    fn seek_fwd<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        let (p, inclusive) = match start {
            Bound::Unbounded => return self.leftmost(s, m, root),
            Bound::Included(p) => (p, true),
            Bound::Excluded(p) => (p, false),
        };
        let mut n = root;
        let mut sh = Shared::default();
        loop {
            let j = match search(s, m, n, p, &mut sh)? {
                Ok(i) if inclusive => return self.path.push(n, i),
                Ok(i) => {
                    self.path.push(n, i.wrapping_add(1))?;
                    if !is_leaf(m, n)? {
                        let c = child(s, m, n, i.wrapping_add(1))?;
                        self.leftmost(s, m, c)?;
                    }
                    return Ok(());
                }
                Err(j) => j,
            };
            self.path.push(n, j)?;
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, j)?;
        }
    }

    #[inline]
    fn next_fwd<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        end: Bound<&P>,
    ) -> Result<Option<(u64, u64)>, Fault> {
        while let Some((n, i)) = self.path.last() {
            if i >= count(s, m, n)? {
                self.path.pop();
                continue;
            }
            let beyond = match end {
                Bound::Unbounded => false,
                Bound::Included(e) => key_cmp(s, m, n, i, e)? == Ordering::Greater,
                Bound::Excluded(e) => key_cmp(s, m, n, i, e)? != Ordering::Less,
            };
            if beyond {
                self.path.clear();
                return Ok(None);
            }
            self.path.set_last_index(i.wrapping_add(1));
            if !is_leaf(m, n)? {
                let c = child(s, m, n, i.wrapping_add(1))?;
                self.leftmost(s, m, c)?;
            }
            return Ok(Some((n, i)));
        }
        Ok(None)
    }

    // ---- reverse: a frame (n, j) means entry j − 1 of n is next ----

    #[inline(always)]
    fn rightmost<S: Shape, M: NodeMem + ?Sized>(&mut self, s: S, m: &M, mut n: u64) -> Result<(), Fault> {
        loop {
            let c = count(s, m, n)?;
            self.path.push(n, c)?;
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, c)?;
        }
    }

    #[inline(always)]
    fn seek_rev<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        let (p, inclusive) = match start {
            Bound::Unbounded => return self.rightmost(s, m, root),
            Bound::Included(p) => (p, true),
            Bound::Excluded(p) => (p, false),
        };
        let mut n = root;
        let mut sh = Shared::default();
        loop {
            let j = match search(s, m, n, p, &mut sh)? {
                Ok(i) if inclusive => return self.path.push(n, i.wrapping_add(1)),
                Ok(i) => {
                    self.path.push(n, i)?;
                    if !is_leaf(m, n)? {
                        let c = child(s, m, n, i)?;
                        self.rightmost(s, m, c)?;
                    }
                    return Ok(());
                }
                Err(j) => j,
            };
            self.path.push(n, j)?;
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, j)?;
        }
    }

    fn next_rev<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        end: Bound<&P>,
    ) -> Result<Option<(u64, u64)>, Fault> {
        while let Some((n, j)) = self.path.last() {
            if j == 0 {
                self.path.pop();
                continue;
            }
            let i = j.wrapping_sub(1);
            if S::CHECKED && i >= count(s, m, n)? {
                // A frame past a clamped count: skip it rather than read beyond the node.
                self.path.set_last_index(i);
                continue;
            }
            let beyond = match end {
                Bound::Unbounded => false,
                Bound::Included(e) => key_cmp(s, m, n, i, e)? == Ordering::Less,
                Bound::Excluded(e) => key_cmp(s, m, n, i, e)? != Ordering::Greater,
            };
            if beyond {
                self.path.clear();
                return Ok(None);
            }
            self.path.set_last_index(i);
            if !is_leaf(m, n)? {
                let c = child(s, m, n, i)?;
                self.rightmost(s, m, c)?;
            }
            return Ok(Some((n, i)));
        }
        Ok(None)
    }
}

/// Stored key i of node n against the probe, from byte 0.
#[inline(always)]
fn key_cmp<S: Shape, M: NodeMem + ?Sized, P: Probe<M>>(s: S, m: &M, n: u64, i: u64, p: &P) -> Result<Ordering, Fault> {
    let koff = if S::CHECKED {
        let sh = s.shape(is_leaf(m, n)?);
        ko(s, &sh, i)
    } else {
        8u64.wrapping_add(i.wrapping_mul(s.ks()))
    };
    p.cmp_at(m, n, koff)
}
