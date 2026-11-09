//! Insert and split, `PBTreeMap`'s `insert_new` and `insert_at` step for step.

use super::node::{
    count, is_leaf, move_children, move_entries, read_pending, set_child, set_count, shape_of, write_pending,
};
use super::shape::Shape;
use super::Path;
use crate::fault::Fault;
use crate::host::{NodeMem, NodeRead, Pending};
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
/// entries at h = ⌈c/2⌉; a split root gets a new root, written to `root`. Returns where `p`'s entry ends.
///
/// An inline hint: the optimizer then inlines a typed insert into the one place that inserts a new
/// entry, as it inlines today's typed maps' insert there.
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
    loop {
        let cnt = count(s, m, n)?;
        let (leaf, sh) = shape_of(s, m, n)?;
        let c = sh.cap;
        if S::CHECKED && i > cnt {
            i = cnt;
        }
        if cnt < c {
            move_entries(s, m, n, i, n, i.wrapping_add(1), cnt.wrapping_sub(i))?;
            if !leaf {
                move_children(s, m, n, i.wrapping_add(1), n, i.wrapping_add(2), cnt.wrapping_sub(i))?;
                set_child(s, m, n, i.wrapping_add(1), p.child())?;
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
                    set_child(s, m, r, 0, p.child())?;
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
                    set_child(s, m, n, i.wrapping_add(1), p.child())?;
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
                    set_child(s, m, r, i.wrapping_sub(h), p.child())?;
                    move_children(s, m, n, i.wrapping_add(1), r, i.wrapping_sub(h).wrapping_add(1), c.wrapping_sub(i))?;
                }
                placed.get_or_insert((r, at));
                core::mem::swap(p, &mut up);
            }
        }
        set_count(m, n, h)?;
        set_count(m, r, c.wrapping_sub(h))?;
        p.set_child(r);
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
                set_count(m, q, 1)?;
                *root = q;
                return Ok(placed.unwrap_or((q, 0)));
            }
        }
    }
}

/// Whether node n is a leaf (re-exported for front ends).
#[inline(always)]
pub fn leaf<M: NodeRead + ?Sized>(m: &M, n: u64) -> Result<bool, Fault> {
    is_leaf(m, n)
}
