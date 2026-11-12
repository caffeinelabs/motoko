//! Insert and split, `PBTreeMap`'s `insert_new` and `insert_at` step for step. In a counted tree a split also
//! counts the rows of its two nodes, for the slots its parent keeps them in.

use super::node::{
    count, move_children, move_entries, node_rows, read_pending, set_child, set_count, set_rows, shape_of,
    write_pending,
};
use super::shape::Shape;
use super::Path;
use crate::fault::Fault;
use crate::host::{NodeMem, Pending};
use core::cmp::Ordering;

/// The entry of an empty tree: a new leaf holding `p`; returns the new root, the entry being (root, 0).
#[inline]
pub fn insert_first<S: Shape, M: NodeMem>(s: S, m: &mut M, p: &M::Entry<'_>) -> Result<u64, Fault> {
    let n = m.alloc(true)?;
    write_pending(s, m, n, 0, p)?;
    set_count(m, n, 1)?;
    Ok(n)
}

/// Inserts the pending entry `p` at position `i` of node `n`, which `path` leads to from the root, with,
/// in an internal node, `p.child()` as child i + 1. A full node of capacity c splits as if holding c + 1
/// entries at h = ⌈c/2⌉; a split root gets a new root, written to `root`. Returns where `p`'s entry ends. In a
/// counted tree the caller has added the entry's row, when it is one, to the rows of every frame of `path`.
#[inline]
pub fn insert_at<'k, S: Shape, M: NodeMem>(
    s: S,
    m: &mut M,
    root: &mut u64,
    mut n: u64,
    mut i: u64,
    p: &mut M::Entry<'k>,
    path: &mut Path,
) -> Result<(u64, u64), Fault> {
    let mut placed: Option<(u64, u64)> = None;
    // In a counted tree, after a split: the rows of its left node, child i of n, and of its right one, `p.child()`.
    let mut rows: Option<(u64, u64)> = None;
    loop {
        let cnt = count(s, m, n)?;
        let (leaf, sh) = shape_of(s, m, n)?;
        let c = sh.cap;
        if i > cnt {
            i = cnt;
        }
        if let Some((l, _)) = rows {
            set_rows(s, m, n, i, l)?;
        }
        if cnt < c {
            move_entries(s, m, n, i, n, i.wrapping_add(1), cnt.wrapping_sub(i))?;
            if !leaf {
                move_children(s, m, n, i.wrapping_add(1), n, i.wrapping_add(2), cnt.wrapping_sub(i))?;
                put_child(s, m, n, i.wrapping_add(1), p.child(), rows)?;
            }
            write_pending(s, m, n, i, p)?;
            set_count(m, n, cnt.wrapping_add(1))?;
            return Ok(placed.unwrap_or((n, i)));
        }
        let r = m.alloc(leaf)?;
        // h = ⌈c/2⌉, written as c − ⌊c/2⌋: the form the optimizer gives today's `c.div_ceil(2)`, reused
        // for c − h.
        let h = c.wrapping_sub(c / 2);
        match i.cmp(&h) {
            Ordering::Equal => {
                // `p` itself moves up; the right node takes entries [h, c).
                move_entries(s, m, n, h, r, 0, c.wrapping_sub(h))?;
                if !leaf {
                    put_child(s, m, r, 0, p.child(), rows)?;
                    move_children(s, m, n, h.wrapping_add(1), r, 1, c.wrapping_sub(h))?;
                }
            }
            Ordering::Less => {
                // Entry h − 1 moves up; `p` goes in at i.
                let mut up = <M::Entry<'k> as Pending>::empty();
                move_entries(s, m, n, h, r, 0, c.wrapping_sub(h))?;
                if !leaf {
                    move_children(s, m, n, h, r, 0, c.wrapping_sub(h).wrapping_add(1))?;
                }
                read_pending(s, m, n, h.wrapping_sub(1), &mut up)?;
                let k = h.wrapping_sub(1).wrapping_sub(i);
                move_entries(s, m, n, i, n, i.wrapping_add(1), k)?;
                if !leaf {
                    move_children(s, m, n, i.wrapping_add(1), n, i.wrapping_add(2), k)?;
                    put_child(s, m, n, i.wrapping_add(1), p.child(), rows)?;
                }
                write_pending(s, m, n, i, p)?;
                placed.get_or_insert((n, i));
                core::mem::swap(p, &mut up);
            }
            Ordering::Greater => {
                // Entry h moves up; `p` goes into the right node at i − h − 1.
                let mut up = <M::Entry<'k> as Pending>::empty();
                read_pending(s, m, n, h, &mut up)?;
                let at = i.wrapping_sub(h).wrapping_sub(1);
                move_entries(s, m, n, h.wrapping_add(1), r, 0, at)?;
                write_pending(s, m, r, at, p)?;
                move_entries(s, m, n, i, r, i.wrapping_sub(h), c.wrapping_sub(i))?;
                if !leaf {
                    move_children(s, m, n, h.wrapping_add(1), r, 0, i.wrapping_sub(h))?;
                    put_child(s, m, r, i.wrapping_sub(h), p.child(), rows)?;
                    move_children(s, m, n, i.wrapping_add(1), r, i.wrapping_sub(h).wrapping_add(1), c.wrapping_sub(i))?;
                }
                placed.get_or_insert((r, at));
                core::mem::swap(p, &mut up);
            }
        }
        set_count(m, n, h)?;
        set_count(m, r, c.wrapping_sub(h))?;
        p.set_child(r);
        if M::COUNTED {
            rows = Some((node_rows(s, m, n)?, node_rows(s, m, r)?));
        }
        match path.pop() {
            Some((parent, j)) => {
                n = parent;
                i = j;
            }
            None => {
                let q = m.alloc(false)?;
                write_pending(s, m, q, 0, p)?;
                set_child(s, m, q, 0, n)?;
                set_child(s, m, q, 1, r)?;
                if let Some((a, b)) = rows {
                    set_rows(s, m, q, 0, a)?;
                    set_rows(s, m, q, 1, b)?;
                }
                set_count(m, q, 1)?;
                *root = q;
                return Ok(placed.unwrap_or((q, 0)));
            }
        }
    }
}

/// Sets child j of internal node n to `x`, and in a counted tree the rows under it, the second of `rows`.
#[inline(always)]
fn put_child<S: Shape, M: NodeMem>(
    s: S,
    m: &mut M,
    n: u64,
    j: u64,
    x: u64,
    rows: Option<(u64, u64)>,
) -> Result<(), Fault> {
    set_child(s, m, n, j, x)?;
    match rows {
        Some((_, r)) => set_rows(s, m, n, j, r),
        None => Ok(()),
    }
}
