//! Node offsets and the primitive moves, all through the node host's field methods.
//!
//! For a typed shape these compute today's offsets with today's arithmetic (wrapping, unchecked); for an
//! index shape every count is clamped to the node's capacity, every entry index to c − 1 and every child
//! index to c, so every offset lies inside the node by construction.

use super::shape::{NodeShape, Shape};
use crate::consts::FLAG_LEAF;
use crate::fault::Fault;
use crate::host::{NodeMem, NodeRead};

#[inline(always)]
pub fn is_leaf<M: NodeRead + ?Sized>(m: &M, n: u64) -> Result<bool, Fault> {
    Ok(m.flags(n)? & FLAG_LEAF != 0)
}

#[inline(always)]
pub fn shape_of<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64) -> Result<(bool, NodeShape), Fault> {
    let leaf = is_leaf(m, n)?;
    Ok((leaf, s.shape(leaf)))
}

#[inline(always)]
pub fn cap<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64) -> Result<u64, Fault> {
    Ok(shape_of(s, m, n)?.1.cap)
}

/// The count of node n, clamped to its capacity when the shape is checked.
#[inline(always)]
pub fn count<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64) -> Result<u64, Fault> {
    let c = u64::from(m.count(n)?);
    if S::CHECKED {
        Ok(c.min(cap(s, m, n)?))
    } else {
        Ok(c)
    }
}

#[inline(always)]
pub fn set_count<M: NodeMem + ?Sized>(m: &mut M, n: u64, c: u64) -> Result<(), Fault> {
    // A count is at most a capacity, below 4,080.
    #[allow(clippy::cast_possible_truncation)]
    m.set_count(n, c as u16)
}

#[inline(always)]
fn clamp_entry<S: Shape>(sh: &NodeShape, i: u64) -> u64 {
    if S::CHECKED {
        i.min(sh.cap.saturating_sub(1))
    } else {
        i
    }
}

/// Key offset of entry i: 8 + i·ks.
#[inline(always)]
pub fn ko<S: Shape>(s: S, sh: &NodeShape, i: u64) -> u64 {
    8u64.wrapping_add(clamp_entry::<S>(sh, i).wrapping_mul(s.ks()))
}

/// Value offset of entry i: voff + i·vs.
#[inline(always)]
pub fn vo<S: Shape>(s: S, sh: &NodeShape, i: u64) -> u64 {
    sh.voff.wrapping_add(clamp_entry::<S>(sh, i).wrapping_mul(s.vs()))
}

/// Child offset of child j of an internal node: coff + 8j.
#[inline(always)]
pub fn co<S: Shape>(s: S, j: u64) -> u64 {
    let internal = s.shape(false);
    let j = if S::CHECKED { j.min(internal.cap) } else { j };
    internal.coff.wrapping_add(j.wrapping_mul(8))
}

#[inline(always)]
pub fn child<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64, j: u64) -> Result<u64, Fault> {
    m.word(n, co(s, j))
}

#[inline(always)]
pub fn set_child<S: Shape, M: NodeMem + ?Sized>(s: S, m: &mut M, n: u64, j: u64, c: u64) -> Result<(), Fault> {
    m.set_word(n, co(s, j), c)
}

/// Clamps a run length to what fits from index i of a node of capacity c (index forms only).
#[inline(always)]
fn clamp_run<S: Shape>(cap: u64, i: u64, k: u64) -> u64 {
    if S::CHECKED {
        k.min(cap.saturating_sub(i.min(cap)))
    } else {
        k
    }
}

/// Moves k entries from entry si of s to entry di of d: the key parts, then the value parts, each node at
/// its own voff; overlapping ranges included. k = 0 does nothing.
#[inline(always)]
pub fn move_entries<S: Shape, M: NodeMem + ?Sized>(
    s: S,
    m: &mut M,
    src: u64,
    si: u64,
    dst: u64,
    di: u64,
    k: u64,
) -> Result<(), Fault> {
    if k == 0 {
        return Ok(());
    }
    if !S::CHECKED {
        // Unchecked (typed maps): today's sequence. The key parts first, whose offsets need no shape, then
        // each node's value offset from its own leaf bit, read after the key move as today's code reads
        // it, then the value parts.
        let ks = s.ks();
        m.copy(src, key_off(ks, si), dst, key_off(ks, di), k.wrapping_mul(ks))?;
        let from = value_off(s, m, src, si)?;
        let to = value_off(s, m, dst, di)?;
        return m.copy(src, from, dst, to, k.wrapping_mul(s.vs()));
    }
    let (_, ss) = shape_of(s, m, src)?;
    let (_, ds) = if dst == src { (false, ss) } else { shape_of(s, m, dst)? };
    let k = clamp_run::<S>(ds.cap, di, clamp_run::<S>(ss.cap, si, k));
    m.copy(src, ko(s, &ss, si), dst, ko(s, &ds, di), k.wrapping_mul(s.ks()))?;
    m.copy(src, vo(s, &ss, si), dst, vo(s, &ds, di), k.wrapping_mul(s.vs()))
}

/// Key offset of entry i, unclamped: 8 + i·ks.
#[inline(always)]
fn key_off(ks: u64, i: u64) -> u64 {
    8u64.wrapping_add(i.wrapping_mul(ks))
}

/// Value offset of entry i of node n, unclamped: the node's voff by its leaf bit, + i·vs.
#[inline(always)]
fn value_off<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64, i: u64) -> Result<u64, Fault> {
    let voff = s.shape(is_leaf(m, n)?).voff;
    Ok(voff.wrapping_add(i.wrapping_mul(s.vs())))
}

/// Moves k children from child sj of internal node s to child dj of internal node d.
#[inline(always)]
pub fn move_children<S: Shape, M: NodeMem + ?Sized>(
    s: S,
    m: &mut M,
    src: u64,
    sj: u64,
    dst: u64,
    dj: u64,
    k: u64,
) -> Result<(), Fault> {
    if k == 0 {
        return Ok(());
    }
    let slots = s.shape(false).cap.wrapping_add(1);
    let k = clamp_run::<S>(slots, dj, clamp_run::<S>(slots, sj, k));
    m.copy(src, co(s, sj), dst, co(s, dj), k.wrapping_mul(8))
}

/// Writes the pending entry's parts as entry i of node n.
#[inline(always)]
pub fn write_pending<S: Shape, M: NodeMem + ?Sized>(
    s: S,
    m: &mut M,
    n: u64,
    i: u64,
    p: &M::Entry<'_>,
) -> Result<(), Fault> {
    let (_, sh) = shape_of(s, m, n)?;
    m.put_entry(n, ko(s, &sh, i), vo(s, &sh, i), p)
}

/// Moves entry i of node n into the pending entry.
#[inline(always)]
pub fn read_pending<S: Shape, M: NodeMem + ?Sized>(
    s: S,
    m: &mut M,
    n: u64,
    i: u64,
    p: &mut M::Entry<'_>,
) -> Result<(), Fault> {
    let (_, sh) = shape_of(s, m, n)?;
    m.take_entry(n, ko(s, &sh, i), vo(s, &sh, i), p)
}
