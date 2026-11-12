//! Search, find and descend.

use super::node::{child, child_rows, count, entry_rows, is_leaf, ko};
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
/// `P::PREFIX` is set.
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
    let ns = s.shape(super::node::is_leaf(m, n)?);
    let (mut lo, mut hi) = (0u64, count(s, m, n)?);
    // The node's bytes, lent once for every comparison below; when the memory does not lend them whole,
    // each comparison asks for its own part, as without them.
    let whole = if P::PREFIX && hi > 0 { m.bytes(n, 0, NODE_SIZE).ok() } else { None };
    if P::PREFIX {
        if let Some(r) = whole.and_then(|b| p.search_node(m, b, s.ks(), hi)) {
            return Ok(r);
        }
    }
    while lo < hi {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
        let koff = ko(s, &ns, mid);
        let ord = if P::PREFIX {
            let skip = sh.lo.min(sh.hi);
            let (o, shared) = match whole {
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

/// The node and index of the probe's entry, if present, in the tree whose root is `root` (an empty tree, root
/// 0, is not searched). Records no path and allocates nothing. It counts levels and fails with `PathOverflow`
/// past `PATH_STACK`.
#[inline(always)]
pub fn find<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(
    s: S,
    m: &M,
    root: u64,
    p: &P,
) -> Result<Option<(u64, u64)>, Fault> {
    find_outlined(s, m, root, p)
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
    let mut sh = Shared::default();
    let mut levels = 0u64;
    loop {
        match search(s, m, n, p, &mut sh)? {
            Ok(i) => return Ok(Some((n, i))),
            Err(j) => {
                if is_leaf(m, n)? {
                    return Ok(None);
                }
                levels = levels.wrapping_add(1);
                if levels >= PATH_STACK {
                    return Err(Fault::PathOverflow);
                }
                n = child(s, m, n, j)?;
            }
        }
    }
}

/// In a counted tree whose root is `root` (not 0): the row entries before the probe's place, which no entry
/// equals. One descent, adding at each node the row entries before the place and, internal, the rows under the
/// children before it. It fails with `PathOverflow` past `PATH_STACK` levels.
pub fn rank<S: Shape, M: NodeRead + ?Sized, P: Probe<M>>(s: S, m: &M, root: u64, p: &P) -> Result<u64, Fault> {
    let (mut n, mut sh, mut r) = (root, Shared::default(), 0u64);
    for _ in 0..PATH_STACK {
        let (Ok(j) | Err(j)) = search(s, m, n, p, &mut sh)?;
        r = r.wrapping_add(entry_rows(s, m, n, 0, j)?);
        if is_leaf(m, n)? {
            return Ok(r);
        }
        r = r.wrapping_add(child_rows(s, m, n, j)?);
        n = child(s, m, n, j)?;
    }
    Err(Fault::PathOverflow)
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
                path.push(n, j);
                n = child(s, m, n, j)?;
            }
        }
    }
}
