//! Search, find and descend.

use super::node::{child, count, is_leaf, ko};
use super::shape::Shape;
use super::{Path, Probe};
use crate::consts::{NODE_SIZE, PATH_STACK};
use crate::fault::Fault;
use crate::host::NodeRead;
use core::cmp::Ordering;

/// Where a search for a key ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    /// Entry i of node n holds it.
    Found(u64, u64),
    /// It would go at position j of leaf n.
    Vacant(u64, u64),
}

/// The two shared lengths of one descent: s_lo and s_hi, 0 at the root. Read and written only when
/// `P::PREFIX` is set, so a typed search never touches them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shared {
    pub lo: u64,
    pub hi: u64,
}

/// The position of the probe among the keys of node n: `Ok(i)` when key i equals it, else `Err(j)`, j the
/// number of keys below it. The lower midpoint, the stored key on the left.
#[inline(always)]
pub fn search<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    n: u64,
    p: &P,
    sh: &mut Shared,
) -> Result<Result<u64, u64>, Fault> {
    let node = if S::CHECKED || P::PREFIX { Some(s.shape(super::node::is_leaf(m, n)?)) } else { None };
    let (mut lo, mut hi) = (0u64, count(s, m, n)?);
    // The node's bytes, lent once for every comparison below; when the memory does not lend them whole,
    // each comparison asks for its own part, as without them.
    let whole = if P::PREFIX && hi > 0 { m.bytes(n, 0, NODE_SIZE).ok() } else { None };
    if P::PREFIX && !S::TYPED && S::CHECKED {
        if let Some(r) = whole.as_deref().and_then(|b| p.search_node(m, b, s.ks(), hi)) {
            return Ok(r);
        }
    }
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        let koff = match node {
            Some(ns) => ko(s, &ns, mid),
            None => 8u64.wrapping_add(mid.wrapping_mul(s.ks())),
        };
        let ord = if P::PREFIX {
            let skip = sh.lo.min(sh.hi);
            let (o, shared) = match whole.as_deref() {
                Some(node) => p.cmp_in(m, node, n, koff, skip)?,
                None => p.cmp_from(m, n, koff, skip)?,
            };
            match o {
                Ordering::Less => sh.lo = shared,
                Ordering::Greater => sh.hi = shared,
                Ordering::Equal => {}
            }
            o
        } else {
            p.cmp_at(m, n, koff)?
        };
        match ord {
            Ordering::Less => lo = mid.wrapping_add(1),
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(Ok(mid)),
        }
    }
    Ok(Err(lo))
}

/// The node and index of the probe's entry, if present. Records no path and allocates nothing. An index
/// form counts levels and fails with `PathOverflow` past `PATH_STACK`; a typed find keeps no counter.
///
/// A typed find is always inlined into its caller, as today's typed maps inline theirs: called out of
/// line, the stores of its returned `Result` are merged across its exits before inlining, and the value
/// stays in memory instead of registers. Index forms keep an outlined body.
#[inline(always)]
pub fn find<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    root: u64,
    p: &P,
) -> Result<Option<(u64, u64)>, Fault> {
    if S::TYPED {
        find_body(s, m, root, p)
    } else {
        find_outlined(s, m, root, p)
    }
}

#[inline]
fn find_outlined<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    root: u64,
    p: &P,
) -> Result<Option<(u64, u64)>, Fault> {
    find_body(s, m, root, p)
}

#[inline(always)]
fn find_body<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    root: u64,
    p: &P,
) -> Result<Option<(u64, u64)>, Fault> {
    let mut n = root;
    if n == 0 {
        return Ok(None);
    }
    let mut sh = Shared::default();
    let mut levels = 0u64;
    loop {
        match search(s, m, n, p, &mut sh)? {
            Ok(i) => return Ok(Some((n, i))),
            Err(j) => {
                if is_leaf(m, n)? {
                    return Ok(None);
                }
                if !S::TYPED {
                    levels = levels.wrapping_add(1);
                    if levels >= PATH_STACK {
                        return Err(Fault::PathOverflow);
                    }
                }
                n = child(s, m, n, j)?;
            }
        }
    }
}

/// Searches the non-empty tree from the root, recording each internal node passed and the child index
/// taken; the final node is not pushed.
#[inline]
pub fn descend<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    root: u64,
    p: &P,
    path: &mut Path,
) -> Result<Search, Fault> {
    let mut n = root;
    let mut sh = Shared::default();
    loop {
        match search(s, m, n, p, &mut sh)? {
            Ok(i) => return Ok(Search::Found(n, i)),
            Err(j) => {
                if is_leaf(m, n)? {
                    return Ok(Search::Vacant(n, j));
                }
                path.push(n, j)?;
                n = child(s, m, n, j)?;
            }
        }
    }
}
