//! Remove with merge or rotation, `PBTreeMap`'s `remove` and `rebalance` step for step.
//! The front end disposes of the removed entry's parts first; the algorithm allocates nothing and hands
//! back each node emptied by a merge, bottom-up, then an emptied root, last. In a counted tree it also takes the
//! removed row, and a row moved up from a leaf, from the rows its parents keep, and moves the rows a merge or a
//! rotation moves.

use super::node::{
    add_rows, cap, child, count, entry_rows, is_leaf, move_children, move_entries, rows, set_child, set_count, set_rows,
};
use super::shape::Shape;
use super::Path;
use crate::fault::Fault;
use crate::host::NodeMem;

/// Removes entry i of node n, which `path` (from `descend`) leads to; rebalances up the path and may
/// change `root`.
///
#[inline]
pub fn remove_found<S: Shape, M: NodeMem>(
    s: S,
    m: &mut M,
    root: &mut u64,
    n: u64,
    i: u64,
    path: &mut Path,
) -> Result<(), Fault> {
    let above = path.len();
    // In a counted tree: whether the entry removed is a row (1) or not (0).
    let gone = if M::COUNTED { entry_rows(s, m, n, i, i.wrapping_add(1))? } else { 0 };
    let (leaf, gap) = if is_leaf(m, n)? {
        (n, i)
    } else {
        path.push(n, i);
        let mut x = child(s, m, n, i)?;
        while !is_leaf(m, x)? {
            let c = count(s, m, x)?;
            path.push(x, c);
            x = child(s, m, x, c)?;
        }
        let last = count(s, m, x)?.wrapping_sub(1);
        if M::COUNTED {
            // The entry that moves up into n leaves the subtrees of the frames from n's on.
            let moved = entry_rows(s, m, x, last, last.wrapping_add(1))?;
            add_rows(s, m, path.frames().get(above..).unwrap_or(&[]), moved.wrapping_neg())?;
        }
        move_entries(s, m, x, last, n, i, 1)?;
        (x, last)
    };
    if gone != 0 {
        // The entry removed leaves the subtrees of the frames above it.
        add_rows(s, m, path.frames().get(..above).unwrap_or(&[]), u64::MAX)?;
    }
    let cnt = count(s, m, leaf)?;
    move_entries(s, m, leaf, gap.wrapping_add(1), leaf, gap, cnt.wrapping_sub(gap).wrapping_sub(1))?;
    set_count(m, leaf, cnt.wrapping_sub(1))?;
    rebalance(s, m, root, leaf, path)
}

/// Restores the minimum entry counts from node n, which just lost an entry, up the path. Inlined into its
/// one caller, as today's code inlines it into the removal.
#[inline(always)]
fn rebalance<S: Shape, M: NodeMem>(s: S, m: &mut M, root: &mut u64, mut n: u64, path: &mut Path) -> Result<(), Fault> {
    loop {
        let cnt = count(s, m, n)?;
        let Some((p, j)) = path.pop() else {
            if cnt == 0 {
                *root = if is_leaf(m, n)? { 0 } else { child(s, m, n, 0)? };
                m.hand_back(n)?;
            }
            return Ok(());
        };
        let c = cap(s, m, n)?;
        if cnt >= c / 2 {
            return Ok(());
        }
        let leaf = is_leaf(m, n)?;
        let pc = count(s, m, p)?;
        let (left, right, sep) =
            if j > 0 { (child(s, m, p, j.wrapping_sub(1))?, n, j.wrapping_sub(1)) } else { (n, child(s, m, p, 1)?, 0) };
        let (lc, rc) = (count(s, m, left)?, count(s, m, right)?);
        if lc.wrapping_add(rc) < c {
            if M::COUNTED {
                // The merged node holds the rows of both and the separator's.
                let r = rows(s, m, p, sep)?.wrapping_add(rows(s, m, p, sep.wrapping_add(1))?);
                let r = r.wrapping_add(entry_rows(s, m, p, sep, sep.wrapping_add(1))?);
                set_rows(s, m, p, sep, r)?;
            }
            move_entries(s, m, p, sep, left, lc, 1)?;
            move_entries(s, m, right, 0, left, lc.wrapping_add(1), rc)?;
            if !leaf {
                move_children(s, m, right, 0, left, lc.wrapping_add(1), rc.wrapping_add(1))?;
            }
            set_count(m, left, lc.wrapping_add(1).wrapping_add(rc))?;
            let tail = pc.wrapping_sub(sep).wrapping_sub(1);
            move_entries(s, m, p, sep.wrapping_add(1), p, sep, tail)?;
            move_children(s, m, p, sep.wrapping_add(2), p, sep.wrapping_add(1), tail)?;
            set_count(m, p, pc.wrapping_sub(1))?;
            m.hand_back(right)?;
            n = p;
        } else if j > 0 {
            // The left sibling's last entry replaces the separator, which moves to the node's front.
            move_entries(s, m, n, 0, n, 1, rc)?;
            move_entries(s, m, p, sep, n, 0, 1)?;
            move_entries(s, m, left, lc.wrapping_sub(1), p, sep, 1)?;
            if !leaf {
                move_children(s, m, n, 0, n, 1, rc.wrapping_add(1))?;
                let c0 = child(s, m, left, lc)?;
                set_child(s, m, n, 0, c0)?;
                if M::COUNTED {
                    let r0 = rows(s, m, left, lc)?;
                    set_rows(s, m, n, 0, r0)?;
                }
            }
            set_count(m, left, lc.wrapping_sub(1))?;
            set_count(m, n, rc.wrapping_add(1))?;
            return rotated(s, m, p, sep, n, 0, (!leaf).then_some(0), false);
        } else {
            // The right sibling's first entry replaces the separator, which moves to the node's end.
            move_entries(s, m, p, sep, n, lc, 1)?;
            move_entries(s, m, right, 0, p, sep, 1)?;
            move_entries(s, m, right, 1, right, 0, rc.wrapping_sub(1))?;
            if !leaf {
                let c0 = child(s, m, right, 0)?;
                set_child(s, m, n, lc.wrapping_add(1), c0)?;
                if M::COUNTED {
                    let r0 = rows(s, m, right, 0)?;
                    set_rows(s, m, n, lc.wrapping_add(1), r0)?;
                }
                move_children(s, m, right, 1, right, 0, rc)?;
            }
            set_count(m, right, rc.wrapping_sub(1))?;
            set_count(m, n, lc.wrapping_add(1))?;
            return rotated(s, m, p, sep, n, lc, (!leaf).then_some(lc.wrapping_add(1)), true);
        }
    }
}

/// In a counted tree, after a rotation through separator sep of p: node n, child sep of p when `left`, else child
/// sep + 1, took entry i, the separator, and, internal, child k from its sibling, whose entry now separates them.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn rotated<S: Shape, M: NodeMem>(
    s: S,
    m: &mut M,
    p: u64,
    sep: u64,
    n: u64,
    i: u64,
    k: Option<u64>,
    left: bool,
) -> Result<(), Fault> {
    if !M::COUNTED {
        return Ok(());
    }
    let c = match k {
        Some(k) => rows(s, m, n, k)?,
        None => 0,
    };
    let gain = entry_rows(s, m, n, i, i.wrapping_add(1))?.wrapping_add(c);
    let loss = entry_rows(s, m, p, sep, sep.wrapping_add(1))?.wrapping_add(c);
    let (to_n, from) = if left { (sep, sep.wrapping_add(1)) } else { (sep.wrapping_add(1), sep) };
    add_rows(s, m, &[(p, to_n)], gain)?;
    add_rows(s, m, &[(p, from)], loss.wrapping_neg())
}
