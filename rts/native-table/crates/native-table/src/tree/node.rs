//! Node offsets and the primitive moves, all through the node host's field methods.
//!
//! Every count is clamped to the node's capacity, every entry index to c − 1 and every child index to c, so
//! every offset lies inside the node by construction.

use super::shape::{NodeShape, Shape};
use crate::consts::{FLAG_LEAF, F_COUNT, PATH_STACK};
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

/// The count of node n, clamped to its capacity.
#[inline(always)]
pub fn count<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64) -> Result<u64, Fault> {
    let c = u64::from(m.count(n)?);
    Ok(c.min(cap(s, m, n)?))
}

#[inline(always)]
pub fn set_count<M: NodeMem + ?Sized>(m: &mut M, n: u64, c: u64) -> Result<(), Fault> {
    // A count is at most a capacity, below 4,080.
    #[allow(clippy::cast_possible_truncation)]
    m.set_count(n, c as u16)
}

#[inline(always)]
fn clamp_entry(sh: &NodeShape, i: u64) -> u64 {
    i.min(sh.cap.saturating_sub(1))
}

/// Key offset of entry i: 8 + i·ks.
#[inline(always)]
pub fn ko<S: Shape>(s: S, sh: &NodeShape, i: u64) -> u64 {
    8u64.wrapping_add(clamp_entry(sh, i).wrapping_mul(s.ks()))
}

/// Value offset of entry i: voff + i·vs.
#[inline(always)]
pub fn vo<S: Shape>(s: S, sh: &NodeShape, i: u64) -> u64 {
    sh.voff.wrapping_add(clamp_entry(sh, i).wrapping_mul(s.vs()))
}

/// Child offset of child j of an internal node: coff + 8j.
#[inline(always)]
pub fn co<S: Shape>(s: S, j: u64) -> u64 {
    let internal = s.shape(false);
    let j = j.min(internal.cap);
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

// ---- counted trees: row entries are the entries whose field has no `F_COUNT`

/// The count flag in the first byte of an entry's big-endian field, the last 8 bytes of its key part.
const COUNT_BYTE: u8 = 0x40;
const _: () = assert!(F_COUNT == (COUNT_BYTE as u64) << 56);

/// Offset of the rows under child j of a counted internal node, after its children: coff + 8·(c + 1) + 8j.
#[inline(always)]
fn to<S: Shape>(s: S, j: u64) -> u64 {
    let internal = s.shape(false);
    internal.coff.wrapping_add(internal.cap.wrapping_add(1).wrapping_add(j.min(internal.cap)).wrapping_mul(8))
}

/// The rows under child j of internal node n.
#[inline(always)]
pub fn rows<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64, j: u64) -> Result<u64, Fault> {
    m.word(n, to(s, j))
}

#[inline(always)]
pub fn set_rows<S: Shape, M: NodeMem + ?Sized>(s: S, m: &mut M, n: u64, j: u64, r: u64) -> Result<(), Fault> {
    m.set_word(n, to(s, j), r)
}

/// Adds `d` to the rows under the child each frame (n, j) of `frames` took; `d` = `u64::MAX` takes one away.
#[inline(always)]
pub fn add_rows<S: Shape, M: NodeMem + ?Sized>(s: S, m: &mut M, frames: &[(u64, u64)], d: u64) -> Result<(), Fault> {
    for &(n, j) in frames {
        m.add_word(n, to(s, j), d)?;
    }
    Ok(())
}

/// The row entries among the key parts of `ks` bytes that `run` holds.
#[inline(always)]
pub fn rows_in(run: &[u8], ks: u64) -> u64 {
    let (Ok(mut at), Ok(ks)) = (usize::try_from(ks.wrapping_sub(8)), usize::try_from(ks.max(8))) else { return 0 };
    let mut r = 0u64;
    while let Some(&b) = run.get(at) {
        r = r.wrapping_add(u64::from(b & COUNT_BYTE == 0));
        at = at.wrapping_add(ks);
    }
    r
}

/// The row entries among entries [i, k) of node n.
pub fn entry_rows<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64, i: u64, k: u64) -> Result<u64, Fault> {
    if k <= i {
        return Ok(0);
    }
    let (_, sh) = shape_of(s, m, n)?;
    let ks = s.ks();
    Ok(rows_in(m.bytes(n, ko(s, &sh, i), clamp_run(sh.cap, i, k.wrapping_sub(i)).wrapping_mul(ks))?, ks))
}

/// The rows under children [0, j) of counted internal node n.
pub fn child_rows<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64, j: u64) -> Result<u64, Fault> {
    let run = m.bytes(n, to(s, 0), j.min(s.shape(false).cap.wrapping_add(1)).wrapping_mul(8))?;
    Ok(run.chunks_exact(8).fold(0, |t, w| t.wrapping_add(w.first_chunk().map_or(0, |w| u64::from_le_bytes(*w)))))
}

/// The rows of node n's subtree in a counted tree: its row entries and, internal, the rows under its children.
pub fn node_rows<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, n: u64) -> Result<u64, Fault> {
    let c = count(s, m, n)?;
    let r = entry_rows(s, m, n, 0, c)?;
    if is_leaf(m, n)? {
        return Ok(r);
    }
    Ok(r.wrapping_add(child_rows(s, m, n, c.wrapping_add(1))?))
}

/// In a counted tree whose root is `root` (not 0): the node and index of the row entry with `k` row entries
/// before it, `None` when the tree holds no more. One descent: at each node, past the rows under the children and
/// the row entries before it, into the child whose rows hold it. It fails with `PathOverflow` past `PATH_STACK`
/// levels.
pub fn select<S: Shape, M: NodeRead + ?Sized>(s: S, m: &M, root: u64, mut k: u64) -> Result<Option<(u64, u64)>, Fault> {
    let ks = s.ks();
    // The count flag of entry i is byte `flag` + i·`step` of the node's key parts.
    let (Ok(flag), Ok(step)) = (usize::try_from(ks.wrapping_sub(8)), usize::try_from(ks.max(8))) else {
        return Ok(None);
    };
    let mut n = root;
    'node: for _ in 0..PATH_STACK {
        let (leaf, sh) = shape_of(s, m, n)?;
        let c = count(s, m, n)?;
        let keys = m.bytes(n, ko(s, &sh, 0), c.wrapping_mul(ks))?;
        let under = if leaf { &[][..] } else { m.bytes(n, to(s, 0), c.wrapping_add(1).wrapping_mul(8))? };
        let mut under = under.chunks_exact(8);
        let (mut i, mut at) = (0u64, flag);
        loop {
            if let Some(w) = under.next().and_then(|w| w.first_chunk()) {
                let r = u64::from_le_bytes(*w);
                if k < r {
                    n = child(s, m, n, i)?;
                    continue 'node;
                }
                k = k.wrapping_sub(r);
            }
            let Some(&b) = keys.get(at) else { return Ok(None) };
            if b & COUNT_BYTE == 0 {
                if k == 0 {
                    return Ok(Some((n, i)));
                }
                k = k.wrapping_sub(1);
            }
            (i, at) = (i.wrapping_add(1), at.wrapping_add(step));
        }
    }
    Err(Fault::PathOverflow)
}

/// Clamps a run length to what fits from index i of a node of capacity c.
#[inline(always)]
fn clamp_run(cap: u64, i: u64, k: u64) -> u64 {
    k.min(cap.saturating_sub(i.min(cap)))
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
    let (_, ss) = shape_of(s, m, src)?;
    let (_, ds) = if dst == src { (false, ss) } else { shape_of(s, m, dst)? };
    let k = clamp_run(ds.cap, di, clamp_run(ss.cap, si, k));
    m.copy(src, ko(s, &ss, si), dst, ko(s, &ds, di), k.wrapping_mul(s.ks()))?;
    m.copy(src, vo(s, &ss, si), dst, vo(s, &ds, di), k.wrapping_mul(s.vs()))
}

/// Moves k children from child sj of internal node s to child dj of internal node d, and in a counted tree the
/// rows under them.
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
    let k = clamp_run(slots, dj, clamp_run(slots, sj, k));
    m.copy(src, co(s, sj), dst, co(s, dj), k.wrapping_mul(8))?;
    if M::COUNTED {
        m.copy(src, to(s, sj), dst, to(s, dj), k.wrapping_mul(8))?;
    }
    Ok(())
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
