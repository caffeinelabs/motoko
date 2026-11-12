//! Inserts into an index's tree in table memory at the vacant position its descent found: exactly the bytes
//! `tree::insert_at` writes with the descent's path, with the common case done in place.
//!
//! An entry whose key fits a key area of W ≤ 32 bytes, in a tree whose value parts are at most 8 bytes, is
//! inserted in place:
//! - a leaf with room takes the entry inside the one lent block: the entries after its position move up by
//!   one part with the crate's `shift`, and the key's words, the big-endian field, the value and the count
//!   are written as fixed stores;
//! - a full leaf whose parent (the path's last frame) has room splits in place, step for step as `insert_at`
//!   splits it, between the two lent leaves and the parent, the entry that moves up held in a stack buffer of
//!   48 bytes.
//!
//! Everything else takes `insert_at` with the path: a full leaf whose parent is full too, a full leaf that is
//! the root, a key longer than W, W > 32, V > 8.

use core::cmp::Ordering;

use crate::consts::FLAG_LEAF;
use crate::fault::{bounds, Fault};
use crate::host::NodeMem;
use crate::index::area_nodes::AreaNodes;
use crate::key::probe::IndexPending;
use crate::key::words::KeyWords;
use crate::mem::{copy_bytes, shift, sub, sub_mut, PageHost, Region};
use crate::tree::node::rows_in;
use crate::tree::{self, IndexShape, Path};

/// The widest key area inserted in place: four words.
const W_IN_PLACE: u64 = 32;
/// The widest value part inserted in place: one word.
const V_IN_PLACE: u64 = 8;
/// An entry's key part and value part in place, at most 32 + 8 + 8 bytes.
const UP: usize = 48;

/// A tree's node geometry.
#[derive(Clone, Copy)]
struct Geo {
    /// The capacities of a leaf and of an internal node.
    cap_l: u64,
    cap_i: u64,
    /// The value parts' offsets in a leaf and in an internal node, and the children's in an internal node.
    voff_l: u64,
    voff_i: u64,
    coff_i: u64,
    /// The value part's bytes, 0 or 8.
    vs: u64,
}

impl Geo {
    #[inline(always)]
    fn of(s: IndexShape) -> Geo {
        Geo {
            cap_l: s.leaf.cap,
            cap_i: s.internal.cap,
            voff_l: s.leaf.voff,
            voff_i: s.internal.voff,
            coff_i: s.internal.coff,
            vs: s.v,
        }
    }
}

/// The internal node above the leaf: its reference, the child index taken, and its count.
#[derive(Clone, Copy)]
struct Parent {
    n: u64,
    j: u64,
    cnt: u64,
}

/// The words N of the key area when an entry with key `key` and value part `value` goes in place into a tree
/// of shape s; `None` when it takes the general way.
#[inline(always)]
fn words_in_place(s: IndexShape, key: &[u8], value: &[u8]) -> Option<u64> {
    if s.w > W_IN_PLACE || key.len() as u64 > s.w || s.v > V_IN_PLACE || value.len() as u64 != s.v {
        return None;
    }
    Some(s.w / 8)
}

/// A lent node's leaf flag and its count, clamped to its capacity as the tree clamps it.
#[inline(always)]
fn head(node: &[u8], g: &Geo) -> Option<(bool, u64)> {
    let [c0, c1, f0, f1] = *node.first_chunk::<4>()?;
    let leaf = u16::from_le_bytes([f0, f1]) & FLAG_LEAF != 0;
    let cap = if leaf { g.cap_l } else { g.cap_i };
    Some((leaf, u64::from(u16::from_le_bytes([c0, c1])).min(cap)))
}

/// The 8 bytes at `off` of a lent block, writable.
#[inline(always)]
fn word_at(b: &mut [u8], off: u64) -> Result<&mut [u8; 8], Fault> {
    let at = usize::try_from(off).map_err(|_| bounds(off))?;
    b.get_mut(at..).and_then(|t| t.first_chunk_mut::<8>()).ok_or(bounds(off))
}

/// Sets a lent node's count; `c` is at most a capacity, below 4,080.
#[inline(always)]
fn set_count(b: &mut [u8], c: u64) -> Result<(), Fault> {
    let c = u16::try_from(c).map_err(|_| bounds(c))?;
    *b.first_chunk_mut::<2>().ok_or(bounds(0))? = c.to_le_bytes();
    Ok(())
}

/// Moves `len` bytes at `from` of a lent block `by` bytes up, inside the block; nothing for 0.
#[inline(always)]
fn open(b: &mut [u8], from: u64, by: u64, len: u64) -> Result<(), Fault> {
    if len == 0 {
        return Ok(());
    }
    shift(b, from, from.wrapping_add(by), len)
}

/// Copies `len` bytes at `from` of `a` to `to` of `b`; nothing for 0.
#[inline(always)]
fn cross(a: &[u8], from: u64, b: &mut [u8], to: u64, len: u64) -> Result<(), Fault> {
    if len == 0 {
        return Ok(());
    }
    copy_bytes(sub_mut(b, to, len)?, sub(a, from, len)?);
    Ok(())
}

/// Writes the new entry: its key's N words (the key zero-padded to W) and its big-endian field at key offset
/// `ko`, its value part (0 or 8 bytes) at value offset `vo`. The bytes `IndexPending::write` writes.
#[inline(always)]
fn write_entry<const N: usize>(
    b: &mut [u8],
    ko: u64,
    vo: u64,
    kw: &KeyWords,
    field: u64,
    value: &[u8],
) -> Result<(), Fault> {
    let mut q = 0usize;
    let mut at = ko;
    while q < N {
        *word_at(b, at)? = kw.get(q).to_le_bytes();
        q = q.wrapping_add(1);
        at = at.wrapping_add(8);
    }
    *word_at(b, at)? = field.to_be_bytes();
    if let Some(v) = value.first_chunk::<8>() {
        *word_at(b, vo)? = *v;
    }
    Ok(())
}

/// The new entry at position j of leaf n (count `cnt`), in place: in the leaf when it has room (`Ok(true)`),
/// else, when the leaf's parent has room, by splitting the leaf in place (`Ok(true)`); `Ok(false)` with
/// nothing written otherwise. The same bytes as `insert_at` with the path to n.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn put<H: PageHost, const C: bool, const N: usize, const KS: usize>(
    m: &mut AreaNodes<'_, H, C>,
    g: &Geo,
    n: u64,
    j: u64,
    cnt: u64,
    parent: Option<Parent>,
    kw: &KeyWords,
    field: u64,
    value: &[u8],
) -> Result<bool, Fault> {
    let vs = g.vs;
    let ks = KS as u64;
    // Every product below is a byte offset inside one 4 KiB node: positions are at most a capacity.
    let ko = |x: u64| 8u64.wrapping_add(x.wrapping_mul(ks));
    let vo = |x: u64| g.voff_l.wrapping_add(x.wrapping_mul(vs));
    if cnt < g.cap_l {
        // Key parts j..cnt up by one part, then the value parts; cnt < cap, so both stay in the leaf.
        let b = m.host.get_mut(Region::Block(n))?;
        let moved = cnt.wrapping_sub(j);
        open(b, ko(j), ks, moved.wrapping_mul(ks))?;
        open(b, vo(j), vs, moved.wrapping_mul(vs))?;
        write_entry::<N>(b, ko(j), vo(j), kw, field, value)?;
        set_count(b, cnt.wrapping_add(1))?;
        return Ok(true);
    }
    let Some(p) = parent else { return Ok(false) };
    if p.cnt >= g.cap_i {
        return Ok(false);
    }
    // A full leaf of capacity c splits as holding c + 1 entries at h = ⌈c/2⌉, as `insert_at` splits it.
    let r = m.alloc(true)?;
    let c = g.cap_l;
    let h = c.wrapping_sub(c / 2);
    let i = j;
    let mut up = [0u8; UP];
    // In a counted tree, the rows of the two leaves.
    let mut rows = (0u64, 0u64);
    {
        let (ln, rn) = m.host.pair_mut(Region::Block(n), Region::Block(r))?;
        // The entry at x of the left leaf moves up.
        let take_up = |ln: &[u8], x: u64, up: &mut [u8; UP]| -> Result<(), Fault> {
            cross(ln, ko(x), up, 0, ks)?;
            cross(ln, vo(x), up, ks, vs)
        };
        let right = c.wrapping_sub(h);
        match i.cmp(&h) {
            Ordering::Equal => {
                // The new entry itself moves up; the right leaf takes entries [h, c).
                cross(ln, ko(h), rn, ko(0), right.wrapping_mul(ks))?;
                cross(ln, vo(h), rn, vo(0), right.wrapping_mul(vs))?;
                write_entry::<N>(&mut up, 0, ks, kw, field, value)?;
            }
            Ordering::Less => {
                // Entry h − 1 moves up; the new entry goes in at i.
                cross(ln, ko(h), rn, ko(0), right.wrapping_mul(ks))?;
                cross(ln, vo(h), rn, vo(0), right.wrapping_mul(vs))?;
                take_up(ln, h.wrapping_sub(1), &mut up)?;
                let k = h.wrapping_sub(1).wrapping_sub(i);
                open(ln, ko(i), ks, k.wrapping_mul(ks))?;
                open(ln, vo(i), vs, k.wrapping_mul(vs))?;
                write_entry::<N>(ln, ko(i), vo(i), kw, field, value)?;
            }
            Ordering::Greater => {
                // Entry h moves up; the new entry goes into the right leaf at i − h − 1.
                take_up(ln, h, &mut up)?;
                let at = i.wrapping_sub(h).wrapping_sub(1);
                cross(ln, ko(h.wrapping_add(1)), rn, ko(0), at.wrapping_mul(ks))?;
                cross(ln, vo(h.wrapping_add(1)), rn, vo(0), at.wrapping_mul(vs))?;
                write_entry::<N>(rn, ko(at), vo(at), kw, field, value)?;
                let rest = c.wrapping_sub(i);
                cross(ln, ko(i), rn, ko(at.wrapping_add(1)), rest.wrapping_mul(ks))?;
                cross(ln, vo(i), rn, vo(at.wrapping_add(1)), rest.wrapping_mul(vs))?;
            }
        }
        set_count(ln, h)?;
        set_count(rn, right)?;
        if C {
            rows = (
                rows_in(sub(ln, ko(0), h.wrapping_mul(ks))?, ks),
                rows_in(sub(rn, ko(0), right.wrapping_mul(ks))?, ks),
            );
        }
    }
    // The parent: entries pj..pc up by one, their value parts, children pj + 1..=pc up by one; the entry
    // that moved up at pj, the new leaf as child pj + 1.
    let (pj, pc) = (p.j, p.cnt);
    let b = m.host.get_mut(Region::Block(p.n))?;
    let k0 = 8u64.wrapping_add(pj.wrapping_mul(ks));
    let v0 = g.voff_i.wrapping_add(pj.wrapping_mul(vs));
    let c0 = g.coff_i.wrapping_add(pj.wrapping_add(1).wrapping_mul(8));
    let moved = pc.wrapping_sub(pj);
    open(b, k0, ks, moved.wrapping_mul(ks))?;
    open(b, v0, vs, moved.wrapping_mul(vs))?;
    open(b, c0, 8, moved.wrapping_mul(8))?;
    if C {
        // The rows under children pj + 1..=pc up by one, after the children; then the two leaves' rows.
        let t0 = g.coff_i.wrapping_add(g.cap_i.wrapping_add(1).wrapping_add(pj).wrapping_mul(8));
        open(b, t0.wrapping_add(8), 8, moved.wrapping_mul(8))?;
        *word_at(b, t0)? = rows.0.to_le_bytes();
        *word_at(b, t0.wrapping_add(8))? = rows.1.to_le_bytes();
    }
    cross(&up, 0, b, k0, ks)?;
    cross(&up, ks, b, v0, vs)?;
    *word_at(b, c0)? = r.to_le_bytes();
    set_count(b, pc.wrapping_add(1))?;
    Ok(true)
}

/// Inserts an entry into the area tree `m` with root `root` (the new root is written back) at the vacant
/// position j of leaf n that a descent with `path` found: its key `key`, kept whole (a key longer than W has
/// its record in the key log already, named by the overflowed `field`), its field and its value part of V
/// bytes. In place when the key fits W ≤ 32 and V ≤ 8 and the leaf, or the path's last parent, has room; else
/// `insert_at` with the path. The same bytes as `insert_at`.
#[allow(clippy::too_many_arguments)]
pub fn insert_vacant<H: PageHost, const C: bool>(
    m: &mut AreaNodes<'_, H, C>,
    root: &mut u64,
    n: u64,
    j: u64,
    key: &[u8],
    field: u64,
    value: &[u8],
    path: &mut Path,
) -> Result<(), Fault> {
    if let Some(nw) = words_in_place(m.shape, key, value) {
        let done = match nw {
            1 => vacant_put::<H, C, 1, 16>(m, n, j, key, field, value, path)?,
            2 => vacant_put::<H, C, 2, 24>(m, n, j, key, field, value, path)?,
            3 => vacant_put::<H, C, 3, 32>(m, n, j, key, field, value, path)?,
            _ => vacant_put::<H, C, 4, 40>(m, n, j, key, field, value, path)?,
        };
        if done {
            return Ok(());
        }
    }
    let s = m.shape;
    let mut p = IndexPending::Borrowed { key, field, value, child: 0 };
    tree::insert_at(s, m, root, n, j, &mut p, path)?;
    Ok(())
}

/// `insert_vacant` in place at key width W = 8N: `Ok(false)` with nothing written when `insert_at` must do it.
#[inline(always)]
fn vacant_put<H: PageHost, const C: bool, const N: usize, const KS: usize>(
    m: &mut AreaNodes<'_, H, C>,
    n: u64,
    j: u64,
    key: &[u8],
    field: u64,
    value: &[u8],
    path: &Path,
) -> Result<bool, Fault> {
    let g = Geo::of(m.shape);
    let (leaf, cnt) = head(m.host.get(Region::Block(n))?, &g).ok_or(bounds(n))?;
    if !leaf {
        return Ok(false);
    }
    // `insert_at` clamps the position to the count.
    let j = j.min(cnt);
    let mut parent = None;
    if cnt >= g.cap_l {
        // The parent is read only for a split, and only an internal node takes one in place.
        let Some((pn, pj)) = path.last() else { return Ok(false) };
        let (pleaf, pc) = head(m.host.get(Region::Block(pn))?, &g).ok_or(bounds(pn))?;
        if pleaf {
            return Ok(false);
        }
        parent = Some(Parent { n: pn, j: pj.min(pc), cnt: pc });
    }
    put::<H, C, N, KS>(m, &g, n, j, cnt, parent, &KeyWords::first(key, N), field, value)
}
