//! The crate's tree over typed nodes reached by address, through the host that the map of Rust EOP (a Rust
//! library for upgrades that keep the heap in place) will give it:
//! today's raw field accessors at the node's address (`read_u16`/`write_u16`, `read_u64`/`write_u64`,
//! `ptr::copy` for `copy`, `copy_nonoverlapping` between the pending buffers and the parts), `bytes`
//! refused with `BadRef`, nodes from `node_alloc` with the layout first, `hand_back` at once. No method
//! forms a reference wider than the field it reads or writes, so values handed out stay valid.
//!
//! `CrateMap` is a front end shaped like `PBTreeMap`'s: the same parts handling, the same raw `Node`
//! accessors for the keys and values it hands out, and the crate's `find`, `descend`, `insert_first`,
//! `insert_at`, `remove_found` and `Cursor<Fwd>` in place of the transcribed tree. Its iterator cursor keeps
//! the end bound and checks it itself, between the crate cursor's `peek` and `advance`, on the key reference
//! it hands out, as today's iterators do; its seek stays out of line, as today's does once a map is iterated
//! from more than one place. A `Fault`, which the typed host never returns, traps with its text.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::{Bound, RangeBounds};

use native_table::host::{NodeMem, Pending as PendingEntry};
use native_table::tree::{self, Cursor, Fwd, Path, Probe, Search, Typed, TypedShape};
use native_table::Fault;

use crate::map::{Node, Pending, INTERNAL_LAYOUT, LEAF, LEAF_LAYOUT};
use crate::mem::{self, hand_back, node_alloc, trap};
use crate::part::{PKey, Part, Persistent};

/// The pending entry of typed maps: today's two part buffers and a child word.
pub type PartsPending = Pending;

impl PendingEntry for Pending {
    #[inline(always)]
    fn empty() -> Self {
        Pending::new()
    }
    #[inline(always)]
    fn child(&self) -> u64 {
        self.child
    }
    #[inline(always)]
    fn set_child(&mut self, c: u64) {
        self.child = c;
    }
}

/// Typed nodes of a `PBTreeMap<K, V>`, reached by address. It holds nothing: every node is named by its
/// address.
pub struct RawTypedNodes<K, V>(PhantomData<fn() -> (K, V)>);

impl<K, V> RawTypedNodes<K, V> {
    pub const NEW: Self = RawTypedNodes(PhantomData);
}

impl<K: Persistent, V: Persistent> NodeMem for RawTypedNodes<K, V> {
    type Entry<'k> = PartsPending;
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;

    #[inline(always)]
    fn count(&self, n: u64) -> Result<u16, Fault> {
        // SAFETY: `n` is a live node of this map (the tree's invariants), whose first 8 bytes are its header.
        Ok(unsafe { mem::read_u16(n) })
    }
    #[inline(always)]
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault> {
        // SAFETY: as `count`.
        unsafe { mem::write_u16(n, c) };
        Ok(())
    }
    #[inline(always)]
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        // SAFETY: as `count`.
        Ok(unsafe { mem::read_u16(n + 2) })
    }
    #[inline(always)]
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        // SAFETY: a child word of a live internal node: off = coff + 8j, j ≤ nI.
        Ok(unsafe { mem::read_u64(n + off) })
    }
    #[inline(always)]
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault> {
        // SAFETY: as `word`.
        unsafe { mem::write_u64(n + off, w) };
        Ok(())
    }
    #[inline(always)]
    fn bytes(&self, _n: u64, _off: u64, _len: u64) -> Result<&[u8], Fault> {
        Err(Fault::BadRef)
    }
    #[inline(always)]
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault> {
        // SAFETY: both ranges lie within the parts or child words of live nodes; `copy` allows overlap.
        unsafe { core::ptr::copy(mem::at::<u8>(s + so), mem::at::<u8>(d + doff), len as usize) };
        Ok(())
    }
    #[inline(always)]
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &PartsPending) -> Result<(), Fault> {
        // SAFETY: the pending buffers hold the parts' bytes; entry i of the live node holds no parts.
        unsafe {
            core::ptr::copy_nonoverlapping(p.key.0.as_ptr().cast::<u8>(), mem::at(n + koff), Part::<K>::SIZE as usize);
            core::ptr::copy_nonoverlapping(p.val.0.as_ptr().cast::<u8>(), mem::at(n + voff), Part::<V>::SIZE as usize);
        }
        Ok(())
    }
    #[inline(always)]
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut PartsPending) -> Result<(), Fault> {
        // SAFETY: entry i of the live node holds parts; the buffers hold `INLINE_LIMIT` bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(
                mem::at(n + koff),
                p.key.0.as_mut_ptr().cast::<u8>(),
                Part::<K>::SIZE as usize,
            );
            core::ptr::copy_nonoverlapping(
                mem::at(n + voff),
                p.val.0.as_mut_ptr().cast::<u8>(),
                Part::<V>::SIZE as usize,
            );
        }
        Ok(())
    }
    #[inline(always)]
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault> {
        Ok(if leaf { node_alloc(LEAF_LAYOUT, LEAF) } else { node_alloc(INTERNAL_LAYOUT, 0) })
    }
    #[inline(always)]
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        // SAFETY: the tree hands back only nodes nothing references.
        unsafe { hand_back(n) };
        Ok(())
    }
    #[inline(always)]
    fn release(&mut self, _root: u64) -> Result<(), Fault> {
        // Today: `freed::release(root, NODE, 0)`; the arena is reset whole instead.
        Ok(())
    }
}

/// A typed probe: `K::cmp_lookup` of the stored key, formed by a raw pointer at the node's address
/// (through the box address when boxed), against the lookup form. No slice over node bytes is formed.
pub struct LookupProbe<'q, K: PKey + 'q>(pub K::Lookup<'q>);

impl<'q, K: PKey + 'q, V: Persistent> Probe<RawTypedNodes<K, V>> for LookupProbe<'q, K> {
    #[inline(always)]
    fn cmp_at(&self, _m: &RawTypedNodes<K, V>, n: u64, koff: u64) -> Result<Ordering, Fault> {
        // SAFETY: entry (n, koff) is live; the key is borrowed for the comparison only.
        let key = unsafe { &*Part::<K>::value_ptr(mem::at(n + koff)) };
        Ok(K::cmp_lookup(key.as_lookup(), self.0))
    }
}

/// The typed shape of `PBTreeMap<K, V>`: today's part sizes, unchecked.
pub struct MapShape<K, V>(PhantomData<fn() -> (K, V)>);

impl<K: Persistent, V: Persistent> TypedShape for MapShape<K, V> {
    const KS: u64 = Part::<K>::SIZE;
    const VA: u64 = Part::<V>::ALIGN;
    const VS: u64 = Part::<V>::SIZE;
}

/// The same shape with `CHECKED` true, measured for information.
pub struct MapShapeChecked<K, V>(PhantomData<fn() -> (K, V)>);

impl<K: Persistent, V: Persistent> TypedShape for MapShapeChecked<K, V> {
    const KS: u64 = Part::<K>::SIZE;
    const VA: u64 = Part::<V>::ALIGN;
    const VS: u64 = Part::<V>::SIZE;
    const CHECKED: bool = true;
}

/// The value of a tree call. A path deeper than its stack traps with today's text; any other fault, which
/// the typed host never returns, with the fault's text.
#[inline(always)]
fn ok<T>(r: Result<T, Fault>) -> T {
    match r {
        Ok(x) => x,
        Err(Fault::PathOverflow) => trap("map deeper than its path stack"),
        Err(f) => trap(f.text()),
    }
}

/// `PBTreeMap`'s front end over the crate's tree; `T` picks the unchecked or the checked shape.
#[repr(C)]
pub struct CrateMap<K, V, T = MapShape<K, V>> {
    root: u64,
    len: u64,
    _m: PhantomData<fn() -> (K, V, T)>,
}

impl<K, V, T> Default for CrateMap<K, V, T> {
    fn default() -> Self {
        CrateMap { root: 0, len: 0, _m: PhantomData }
    }
}

impl<K: PKey, V: Persistent, T: TypedShape> CrateMap<K, V, T> {
    const S: Typed<T> = Typed::NEW;

    pub const fn new() -> Self {
        CrateMap { root: 0, len: 0, _m: PhantomData }
    }

    pub fn root(&self) -> u64 {
        self.root
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The node and index of `k`'s entry, if present.
    pub fn find(&self, k: K::Lookup<'_>) -> Option<(u64, u64)> {
        ok(tree::find(Self::S, &RawTypedNodes::<K, V>::NEW, self.root, &LookupProbe::<K>(k)))
    }

    pub fn contains_key(&self, k: K::Lookup<'_>) -> bool {
        self.find(k).is_some()
    }

    pub fn get(&self, k: K::Lookup<'_>) -> Option<&V> {
        let (n, i) = self.find(k)?;
        // SAFETY: a live entry; the value is borrowed with the map.
        Some(unsafe { Node::<K, V>::val_ref(n, i) })
    }

    pub fn get_mut(&mut self, k: K::Lookup<'_>) -> Option<&mut V> {
        let (n, i) = self.find(k)?;
        // SAFETY: a live entry; the value is borrowed with the map, mutably.
        Some(unsafe { Node::<K, V>::val_mut(n, i) })
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if self.root == 0 {
            self.insert_new(None, k, v);
            return None;
        }
        let mut path = Path::new();
        match ok(tree::descend(
            Self::S,
            &RawTypedNodes::<K, V>::NEW,
            self.root,
            &LookupProbe::<K>(k.as_lookup()),
            &mut path,
        )) {
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
        let mut m = RawTypedNodes::<K, V>::NEW;
        match at {
            Some((n, j, path)) => ok(tree::insert_at(Self::S, &mut m, &mut self.root, n, j, &mut p, path)),
            None => {
                let n = ok(tree::insert_first(Self::S, &mut m, &p));
                self.root = n;
                (n, 0)
            }
        }
    }

    pub fn remove(&mut self, k: K::Lookup<'_>) -> Option<V> {
        if self.root == 0 {
            return None;
        }
        let mut path = Path::new();
        let (n, i) =
            match ok(tree::descend(Self::S, &RawTypedNodes::<K, V>::NEW, self.root, &LookupProbe::<K>(k), &mut path)) {
                Search::Found(n, i) => (n, i),
                Search::Vacant(..) => return None,
            };
        // SAFETY: a live entry, which nothing borrows; its parts are not used again.
        let v = unsafe {
            let v = Part::<V>::take(Node::<K, V>::val(n, i));
            Part::<K>::drop_at(Node::<K, V>::key(n, i));
            v
        };
        let mut m = RawTypedNodes::<K, V>::NEW;
        ok(tree::remove_found(Self::S, &mut m, &mut self.root, n, i, &mut path));
        self.len -= 1;
        Some(v)
    }

    /// The entry of key `k`: one search; an absent key keeps the path for the insert.
    pub fn entry<'a, 'q>(&'a mut self, k: K::Lookup<'q>) -> CEntry<'a, 'q, K, V, T>
    where
        K: 'q,
    {
        let mut path = Path::new();
        let at = if self.root == 0 {
            CAt::Empty
        } else {
            match ok(tree::descend(Self::S, &RawTypedNodes::<K, V>::NEW, self.root, &LookupProbe::<K>(k), &mut path)) {
                Search::Found(n, i) => CAt::Occupied(n, i),
                Search::Vacant(n, j) => CAt::Vacant(n, j, path),
            }
        };
        CEntry { map: self, key: k, at }
    }

    pub fn iter(&self) -> CIter<'_, '_, K, V, T> {
        CIter { cursor: TCursor::new(self.root, Bound::Unbounded, Bound::Unbounded), _m: PhantomData }
    }

    pub fn iter_mut(&mut self) -> CIterMut<'_, '_, K, V, T> {
        CIterMut { cursor: TCursor::new(self.root, Bound::Unbounded, Bound::Unbounded), _m: PhantomData }
    }

    pub fn range<'a, 'q>(&'a self, b: impl RangeBounds<K::Lookup<'q>>) -> CIter<'a, 'q, K, V, T>
    where
        K: 'q,
    {
        CIter { cursor: TCursor::new(self.root, b.start_bound().cloned(), b.end_bound().cloned()), _m: PhantomData }
    }

    pub fn range_mut<'a, 'q>(&'a mut self, b: impl RangeBounds<K::Lookup<'q>>) -> CIterMut<'a, 'q, K, V, T>
    where
        K: 'q,
    {
        CIterMut { cursor: TCursor::new(self.root, b.start_bound().cloned(), b.end_bound().cloned()), _m: PhantomData }
    }
}

/// Where an entry's key lies, or would go, with the crate's path to it.
#[allow(clippy::large_enum_variant)]
enum CAt {
    Occupied(u64, u64),
    Vacant(u64, u64, Path),
    Empty,
}

/// The entry of one key, over the crate's tree.
pub struct CEntry<'a, 'q, K: PKey + 'q, V, T> {
    map: &'a mut CrateMap<K, V, T>,
    key: K::Lookup<'q>,
    at: CAt,
}

impl<'a, 'q, K: PKey + 'q, V: Persistent, T: TypedShape> CEntry<'a, 'q, K, V, T> {
    /// The value of a present key; otherwise inserts the value `f` returns along the saved path.
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        let (n, i) = match self.at {
            CAt::Occupied(n, i) => (n, i),
            CAt::Vacant(n, j, mut path) => {
                let v = f();
                let k = K::from_lookup(self.key);
                self.map.insert_new(Some((n, j, &mut path)), k, v)
            }
            CAt::Empty => {
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

/// Today's iterator cursor over the crate's: the crate's `Cursor<Fwd>`, whose state is the path, and the end
/// bound, which the front end keeps and passes to each step.
pub struct TCursor<'q, K: PKey + 'q, V, T> {
    cursor: Cursor<Fwd>,
    end: Bound<K::Lookup<'q>>,
    _m: PhantomData<fn() -> (V, T)>,
}

impl<'q, K: PKey + 'q, V: Persistent, T: TypedShape> TCursor<'q, K, V, T> {
    /// A cursor at the first entry of the tree at `root` not below `start`, stopping beyond `end`.
    fn new(root: u64, start: Bound<K::Lookup<'q>>, end: Bound<K::Lookup<'q>>) -> Self {
        let mut c = TCursor { cursor: Cursor::empty(), end, _m: PhantomData };
        if root != 0 {
            c.seek(root, start);
        }
        c
    }

    /// Positions the cursor at the first entry not below `start`, through the crate's seek; a fault traps.
    /// Kept out of line, as today's seek is once a map is iterated from more than one place: inlined, it
    /// would make the cursor's constructor too large to inline, and the cursor would be built in a
    /// temporary and copied.
    #[inline(never)]
    fn seek(&mut self, root: u64, start: Bound<K::Lookup<'q>>) {
        let start = start.map(LookupProbe::<K>);
        ok(self.cursor.seek_here(CrateMap::<K, V, T>::S, &RawTypedNodes::<K, V>::NEW, root, start.as_ref()));
    }

    /// The next entry's node and index. The crate's cursor finds the entry and moves past it; between the
    /// two, the stored key's reference is formed once and checked against the end bound, as today's cursor
    /// does, so the iterator reuses it for the key it hands out.
    fn next(&mut self) -> Option<(u64, u64)> {
        let (s, m) = (CrateMap::<K, V, T>::S, &RawTypedNodes::<K, V>::NEW);
        let (n, i) = ok(self.cursor.peek(s, m))?;
        // SAFETY: a live entry of the unchanged tree.
        let key = unsafe { Node::<K, V>::key_ref(n, i) };
        let beyond = match self.end {
            Bound::Unbounded => false,
            Bound::Included(e) => K::cmp_lookup(key.as_lookup(), e) == Ordering::Greater,
            Bound::Excluded(e) => K::cmp_lookup(key.as_lookup(), e) != Ordering::Less,
        };
        if beyond {
            self.cursor.clear();
            return None;
        }
        ok(self.cursor.advance(s, m, n, i));
        Some((n, i))
    }
}

/// `Iter` over the crate's cursor.
pub struct CIter<'a, 'q, K: PKey + 'q, V, T> {
    cursor: TCursor<'q, K, V, T>,
    _m: PhantomData<&'a CrateMap<K, V, T>>,
}

impl<'a, 'q, K: PKey + 'q, V: Persistent, T: TypedShape> Iterator for CIter<'a, 'q, K, V, T> {
    type Item = (K::Lookup<'a>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (n, i) = self.cursor.next()?;
        // SAFETY: a live entry of the map borrowed for `'a`.
        unsafe { Some((Node::<K, V>::key_ref::<'a>(n, i).as_lookup(), Node::<K, V>::val_ref::<'a>(n, i))) }
    }
}

/// `IterMut` over the crate's cursor.
pub struct CIterMut<'a, 'q, K: PKey + 'q, V, T> {
    cursor: TCursor<'q, K, V, T>,
    _m: PhantomData<&'a mut CrateMap<K, V, T>>,
}

impl<'a, 'q, K: PKey + 'q, V: Persistent, T: TypedShape> Iterator for CIterMut<'a, 'q, K, V, T> {
    type Item = (K::Lookup<'a>, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (n, i) = self.cursor.next()?;
        // SAFETY: a live entry of the map borrowed mutably for `'a`; each value is handed out once, and
        // keys and values lie apart.
        unsafe { Some((Node::<K, V>::key_ref::<'a>(n, i).as_lookup(), Node::<K, V>::val_mut::<'a>(n, i))) }
    }
}
