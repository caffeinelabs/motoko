//! Path cursors. The only state is the path; "fused" means an empty path. `seek`, `peek` and `advance` read
//! counts, flags, child words and, for a seek, key parts through the probe; they never reach a value part.

use super::node::{child, count, is_leaf};
use super::search::{search, Shared};
use super::shape::Shape;
use super::{Path, Probe};
use crate::fault::Fault;
use crate::host::NodeRead;
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

/// The run-time direction of a walk over an index; a tree cursor takes its direction as a type instead.
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

    /// A cursor at the first entry not before `start` in its direction.
    #[inline(always)]
    pub fn seek<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
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
    /// cleared first.
    #[inline(always)]
    pub fn seek_here<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        self.seek_outlined(s, m, root, start)
    }

    #[inline]
    fn seek_outlined<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
        &mut self,
        s: S,
        m: &M,
        root: u64,
        start: Bound<&P>,
    ) -> Result<(), Fault> {
        self.seek_body(s, m, root, start)
    }

    #[inline(always)]
    fn seek_body<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
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
    /// once the cursor is exhausted (its path is then empty). With `advance`, the two halves of `next`. Always
    /// inlined, like `advance`: out of line, their `Result`s would come back through memory.
    #[inline(always)]
    pub fn peek<S: Shape, M: NodeRead + ?Sized>(&mut self, s: S, m: &M) -> Result<Option<(u64, u64)>, Fault> {
        while let Some((n, j)) = self.path.last() {
            if D::REV {
                if j == 0 {
                    self.path.pop();
                    continue;
                }
                let i = j.wrapping_sub(1);
                if i >= count(s, m, n)? {
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
    pub fn advance<S: Shape, M: NodeRead + ?Sized>(&mut self, s: S, m: &M, n: u64, i: u64) -> Result<(), Fault> {
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

    // ---- forward: a frame (n, i) means entry i of n is next ----

    /// The frames from n down its leftmost path. Always inlined, as today's is: out of line, its `Result`
    /// would come back through memory.
    #[inline(always)]
    fn leftmost<S: Shape, M: NodeRead + ?Sized>(&mut self, s: S, m: &M, mut n: u64) -> Result<(), Fault> {
        loop {
            self.path.push(n, 0);
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, 0)?;
        }
    }

    #[inline(always)]
    fn seek_fwd<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
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
                Ok(i) if inclusive => {
                    self.path.push(n, i);
                    return Ok(());
                }
                Ok(i) => {
                    self.path.push(n, i.wrapping_add(1));
                    if !is_leaf(m, n)? {
                        let c = child(s, m, n, i.wrapping_add(1))?;
                        self.leftmost(s, m, c)?;
                    }
                    return Ok(());
                }
                Err(j) => j,
            };
            self.path.push(n, j);
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, j)?;
        }
    }

    // ---- reverse: a frame (n, j) means entry j − 1 of n is next ----

    #[inline(always)]
    fn rightmost<S: Shape, M: NodeRead + ?Sized>(&mut self, s: S, m: &M, mut n: u64) -> Result<(), Fault> {
        loop {
            let c = count(s, m, n)?;
            self.path.push(n, c);
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, c)?;
        }
    }

    #[inline(always)]
    fn seek_rev<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
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
            // An excluded probe found at entry i of node n goes on as one placed just below it: down child i, all
            // of whose entries lie below the probe, to its rightmost leaf entry.
            let j = match search(s, m, n, p, &mut sh)? {
                Ok(i) if inclusive => {
                    self.path.push(n, i.wrapping_add(1));
                    return Ok(());
                }
                Ok(j) | Err(j) => j,
            };
            self.path.push(n, j);
            if is_leaf(m, n)? {
                return Ok(());
            }
            n = child(s, m, n, j)?;
        }
    }
}
