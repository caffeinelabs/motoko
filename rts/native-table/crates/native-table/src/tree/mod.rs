//! The one B-tree: `PBTreeMap`'s algorithm, generic over a shape, a node-memory host, a
//! probe and a pending entry. Every function performs exactly `PBTreeMap`'s byte moves, in the same order,
//! with the same ranges, and calls the host's `alloc` and `hand_back` at exactly its points.
//!
//! Every count and in-node index is clamped to the node's capacity, so a corrupt node gives `Corrupt` or a
//! wrong answer, never an out-of-bounds access.

pub mod cursor;
pub mod insert;
pub mod node;
pub mod remove;
pub mod search;
pub mod shape;

pub use cursor::{Cursor, Dir, Direction, Fwd, Rev};
pub use insert::{insert_at, insert_first};
pub use remove::remove_found;
pub use search::{descend, find, rank, search, Search, Shared};
pub use shape::{IndexShape, NodeShape, Shape};

use crate::consts::PATH_STACK;
use crate::fault::Fault;

/// A search key. `cmp_at` orders the stored key part at offset `koff` of node `n` on the LEFT against this
/// probe on the right, as `PBTreeMap`'s `K::cmp_lookup(stored.as_lookup(), k)`.
pub trait Probe<M: ?Sized> {
    /// Shared-prefix skipping in descents: false by default, so a probe's search is a plain binary search.
    const PREFIX: bool = false;
    fn cmp_at(&self, mem: &M, n: u64, koff: u64) -> Result<core::cmp::Ordering, Fault>;
    /// As `cmp_at`, when the caller guarantees that the stored key and the probe agree on bytes
    /// [0, skip) of the comparator's byte sequence; also returns their shared length, at least `skip`.
    /// Called only when `PREFIX`.
    fn cmp_from(&self, mem: &M, n: u64, koff: u64, skip: u64) -> Result<(core::cmp::Ordering, u64), Fault> {
        let _ = skip;
        Ok((self.cmp_at(mem, n, koff)?, 0))
    }
    /// As `cmp_from`, with the bytes [0, `NODE_SIZE`) of node n already lent as `node` for the whole search
    /// of the node, so a comparison need not ask the memory for them again. The same result as `cmp_from`,
    /// faults included. Called only when `PREFIX`.
    fn cmp_in(&self, mem: &M, node: &[u8], n: u64, koff: u64, skip: u64) -> Result<(core::cmp::Ordering, u64), Fault> {
        let _ = node;
        self.cmp_from(mem, n, koff, skip)
    }
    /// The probe's position among all `c` entries of a node at once, the node's bytes [0, `NODE_SIZE`) lent
    /// as `node` and its key parts `ks` bytes each from byte 8: `Some` with what the comparisons entry by
    /// entry give, or `None` to make them. The descent's shared lengths are left as they are, which they
    /// stay true to. Called only when `PREFIX`, for index shapes, with `c` at most the node's capacity.
    fn search_node(&self, mem: &M, node: &[u8], ks: u64, c: u64) -> Option<Result<u64, u64>> {
        let _ = (mem, node, ks, c);
        None
    }
}

// PATH_STACK = 64.
#[allow(clippy::cast_possible_truncation)]
const FRAMES: usize = PATH_STACK as usize;

/// The (node, index) pairs from the root: for each internal node passed, the child index descended into,
/// or, in a cursor, the entry index next. `new` zero-fills all 64 frames.
#[derive(Clone)]
pub struct Path {
    len: usize,
    frames: [(u64, u64); FRAMES],
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl Path {
    #[inline(always)]
    pub fn new() -> Self {
        Path { len: 0, frames: [(0, 0); FRAMES] }
    }

    /// Appends (n, i). No tree is deeper than the 64 frames (`PATH_STACK`).
    #[inline(always)]
    pub fn push(&mut self, n: u64, i: u64) {
        if let Some(f) = self.frames.get_mut(self.len) {
            *f = (n, i);
            self.len = self.len.wrapping_add(1);
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<(u64, u64)> {
        self.len = self.len.checked_sub(1)?;
        self.frames.get(self.len).copied()
    }

    #[inline(always)]
    pub fn last(&self) -> Option<(u64, u64)> {
        self.frames.get(self.len.checked_sub(1)?).copied()
    }

    #[inline(always)]
    pub fn set_last_index(&mut self, i: u64) {
        if let Some(f) = self.len.checked_sub(1).and_then(|l| self.frames.get_mut(l)) {
            f.1 = i;
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Keeps the first `n` frames, all of them when there are fewer.
    #[inline(always)]
    pub fn truncate(&mut self, n: usize) {
        self.len = self.len.min(n);
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// The frames, root first.
    pub fn frames(&self) -> &[(u64, u64)] {
        self.frames.get(..self.len).unwrap_or(&[])
    }
}
