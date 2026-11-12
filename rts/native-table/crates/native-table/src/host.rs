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

/// How the one B-tree reads nodes: one field at a time. 0 is never a node.
pub trait NodeRead {
    /// The u16 at node offset 0 (count), as stored; the algorithm clamps it to the node's capacity.
    fn count(&self, n: u64) -> Result<u16, Fault>;
    /// The u16 at node offset 2 (flags).
    fn flags(&self, n: u64) -> Result<u16, Fault>;
    /// The little-endian u64 at node offset `off` (a child word).
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault>;
    /// The `len` bytes at node offset `off`: one key part or value part, lent.
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault>;
}

/// How the one B-tree writes nodes. A read-only view of a tree implements `NodeRead` alone.
pub trait NodeMem: NodeRead {
    /// Whether the tree is counted: its internal nodes keep the row entries under each child, at the offsets
    /// `tree::node` gives, and its shape is `IndexShape::counted`'s.
    const COUNTED: bool = false;
    /// The pending entry of this tree's entry form: an index entry that borrows its key and value part for 'k.
    type Entry<'k>: Pending;
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault>;
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault>;
    /// Adds `d` to the word at node offset `off`, wrapping.
    fn add_word(&mut self, n: u64, off: u64, d: u64) -> Result<(), Fault> {
        let w = self.word(n, off)?;
        self.set_word(n, off, w.wrapping_add(d))
    }
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
}

/// The instruction meter.
pub trait Budget {
    /// `ic0.performance_counter(0)`.
    fn used(&self) -> u64;
    /// For the application's `maintain`: maintenance stops before a step when `used() ≥ limit()`. A write's
    /// first call runs exactly one step and does not read the budget.
    fn limit(&self) -> u64;
}

/// Whether a step finished its job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Done,
    More,
}

/// The scratch source a call runs with. `get(need)` returns exactly `need` bytes, growing the source when
/// needed.
pub trait Scratch {
    fn get(&mut self, need: usize) -> Result<&mut [u8], Fault>;
}

/// The table's maintenance state, as every mutating call leaves it. With the state not `Idle`, the
/// next table write runs one maintenance step first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    Idle,
    Later,
}

/// The maintenance state byte: bit 0 pending.
pub const STATE_PENDING: u8 = 1;
/// The freeing byte after it: 1 while the memory of the structures `clear` let go of is going back.
pub const FREEING: u8 = 1;

/// `Wake` from the table root's maintenance state byte and the freeing byte after it, read as one little-endian
/// u16.
#[inline]
pub fn wake_from_state(state: u16) -> Wake {
    if state & (u16::from(STATE_PENDING) | u16::from(FREEING) << 8) != 0 {
        Wake::Later
    } else {
        Wake::Idle
    }
}
