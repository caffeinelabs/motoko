//! Host traits beyond memory: node field access for the one B-tree, the pending entry of a
//! tree's entry form, the instruction budget, scratch sources and the maintenance wake state.

use crate::fault::Fault;

/// An entry on its way into a node, and the `up` entry of a split. Its bytes reach a node only through
/// `NodeMem::put_entry` and `NodeMem::take_entry`.
pub trait Pending: Sized {
    /// Called only for `up` in a split.
    fn empty() -> Self;
    fn child(&self) -> u64;
    fn set_child(&mut self, c: u64);
}

/// How the one B-tree reads nodes: one field at a time, so a reference a typed front end handed out stays
/// valid across later cursor calls. 0 is never a node.
pub trait NodeRead {
    /// The view `bytes` returns: a lent slice of main memory, or a typed host's own view.
    type Bytes<'a>: core::ops::Deref<Target = [u8]>
    where
        Self: 'a;
    /// The u16 at node offset 0 (count), as stored; the algorithm clamps it when the shape is checked.
    fn count(&self, n: u64) -> Result<u16, Fault>;
    /// The u16 at node offset 2 (flags).
    fn flags(&self, n: u64) -> Result<u16, Fault>;
    /// The little-endian u64 at node offset `off` (a child word).
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault>;
    /// The `len` bytes at node offset `off`: one index-form key part or value part. Index forms only; a
    /// typed host returns `Err(BadRef)`.
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<Self::Bytes<'_>, Fault>;
}

/// How the one B-tree writes nodes. A read-only view of a tree implements `NodeRead` alone.
pub trait NodeMem: NodeRead {
    /// The pending entry of this tree's entry form: the typed front end's two part buffers, or an index
    /// entry that borrows its key and value part for 'k.
    type Entry<'k>: Pending;
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault>;
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault>;
    /// Moves `len` bytes from offset `so` of node `s` to offset `doff` of node `d`; s = d is allowed,
    /// overlapping ranges included (memmove).
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault>;
    /// Writes the pending entry's key part at `koff` and its value part at `voff` of node n.
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &Self::Entry<'_>) -> Result<(), Fault>;
    /// Moves the key part at `koff` and the value part at `voff` of node n into `p`.
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut Self::Entry<'_>) -> Result<(), Fault>;
    /// A fresh node whose header is {count 0, flags of `leaf` and the tree's form, layout}; bytes 8..4,080
    /// are unspecified.
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault>;
    /// Returns node n at once; nothing references it.
    fn hand_back(&mut self, n: u64) -> Result<(), Fault>;
    /// Frees the whole tree under root n later, in bounded steps; O(1) now. Called by front ends only.
    fn release(&mut self, root: u64) -> Result<(), Fault>;
}

/// The instruction meter.
pub trait Budget {
    /// `ic0.performance_counter(0)` in both runtimes; 0 in the builder.
    fn used(&self) -> u64;
    /// For an upload message's work and the application's `maintain`: the step stops before a unit when
    /// `used() ≥ limit()`. A write's step runs exactly `WRITE_STEP_UNITS` units and reads the budget only as
    /// the meter that ends a unit early.
    fn limit(&self) -> u64;
    /// With the `meter-notes` feature only: a maintenance decision on a metered host, at `line` of source file
    /// `file`, took `ahead` instructions as the most that the work it lets begin costs before the next
    /// decision, the unit's end aside (0: the unit's share of work ends there), and, when it checks the unit's
    /// pages, `pages` as the most 4 KiB pages that work changes for the first time in the unit (`u64::MAX`: it
    /// does not check them).
    #[cfg(feature = "meter-notes")]
    fn note(&self, _ahead: u64, _pages: u64, _file: &'static str, _line: u32) {}
}

/// Whether a step finished its job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Done,
    More,
}

/// The scratch source a step runs with. `get(need)` returns exactly `need` bytes, growing the source when
/// needed. A unit asks only for its current working state.
pub trait Scratch {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault>;
}

/// The table's maintenance state, as every mutating call leaves it. With the state not `Idle`, the
/// next table write runs one full unit first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    Idle,
    Later,
    Now,
}

/// The maintenance state byte: bit 0 pending, bit 1 urgent.
pub const STATE_PENDING: u8 = 1;
/// See `STATE_PENDING`.
pub const STATE_URGENT: u8 = 2;

/// `Wake` from the table root's maintenance state byte.
#[inline]
pub fn wake_from_state(state: u8) -> Wake {
    if state & STATE_URGENT != 0 {
        Wake::Now
    } else if state & STATE_PENDING != 0 {
        Wake::Later
    } else {
        Wake::Idle
    }
}
