//! An unchecked transcription of the `PBTreeMap` that Rust EOP (a Rust library for upgrades that keep the heap
//! in place) ships today, statement for statement: the node
//! accessors at the node's address, `Pending` and `Path`, find and descend, insert and split, remove with
//! merge and rotation, the cursor's seek and next, the iterators and the node search, with today's inline
//! attributes.
//!
//! What it leaves out lies outside the tree: the transaction token and `ensure_mutable`, the layout
//! registry (leaf nodes carry layout 1 and internal nodes 2), and the freed list (a dropped key's box is
//! handed back at once). Nodes and boxes come from `mem`'s arenas.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Bound, RangeBounds};

use crate::mem::{self, hand_back, node_alloc, trap};
use crate::part::{PKey, Part, Persistent, INLINE_LIMIT};

/// Bytes of a node.
pub const NODE_SIZE: u64 = 4_080;
/// Frames of a path.
pub const PATH_STACK: usize = 64;
/// Flags bit 0: a leaf.
pub const LEAF: u16 = 1;
/// The layout numbers this crate's nodes carry: leaves 1, internal nodes 2.
pub const LEAF_LAYOUT: u32 = 1;
pub const INTERNAL_LAYOUT: u32 = 2;

/// align_up(a, align) for a power-of-two `align`.
const fn align_up(a: u64, align: u64) -> u64 {
    (a + align - 1) & !(align - 1)
}

/// (voff, coff) of a node of capacity `c`.
const fn node_offsets(c: u64, ks: u64, va: u64, vs: u64) -> (u64, u64) {
    let voff = align_up(8 + c * ks, va);
    (voff, align_up(voff + c * vs, 8))
}

/// (capacity, voff, coff), as the layout engine computes them.
pub const fn node_shape(ks: u64, va: u64, vs: u64, internal: bool) -> (u64, u64, u64) {
    let size = NODE_SIZE;
    let mut c = 0;
    while c < size {
        let (voff, coff) = node_offsets(c + 1, ks, va, vs);
        let end = if internal { coff + 8 * (c + 2) } else { voff + (c + 1) * vs };
        if end > size {
            break;
        }
        c += 1;
    }
    let (voff, coff) = node_offsets(c, ks, va, vs);
    (c, voff, coff)
}

/// Node shapes and node access for keys `K` and values `V`. Nodes are reached by address; every
/// accessor's caller passes the address of a live node of this map type, and entry indices and ranges
/// within the node's capacity c (children: within c + 1, of an internal node).
pub struct Node<K, V>(PhantomData<fn() -> (K, V)>);

impl<K: Persistent, V: Persistent> Node<K, V> {
    /// Bytes of a key part.
    pub const KS: u64 = Part::<K>::SIZE;
    /// Bytes of a value part.
    pub const VS: u64 = Part::<V>::SIZE;
    /// (nL, voff, coff) of leaves.
    pub const LEAF: (u64, u64, u64) = {
        let s = node_shape(Self::KS, Part::<V>::ALIGN, Self::VS, false);
        assert!(s.0 >= 2, "a leaf holds at least two entries");
        s
    };
    /// (nI, voff, coff) of internal nodes.
    pub const INTERNAL: (u64, u64, u64) = {
        let s = node_shape(Self::KS, Part::<V>::ALIGN, Self::VS, true);
        assert!(s.0 >= 2, "an internal node holds at least two entries");
        s
    };

    /// The node's entry count.
    #[inline(always)]
    pub fn count(n: u64) -> u64 {
        // SAFETY: `n` is a live node, whose first 8 bytes are its header.
        unsafe { mem::read_u16(n) as u64 }
    }

    /// Sets the node's entry count, at most its capacity.
    #[inline(always)]
    pub fn set_count(n: u64, c: u64) {
        // SAFETY: `n` is a live node, whose first 8 bytes are its header.
        unsafe { mem::write_u16(n, c as u16) }
    }

    /// Whether the node is a leaf.
    #[inline(always)]
    pub fn is_leaf(n: u64) -> bool {
        // SAFETY: `n` is a live node, whose first 8 bytes are its header.
        unsafe { mem::read_u16(n + 2) & LEAF != 0 }
    }

    /// The node's capacity: nL or nI.
    #[inline(always)]
    pub fn cap(n: u64) -> u64 {
        if Self::is_leaf(n) {
            Self::LEAF.0
        } else {
            Self::INTERNAL.0
        }
    }

    /// The pointer to key part i of the node.
    #[inline(always)]
    pub fn key(n: u64, i: u64) -> *mut u8 {
        mem::at(n + 8 + i * Self::KS)
    }

    /// The pointer to value part i of the node.
    #[inline(always)]
    pub fn val(n: u64, i: u64) -> *mut u8 {
        let voff = if Self::is_leaf(n) { Self::LEAF.1 } else { Self::INTERNAL.1 };
        mem::at(n + voff + i * Self::VS)
    }

    /// Address of child j of the internal node.
    #[inline(always)]
    pub fn child_at(n: u64, j: u64) -> u64 {
        n + Self::INTERNAL.2 + 8 * j
    }

    /// Child j of the internal node.
    #[inline(always)]
    pub fn child(n: u64, j: u64) -> u64 {
        // SAFETY: `n` is a live internal node and j ≤ nI: child j's word.
        unsafe { mem::read_u64(Self::child_at(n, j)) }
    }

    /// Sets child j of the internal node.
    #[inline(always)]
    pub fn set_child(n: u64, j: u64, c: u64) {
        // SAFETY: `n` is a live internal node and j ≤ nI: child j's word.
        unsafe { mem::write_u64(Self::child_at(n, j), c) }
    }

    /// The key stored as entry i of the node.
    ///
    /// # Safety
    ///
    /// Entry i is live, and no mutable reference to the key lives during `'a`.
    #[inline(always)]
    pub unsafe fn key_ref<'a>(n: u64, i: u64) -> &'a K {
        // SAFETY: the caller's contract; parts are aligned within the node.
        unsafe { &*Part::<K>::value_ptr(Self::key(n, i)) }
    }

    /// The value stored as entry i of the node.
    ///
    /// # Safety
    ///
    /// Entry i is live, and no mutable reference to the value lives during `'a`.
    #[inline(always)]
    pub unsafe fn val_ref<'a>(n: u64, i: u64) -> &'a V {
        // SAFETY: the caller's contract; parts are aligned within the node.
        unsafe { &*Part::<V>::value_ptr(Self::val(n, i)) }
    }

    /// The value stored as entry i of the node, mutably.
    ///
    /// # Safety
    ///
    /// Entry i is live, and no other reference to the value lives during `'a`.
    #[inline(always)]
    pub unsafe fn val_mut<'a>(n: u64, i: u64) -> &'a mut V {
        // SAFETY: the caller's contract; parts are aligned within the node.
        unsafe { &mut *Part::<V>::value_ptr(Self::val(n, i)) }
    }

    /// Moves the stored bytes of `count` entries from entry `si` of node `s` to entry `di` of node `d`,
    /// overlapping ranges included; ownership moves with them.
    pub fn move_entries(s: u64, si: u64, d: u64, di: u64, count: u64) {
        if count == 0 {
            return;
        }
        // SAFETY: both nodes are live and both ranges within their capacities, so within their key and
        // value areas; `copy` allows overlap.
        unsafe {
            core::ptr::copy(Self::key(s, si), Self::key(d, di), (count * Self::KS) as usize);
            core::ptr::copy(Self::val(s, si), Self::val(d, di), (count * Self::VS) as usize);
        }
    }

    /// Moves `count` children from child `sj` of internal node `s` to child `dj` of internal node `d`,
    /// overlapping ranges included.
    pub fn move_children(s: u64, sj: u64, d: u64, dj: u64, count: u64) {
        if count == 0 {
            return;
        }
        // SAFETY: both nodes are live internal nodes and both ranges within their nI + 1 children.
        unsafe {
            core::ptr::copy(
                mem::at::<u8>(Self::child_at(s, sj)),
                mem::at::<u8>(Self::child_at(d, dj)),
                (8 * count) as usize,
            )
        }
    }

    /// Writes the pending entry's parts as entry i of node `n`, whose entry i holds none.
    pub fn write_pending(n: u64, i: u64, p: &Pending) {
        // SAFETY: the pending buffers hold the parts' bytes; entry i lies within the live node's capacity.
        unsafe {
            core::ptr::copy_nonoverlapping(p.key.0.as_ptr().cast::<u8>(), Self::key(n, i), Self::KS as usize);
            core::ptr::copy_nonoverlapping(p.val.0.as_ptr().cast::<u8>(), Self::val(n, i), Self::VS as usize);
        }
    }

    /// Moves entry i of node `n` into the pending entry's buffers; entry i then holds no parts.
    pub fn read_pending(n: u64, i: u64, p: &mut Pending) {
        // SAFETY: entry i lies within the live node's capacity; the buffers hold `INLINE_LIMIT` bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(Self::key(n, i), p.key.0.as_mut_ptr().cast::<u8>(), Self::KS as usize);
            core::ptr::copy_nonoverlapping(Self::val(n, i), p.val.0.as_mut_ptr().cast::<u8>(), Self::VS as usize);
        }
    }
}

/// Bytes of one stored part outside a node, aligned to 8.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct PartBuf(pub [MaybeUninit<u8>; INLINE_LIMIT as usize]);

/// An entry on its way into a node: its key and value parts' bytes and, inserted into an internal node,
/// the child following it.
pub struct Pending {
    pub key: PartBuf,
    pub val: PartBuf,
    pub child: u64,
}

impl Pending {
    pub fn new() -> Self {
        Pending {
            key: PartBuf([MaybeUninit::uninit(); INLINE_LIMIT as usize]),
            val: PartBuf([MaybeUninit::uninit(); INLINE_LIMIT as usize]),
            child: 0,
        }
    }
}

impl Default for Pending {
    fn default() -> Self {
        Self::new()
    }
}

/// The (node, index) pairs from the root to a node.
pub struct Path {
    len: usize,
    frames: [(u64, u64); PATH_STACK],
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl Path {
    pub fn new() -> Self {
        Path { len: 0, frames: [(0, 0); PATH_STACK] }
    }

    /// Appends (n, i); traps beyond `PATH_STACK` pairs.
    #[inline(always)]
    fn push(&mut self, n: u64, i: u64) {
        match self.frames.get_mut(self.len) {
            Some(f) => *f = (n, i),
            None => trap("map deeper than its path stack"),
        }
        self.len += 1;
    }

    /// Removes and returns the last pair.
    #[inline(always)]
    fn pop(&mut self) -> Option<(u64, u64)> {
        self.len = self.len.checked_sub(1)?;
        self.frames.get(self.len).copied()
    }

    /// The last pair.
    #[inline(always)]
    fn last(&self) -> Option<(u64, u64)> {
        self.frames.get(self.len.checked_sub(1)?).copied()
    }

    /// Sets the last pair's index.
    #[inline(always)]
    fn set_last_index(&mut self, i: u64) {
        if let Some(f) = self.len.checked_sub(1).and_then(|l| self.frames.get_mut(l)) {
            f.1 = i;
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}

/// Where a search for a key ended.
pub enum Search {
    /// Entry i of node n holds it.
    Found(u64, u64),
    /// It would go at position j of leaf n.
    Vacant(u64, u64),
}

/// `PBTreeMap`'s header `{root, len}` and its operations.
#[repr(C)]
pub struct RefMap<K, V> {
    root: u64,
    len: u64,
    _m: PhantomData<fn() -> (K, V)>,
}

impl<K, V> Default for RefMap<K, V> {
    fn default() -> Self {
        RefMap { root: 0, len: 0, _m: PhantomData }
    }
}

impl<K: PKey, V: Persistent> RefMap<K, V> {
    /// An empty map without nodes.
    pub const fn new() -> Self {
        RefMap { root: 0, len: 0, _m: PhantomData }
    }

    /// The root node's address, 0 for none.
    pub fn root(&self) -> u64 {
        self.root
    }

    /// Number of entries.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn descend(&self, k: K::Lookup<'_>, path: &mut Path) -> Search {
        let mut n = self.root;
        loop {
            match search::<K, V>(n, k) {
                Ok(i) => return Search::Found(n, i),
                Err(j) => {
                    if Node::<K, V>::is_leaf(n) {
                        return Search::Vacant(n, j);
                    }
                    path.push(n, j);
                    n = Node::<K, V>::child(n, j);
                }
            }
        }
    }

    /// The node and index of `k`'s entry, if present.
    pub fn find(&self, k: K::Lookup<'_>) -> Option<(u64, u64)> {
        let mut n = self.root;
        if n == 0 {
            return None;
        }
        loop {
            match search::<K, V>(n, k) {
                Ok(i) => return Some((n, i)),
                Err(j) => {
                    if Node::<K, V>::is_leaf(n) {
                        return None;
                    }
                    n = Node::<K, V>::child(n, j);
                }
            }
        }
    }

    /// Whether the map holds key `k`.
    pub fn contains_key(&self, k: K::Lookup<'_>) -> bool {
        self.find(k).is_some()
    }

    /// The value of key `k`, if present.
    pub fn get(&self, k: K::Lookup<'_>) -> Option<&V> {
        let (n, i) = self.find(k)?;
        // SAFETY: a live entry; the value is borrowed with the map.
        Some(unsafe { Node::<K, V>::val_ref(n, i) })
    }

    /// The value of key `k`, if present, mutably.
    pub fn get_mut(&mut self, k: K::Lookup<'_>) -> Option<&mut V> {
        let (n, i) = self.find(k)?;
        // SAFETY: a live entry; the value is borrowed with the map, mutably.
        Some(unsafe { Node::<K, V>::val_mut(n, i) })
    }

    /// Inserts `v` under `k`; a present key keeps its stored key and gets `v`, and the old value is
    /// returned.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if self.root == 0 {
            self.insert_new(None, k, v);
            return None;
        }
        let mut path = Path::new();
        match self.descend(k.as_lookup(), &mut path) {
            Search::Found(n, i) => {
                // SAFETY: a live entry, which nothing borrows during the call.
                let old = unsafe { Part::<V>::replace(Node::<K, V>::val(n, i), v) };
                drop(k);
                Some(old)
            }
            Search::Vacant(n, j) => {
                self.insert_new(Some((n, j, &mut path)), k, v);
                None
            }
        }
    }

    fn insert_new(&mut self, at: Option<(u64, u64, &mut Path)>, k: K, v: V) -> (u64, u64) {
        let mut p = Pending::new();
        // SAFETY: the pending buffers hold `INLINE_LIMIT` bytes at alignment 8.
        unsafe {
            Part::<K>::put(p.key.0.as_mut_ptr().cast(), k);
            Part::<V>::put(p.val.0.as_mut_ptr().cast(), v);
        }
        self.len += 1;
        match at {
            Some((n, j, path)) => self.insert_at(n, j, &mut p, path),
            None => {
                let n = node_alloc(LEAF_LAYOUT, LEAF);
                Node::<K, V>::write_pending(n, 0, &p);
                Node::<K, V>::set_count(n, 1);
                self.root = n;
                (n, 0)
            }
        }
    }

    fn insert_at(&mut self, mut n: u64, mut i: u64, p: &mut Pending, path: &mut Path) -> (u64, u64) {
        type N<K, V> = Node<K, V>;
        let mut placed: Option<(u64, u64)> = None;
        loop {
            let count = N::<K, V>::count(n);
            let c = N::<K, V>::cap(n);
            let leaf = N::<K, V>::is_leaf(n);
            if count < c {
                N::<K, V>::move_entries(n, i, n, i + 1, count - i);
                if !leaf {
                    N::<K, V>::move_children(n, i + 1, n, i + 2, count - i);
                    N::<K, V>::set_child(n, i + 1, p.child);
                }
                N::<K, V>::write_pending(n, i, p);
                N::<K, V>::set_count(n, count + 1);
                return placed.unwrap_or((n, i));
            }
            let r = if leaf { node_alloc(LEAF_LAYOUT, LEAF) } else { node_alloc(INTERNAL_LAYOUT, 0) };
            let h = c.div_ceil(2);
            match i.cmp(&h) {
                Ordering::Equal => {
                    N::<K, V>::move_entries(n, h, r, 0, c - h);
                    if !leaf {
                        N::<K, V>::set_child(r, 0, p.child);
                        N::<K, V>::move_children(n, h + 1, r, 1, c - h);
                    }
                }
                Ordering::Less => {
                    let mut up = Pending::new();
                    N::<K, V>::move_entries(n, h, r, 0, c - h);
                    if !leaf {
                        N::<K, V>::move_children(n, h, r, 0, c - h + 1);
                    }
                    N::<K, V>::read_pending(n, h - 1, &mut up);
                    N::<K, V>::move_entries(n, i, n, i + 1, h - 1 - i);
                    if !leaf {
                        N::<K, V>::move_children(n, i + 1, n, i + 2, h - 1 - i);
                        N::<K, V>::set_child(n, i + 1, p.child);
                    }
                    N::<K, V>::write_pending(n, i, p);
                    placed.get_or_insert((n, i));
                    core::mem::swap(p, &mut up);
                }
                Ordering::Greater => {
                    let mut up = Pending::new();
                    N::<K, V>::read_pending(n, h, &mut up);
                    N::<K, V>::move_entries(n, h + 1, r, 0, i - h - 1);
                    N::<K, V>::write_pending(r, i - h - 1, p);
                    N::<K, V>::move_entries(n, i, r, i - h, c - i);
                    if !leaf {
                        N::<K, V>::move_children(n, h + 1, r, 0, i - h);
                        N::<K, V>::set_child(r, i - h, p.child);
                        N::<K, V>::move_children(n, i + 1, r, i - h + 1, c - i);
                    }
                    placed.get_or_insert((r, i - h - 1));
                    core::mem::swap(p, &mut up);
                }
            }
            N::<K, V>::set_count(n, h);
            N::<K, V>::set_count(r, c - h);
            p.child = r;
            match path.pop() {
                Some((parent, j)) => {
                    n = parent;
                    i = j;
                }
                None => {
                    let root = node_alloc(INTERNAL_LAYOUT, 0);
                    N::<K, V>::write_pending(root, 0, p);
                    N::<K, V>::set_child(root, 0, n);
                    N::<K, V>::set_child(root, 1, r);
                    N::<K, V>::set_count(root, 1);
                    self.root = root;
                    return placed.unwrap_or((root, 0));
                }
            }
        }
    }

    /// Removes key `k`, returning its value.
    pub fn remove(&mut self, k: K::Lookup<'_>) -> Option<V> {
        if self.root == 0 {
            return None;
        }
        let mut path = Path::new();
        let (n, i) = match self.descend(k, &mut path) {
            Search::Found(n, i) => (n, i),
            Search::Vacant(..) => return None,
        };
        // SAFETY: a live entry, which nothing borrows; its parts are not used again.
        let v = unsafe {
            let v = Part::<V>::take(Node::<K, V>::val(n, i));
            Part::<K>::drop_at(Node::<K, V>::key(n, i));
            v
        };
        let (leaf, gap) = if Node::<K, V>::is_leaf(n) {
            (n, i)
        } else {
            path.push(n, i);
            let mut m = Node::<K, V>::child(n, i);
            while !Node::<K, V>::is_leaf(m) {
                let c = Node::<K, V>::count(m);
                path.push(m, c);
                m = Node::<K, V>::child(m, c);
            }
            let last = Node::<K, V>::count(m) - 1;
            Node::<K, V>::move_entries(m, last, n, i, 1);
            (m, last)
        };
        let count = Node::<K, V>::count(leaf);
        Node::<K, V>::move_entries(leaf, gap + 1, leaf, gap, count - gap - 1);
        Node::<K, V>::set_count(leaf, count - 1);
        self.len -= 1;
        self.rebalance(leaf, &mut path);
        Some(v)
    }

    fn rebalance(&mut self, mut n: u64, path: &mut Path) {
        type N<K, V> = Node<K, V>;
        loop {
            let count = N::<K, V>::count(n);
            let Some((p, j)) = path.pop() else {
                if count == 0 {
                    self.root = if N::<K, V>::is_leaf(n) { 0 } else { N::<K, V>::child(n, 0) };
                    // SAFETY: the emptied root, which nothing references any more.
                    unsafe { hand_back(n) };
                }
                return;
            };
            let c = N::<K, V>::cap(n);
            if count >= c / 2 {
                return;
            }
            let leaf = N::<K, V>::is_leaf(n);
            let pc = N::<K, V>::count(p);
            let (left, right, s) =
                if j > 0 { (N::<K, V>::child(p, j - 1), n, j - 1) } else { (n, N::<K, V>::child(p, 1), 0) };
            let (lc, rc) = (N::<K, V>::count(left), N::<K, V>::count(right));
            if lc + rc < c {
                N::<K, V>::move_entries(p, s, left, lc, 1);
                N::<K, V>::move_entries(right, 0, left, lc + 1, rc);
                if !leaf {
                    N::<K, V>::move_children(right, 0, left, lc + 1, rc + 1);
                }
                N::<K, V>::set_count(left, lc + 1 + rc);
                N::<K, V>::move_entries(p, s + 1, p, s, pc - s - 1);
                N::<K, V>::move_children(p, s + 2, p, s + 1, pc - s - 1);
                N::<K, V>::set_count(p, pc - 1);
                // SAFETY: the right node, emptied into the left one; nothing references it.
                unsafe { hand_back(right) };
                n = p;
            } else if j > 0 {
                N::<K, V>::move_entries(n, 0, n, 1, rc);
                N::<K, V>::move_entries(p, s, n, 0, 1);
                N::<K, V>::move_entries(left, lc - 1, p, s, 1);
                if !leaf {
                    N::<K, V>::move_children(n, 0, n, 1, rc + 1);
                    N::<K, V>::set_child(n, 0, N::<K, V>::child(left, lc));
                }
                N::<K, V>::set_count(left, lc - 1);
                N::<K, V>::set_count(n, rc + 1);
                return;
            } else {
                N::<K, V>::move_entries(p, s, n, lc, 1);
                N::<K, V>::move_entries(right, 0, p, s, 1);
                N::<K, V>::move_entries(right, 1, right, 0, rc - 1);
                if !leaf {
                    N::<K, V>::set_child(n, lc + 1, N::<K, V>::child(right, 0));
                    N::<K, V>::move_children(right, 1, right, 0, rc);
                }
                N::<K, V>::set_count(right, rc - 1);
                N::<K, V>::set_count(n, lc + 1);
                return;
            }
        }
    }

    /// The entry of key `k`, for inserting it when absent or reaching its value when present. Searches
    /// once; an absent key keeps the path, so the insert follows it without a second search.
    pub fn entry<'a, 'q>(&'a mut self, k: K::Lookup<'q>) -> Entry<'a, 'q, K, V>
    where
        K: 'q,
    {
        let mut path = Path::new();
        let at = if self.root == 0 {
            At::Empty
        } else {
            match self.descend(k, &mut path) {
                Search::Found(n, i) => At::Occupied(n, i),
                Search::Vacant(n, j) => At::Vacant(n, j, path),
            }
        };
        Entry { map: self, key: k, at }
    }

    /// Iterates over the entries in key order.
    pub fn iter(&self) -> Iter<'_, '_, K, V> {
        Iter { cursor: Cursor::new::<V>(self.root, Bound::Unbounded, Bound::Unbounded), _m: PhantomData }
    }

    /// Iterates over the entries in key order, the values mutably.
    pub fn iter_mut(&mut self) -> IterMut<'_, '_, K, V> {
        IterMut { cursor: Cursor::new::<V>(self.root, Bound::Unbounded, Bound::Unbounded), _m: PhantomData }
    }

    /// Iterates in key order over the entries whose keys lie within `b`.
    pub fn range<'a, 'q>(&'a self, b: impl RangeBounds<K::Lookup<'q>>) -> Iter<'a, 'q, K, V>
    where
        K: 'q,
    {
        Iter { cursor: Cursor::new::<V>(self.root, b.start_bound().cloned(), b.end_bound().cloned()), _m: PhantomData }
    }

    /// Iterates in key order over the entries whose keys lie within `b`, the values mutably.
    pub fn range_mut<'a, 'q>(&'a mut self, b: impl RangeBounds<K::Lookup<'q>>) -> IterMut<'a, 'q, K, V>
    where
        K: 'q,
    {
        IterMut {
            cursor: Cursor::new::<V>(self.root, b.start_bound().cloned(), b.end_bound().cloned()),
            _m: PhantomData,
        }
    }
}

/// Where an entry's key lies, or would go, with the path from the root to it.
#[allow(clippy::large_enum_variant)]
enum At {
    Occupied(u64, u64),
    Vacant(u64, u64, Path),
    Empty,
}

/// The entry of one key in a map: present or absent.
pub struct Entry<'a, 'q, K: PKey, V>
where
    K: 'q,
{
    map: &'a mut RefMap<K, V>,
    key: K::Lookup<'q>,
    at: At,
}

impl<'a, 'q, K: PKey, V: Persistent> Entry<'a, 'q, K, V>
where
    K: 'q,
{
    /// The value of a present key; otherwise inserts the value `f` returns under the key built from the
    /// lookup form, along the saved path, and returns it.
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        let (n, i) = match self.at {
            At::Occupied(n, i) => (n, i),
            At::Vacant(n, j, mut path) => {
                let v = f();
                let k = K::from_lookup(self.key);
                self.map.insert_new(Some((n, j, &mut path)), k, v)
            }
            At::Empty => {
                let v = f();
                let k = K::from_lookup(self.key);
                self.map.insert_new(None, k, v)
            }
        };
        // SAFETY: a live entry; the value is borrowed with the map, mutably, for `'a`.
        unsafe { Node::<K, V>::val_mut(n, i) }
    }

    /// The value of a present key; otherwise inserts `v`.
    pub fn or_insert(self, v: V) -> &'a mut V {
        self.or_insert_with(|| v)
    }

    /// The lookup form the entry was made with.
    pub fn key(&self) -> K::Lookup<'q> {
        self.key
    }
}

/// A position in a tree and the bound iteration stops at: the path from the root to the next entry.
pub struct Cursor<'q, K: PKey>
where
    K: 'q,
{
    path: Path,
    end: Bound<K::Lookup<'q>>,
}

impl<'q, K: PKey> Cursor<'q, K>
where
    K: 'q,
{
    /// A cursor at the first entry of the tree at `root` not below `start`, stopping beyond `end`.
    pub fn new<V: Persistent>(root: u64, start: Bound<K::Lookup<'q>>, end: Bound<K::Lookup<'q>>) -> Self {
        let mut c = Cursor { path: Path::new(), end };
        if root != 0 {
            c.seek::<V>(root, start);
        }
        c
    }

    fn leftmost<V: Persistent>(&mut self, mut n: u64) {
        loop {
            self.path.push(n, 0);
            if Node::<K, V>::is_leaf(n) {
                return;
            }
            n = Node::<K, V>::child(n, 0);
        }
    }

    fn seek<V: Persistent>(&mut self, root: u64, start: Bound<K::Lookup<'q>>) {
        let (s, inclusive) = match start {
            Bound::Unbounded => return self.leftmost::<V>(root),
            Bound::Included(s) => (s, true),
            Bound::Excluded(s) => (s, false),
        };
        let mut n = root;
        loop {
            let j = match search::<K, V>(n, s) {
                Ok(i) if inclusive => return self.path.push(n, i),
                Ok(i) => {
                    self.path.push(n, i + 1);
                    if !Node::<K, V>::is_leaf(n) {
                        self.leftmost::<V>(Node::<K, V>::child(n, i + 1));
                    }
                    return;
                }
                Err(j) => j,
            };
            self.path.push(n, j);
            if Node::<K, V>::is_leaf(n) {
                return;
            }
            n = Node::<K, V>::child(n, j);
        }
    }

    /// The next entry's node and index, advancing past it.
    ///
    /// # Safety
    ///
    /// The tree the cursor was made on is live and unchanged.
    pub unsafe fn next<V: Persistent>(&mut self) -> Option<(u64, u64)> {
        while let Some((n, i)) = self.path.last() {
            if i >= Node::<K, V>::count(n) {
                self.path.pop();
                continue;
            }
            // SAFETY: a live entry of the unchanged tree.
            let key = unsafe { Node::<K, V>::key_ref(n, i) };
            let beyond = match self.end {
                Bound::Unbounded => false,
                Bound::Included(e) => K::cmp_lookup(key.as_lookup(), e) == Ordering::Greater,
                Bound::Excluded(e) => K::cmp_lookup(key.as_lookup(), e) != Ordering::Less,
            };
            if beyond {
                self.path.clear();
                return None;
            }
            self.path.set_last_index(i + 1);
            if !Node::<K, V>::is_leaf(n) {
                self.leftmost::<V>(Node::<K, V>::child(n, i + 1));
            }
            return Some((n, i));
        }
        None
    }
}

/// The position of `k` among the keys of node `n`: `Ok(i)` when key i equals it, else `Err(j)`.
pub fn search<K: PKey, V: Persistent>(n: u64, k: K::Lookup<'_>) -> Result<u64, u64> {
    let (mut lo, mut hi) = (0, Node::<K, V>::count(n));
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        // SAFETY: entry `mid` is live; the key is borrowed for the comparison only.
        let key = unsafe { Node::<K, V>::key_ref(n, mid) };
        match K::cmp_lookup(key.as_lookup(), k) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(mid),
        }
    }
    Err(lo)
}

/// An iterator over a map's entries in key order: items `(K::Lookup<'a>, &'a V)`.
pub struct Iter<'a, 'q, K: PKey, V>
where
    K: 'q,
{
    cursor: Cursor<'q, K>,
    _m: PhantomData<&'a RefMap<K, V>>,
}

impl<'a, 'q, K: PKey, V: Persistent> Iterator for Iter<'a, 'q, K, V>
where
    K: 'q,
{
    type Item = (K::Lookup<'a>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: the map is borrowed for `'a`, so its tree is live and unchanged.
        let (n, i) = unsafe { self.cursor.next::<V>()? };
        // SAFETY: a live entry of the map borrowed for `'a`.
        unsafe { Some((Node::<K, V>::key_ref::<'a>(n, i).as_lookup(), Node::<K, V>::val_ref::<'a>(n, i))) }
    }
}

/// An iterator over a map's entries in key order with mutable values: items `(K::Lookup<'a>, &'a mut V)`.
pub struct IterMut<'a, 'q, K: PKey, V>
where
    K: 'q,
{
    cursor: Cursor<'q, K>,
    _m: PhantomData<&'a mut RefMap<K, V>>,
}

impl<'a, 'q, K: PKey, V: Persistent> Iterator for IterMut<'a, 'q, K, V>
where
    K: 'q,
{
    type Item = (K::Lookup<'a>, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: the map is borrowed mutably for `'a`, so its tree is live and changes only through the
        // values handed out.
        let (n, i) = unsafe { self.cursor.next::<V>()? };
        // SAFETY: a live entry of the map borrowed for `'a`; each value is handed out once, and keys and
        // values lie apart.
        unsafe { Some((Node::<K, V>::key_ref::<'a>(n, i).as_lookup(), Node::<K, V>::val_mut::<'a>(n, i))) }
    }
}
