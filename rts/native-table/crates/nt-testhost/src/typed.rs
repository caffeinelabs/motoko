//! A typed-map test host for the one B-tree: nodes of 4,080 bytes in a `Vec` arena at 16-aligned
//! references, reused LIFO like an allocator's free list. Like production `node_alloc`, `alloc` writes only
//! the 8-byte header; the test host then fills bytes 8..4,080 with the fill pattern P, so a read of an
//! unwritten byte shows up as a difference between the P = 0xA5 and P = 0x5A runs. It logs every `alloc`
//! result and every `hand_back` argument.
//!
//! `TMap` is a front end shaped like `PBTreeMap`: the root and length live outside the nodes, a present
//! key's value is replaced in place, and the tree algorithm comes from the crate.

use std::cmp::Ordering;
use std::marker::PhantomData;

use native_table::consts::NODE_SIZE;
use native_table::host::{NodeMem, Pending};
use native_table::tree::{
    self, descend, find, insert_at, insert_first, remove_found, Cursor, Direction, Path, Probe, Search, Shape, Typed,
    TypedShape,
};
use native_table::Fault;

/// A fixed-size key or value stored inline in typed nodes.
pub trait Part: Copy + Ord + std::fmt::Debug {
    const SIZE: u64;
    fn write(&self, out: &mut [u8]);
    fn read(b: &[u8]) -> Self;
}

impl Part for u64 {
    const SIZE: u64 = 8;
    fn write(&self, out: &mut [u8]) {
        out.copy_from_slice(&self.to_le_bytes());
    }
    fn read(b: &[u8]) -> Self {
        u64::from_le_bytes(b.try_into().unwrap())
    }
}

impl<const N: usize> Part for [u8; N] {
    const SIZE: u64 = N as u64;
    fn write(&self, out: &mut [u8]) {
        out.copy_from_slice(self);
    }
    fn read(b: &[u8]) -> Self {
        b.try_into().unwrap()
    }
}

/// The typed shape of (K, V): value alignment 8, as `Part<V>` for these types.
pub struct Shp<K, V>(PhantomData<(K, V)>);

impl<K: Part, V: Part> TypedShape for Shp<K, V> {
    const KS: u64 = K::SIZE;
    const VA: u64 = 8;
    const VS: u64 = V::SIZE;
}

/// The typed shape with `CHECKED` true, for information (`cost_vs_ref`).
pub struct ShpChecked<K, V>(PhantomData<(K, V)>);

impl<K: Part, V: Part> TypedShape for ShpChecked<K, V> {
    const KS: u64 = K::SIZE;
    const VA: u64 = 8;
    const VS: u64 = V::SIZE;
    const CHECKED: bool = true;
}

/// A host call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeCall {
    Alloc { node: u64, leaf: bool },
    HandBack(u64),
    Release(u64),
}

/// The pending entry: two part buffers and a child word, as `PartsPending`.
pub struct TPending {
    pub key: [u8; 1024],
    pub val: [u8; 1024],
    pub child: u64,
}

impl Pending for TPending {
    fn empty() -> Self {
        TPending { key: [0; 1024], val: [0; 1024], child: 0 }
    }
    fn child(&self) -> u64 {
        self.child
    }
    fn set_child(&mut self, c: u64) {
        self.child = c;
    }
}

/// Node slot stride in the arena: 4,096, so references stay 16-aligned like dlmalloc blocks.
pub const SLOT: u64 = 4_096;
/// The first node's reference.
pub const BASE: u64 = 0x10_0000;

pub struct TypedNodes<K, V> {
    pub mem: Vec<u8>,
    free: Vec<u64>,
    pub live: std::collections::BTreeSet<u64>,
    pub fill: u8,
    pub log: Vec<NodeCall>,
    ks: u64,
    vs: u64,
    _m: PhantomData<(K, V)>,
}

impl<K: Part, V: Part> TypedNodes<K, V> {
    pub fn new(fill: u8) -> Self {
        TypedNodes {
            mem: Vec::new(),
            free: Vec::new(),
            live: Default::default(),
            fill,
            log: Vec::new(),
            ks: K::SIZE,
            vs: V::SIZE,
            _m: PhantomData,
        }
    }

    fn at(&self, n: u64, off: u64, len: u64) -> Result<std::ops::Range<usize>, Fault> {
        if n < BASE || (n - BASE) % SLOT != 0 || !self.live.contains(&n) || off + len > NODE_SIZE {
            return Err(Fault::BadRef);
        }
        let s = (n - BASE + off) as usize;
        Ok(s..s + len as usize)
    }

    /// The 4,080 bytes of node n.
    pub fn node(&self, n: u64) -> &[u8] {
        let r = self.at(n, 0, NODE_SIZE).unwrap();
        &self.mem[r]
    }
}

impl<K: Part, V: Part> NodeMem for TypedNodes<K, V> {
    type Entry<'k> = TPending;
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn count(&self, n: u64) -> Result<u16, Fault> {
        let r = self.at(n, 0, 2)?;
        Ok(u16::from_le_bytes(self.mem[r].try_into().unwrap()))
    }
    fn set_count(&mut self, n: u64, c: u16) -> Result<(), Fault> {
        let r = self.at(n, 0, 2)?;
        self.mem[r].copy_from_slice(&c.to_le_bytes());
        Ok(())
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        let r = self.at(n, 2, 2)?;
        Ok(u16::from_le_bytes(self.mem[r].try_into().unwrap()))
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        let r = self.at(n, off, 8)?;
        Ok(u64::from_le_bytes(self.mem[r].try_into().unwrap()))
    }
    fn set_word(&mut self, n: u64, off: u64, w: u64) -> Result<(), Fault> {
        let r = self.at(n, off, 8)?;
        self.mem[r].copy_from_slice(&w.to_le_bytes());
        Ok(())
    }
    fn bytes(&self, _n: u64, _off: u64, _len: u64) -> Result<&[u8], Fault> {
        Err(Fault::BadRef)
    }
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault> {
        let a = self.at(s, so, len)?;
        let b = self.at(d, doff, len)?;
        self.mem.copy_within(a, b.start);
        Ok(())
    }
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &TPending) -> Result<(), Fault> {
        let (ks, vs) = (self.ks as usize, self.vs as usize);
        let a = self.at(n, koff, self.ks)?;
        self.mem[a].copy_from_slice(&p.key[..ks]);
        let b = self.at(n, voff, self.vs)?;
        self.mem[b].copy_from_slice(&p.val[..vs]);
        Ok(())
    }
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut TPending) -> Result<(), Fault> {
        let (ks, vs) = (self.ks as usize, self.vs as usize);
        let a = self.at(n, koff, self.ks)?;
        p.key[..ks].copy_from_slice(&self.mem[a]);
        let b = self.at(n, voff, self.vs)?;
        p.val[..vs].copy_from_slice(&self.mem[b]);
        Ok(())
    }
    fn alloc(&mut self, leaf: bool) -> Result<u64, Fault> {
        let n = match self.free.pop() {
            Some(n) => n,
            None => {
                let n = BASE + self.mem.len() as u64;
                self.mem.resize(self.mem.len() + SLOT as usize, self.fill);
                n
            }
        };
        self.live.insert(n);
        let s = (n - BASE) as usize;
        // The header as node_alloc writes it: count 0, flags, layout (the test host writes 1 and 2).
        self.mem[s..s + 2].copy_from_slice(&0u16.to_le_bytes());
        self.mem[s + 2..s + 4].copy_from_slice(&u16::from(leaf).to_le_bytes());
        self.mem[s + 4..s + 8].copy_from_slice(&(if leaf { 1u32 } else { 2u32 }).to_le_bytes());
        self.mem[s + 8..s + NODE_SIZE as usize].fill(self.fill);
        self.log.push(NodeCall::Alloc { node: n, leaf });
        Ok(n)
    }
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        if !self.live.remove(&n) {
            return Err(Fault::BadRef);
        }
        self.free.push(n);
        self.log.push(NodeCall::HandBack(n));
        Ok(())
    }
    fn release(&mut self, root: u64) -> Result<(), Fault> {
        self.log.push(NodeCall::Release(root));
        Ok(())
    }
}

/// A typed probe: compares the stored key part as K.
pub struct TProbe<K>(pub K);

impl<K: Part, V: Part> Probe<TypedNodes<K, V>> for TProbe<K> {
    fn cmp_at(&self, mem: &TypedNodes<K, V>, n: u64, koff: u64) -> Result<Ordering, Fault> {
        let r = mem.at(n, koff, K::SIZE)?;
        Ok(K::read(&mem.mem[r]).cmp(&self.0))
    }
}

/// A typed map front end over the crate's tree.
pub struct TMap<K: Part, V: Part> {
    pub root: u64,
    pub len: u64,
    pub nodes: TypedNodes<K, V>,
}

impl<K: Part, V: Part> TMap<K, V> {
    pub const S: Typed<Shp<K, V>> = Typed::NEW;

    pub fn new(fill: u8) -> Self {
        TMap { root: 0, len: 0, nodes: TypedNodes::new(fill) }
    }

    fn pending(k: &K, v: &V) -> TPending {
        let mut p = TPending::empty();
        k.write(&mut p.key[..K::SIZE as usize]);
        v.write(&mut p.val[..V::SIZE as usize]);
        p
    }

    pub fn get(&self, k: K) -> Option<V> {
        let (n, i) = find(Self::S, &self.nodes, self.root, &TProbe(k)).unwrap()?;
        Some(self.value_at(n, i))
    }

    pub fn value_at(&self, n: u64, i: u64) -> V {
        let sh = Self::S.shape(tree::node::is_leaf(&self.nodes, n).unwrap());
        let off = tree::node::vo(Self::S, &sh, i);
        let r = self.nodes.at(n, off, V::SIZE).unwrap();
        V::read(&self.nodes.mem[r])
    }

    pub fn key_at(&self, n: u64, i: u64) -> K {
        let sh = Self::S.shape(tree::node::is_leaf(&self.nodes, n).unwrap());
        let off = tree::node::ko(Self::S, &sh, i);
        let r = self.nodes.at(n, off, K::SIZE).unwrap();
        K::read(&self.nodes.mem[r])
    }

    /// Inserts; returns the old value of a present key, which keeps its stored key.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let s = Self::S;
        if self.root == 0 {
            let p = Self::pending(&k, &v);
            self.len += 1;
            self.root = insert_first(s, &mut self.nodes, &p).unwrap();
            return None;
        }
        let mut path = Path::new();
        match descend(s, &self.nodes, self.root, &TProbe(k), &mut path).unwrap() {
            Search::Found(n, i) => {
                let old = self.value_at(n, i);
                let sh = s.shape(tree::node::is_leaf(&self.nodes, n).unwrap());
                let off = tree::node::vo(s, &sh, i);
                let r = self.nodes.at(n, off, V::SIZE).unwrap();
                v.write(&mut self.nodes.mem[r]);
                Some(old)
            }
            Search::Vacant(n, j) => {
                let mut p = Self::pending(&k, &v);
                self.len += 1;
                let mut root = self.root;
                let (pn, pi) = insert_at(s, &mut self.nodes, &mut root, n, j, &mut p, &mut path).unwrap();
                self.root = root;
                debug_assert_eq!(self.key_at(pn, pi), k);
                None
            }
        }
    }

    pub fn remove(&mut self, k: K) -> Option<V> {
        let s = Self::S;
        if self.root == 0 {
            return None;
        }
        let mut path = Path::new();
        let (n, i) = match descend(s, &self.nodes, self.root, &TProbe(k), &mut path).unwrap() {
            Search::Found(n, i) => (n, i),
            Search::Vacant(..) => return None,
        };
        let v = self.value_at(n, i);
        let mut root = self.root;
        remove_found(s, &mut self.nodes, &mut root, n, i, &mut path).unwrap();
        self.root = root;
        self.len -= 1;
        Some(v)
    }

    /// Entries in the range, in the cursor's direction.
    pub fn range<D: Direction>(&self, lo: std::ops::Bound<K>, hi: std::ops::Bound<K>) -> Vec<(K, V)> {
        use std::ops::Bound::*;
        let s = Self::S;
        let (start, end) = if D::REV { (hi, lo) } else { (lo, hi) };
        let sp = match start {
            Included(k) => Some((TProbe(k), true)),
            Excluded(k) => Some((TProbe(k), false)),
            Unbounded => None,
        };
        let ep = match end {
            Included(k) => Some((TProbe(k), true)),
            Excluded(k) => Some((TProbe(k), false)),
            Unbounded => None,
        };
        fn b<K>(x: &Option<(TProbe<K>, bool)>) -> std::ops::Bound<&TProbe<K>> {
            match x {
                Some((p, true)) => Included(p),
                Some((p, false)) => Excluded(p),
                None => Unbounded,
            }
        }
        let mut c: Cursor<D> = Cursor::seek(s, &self.nodes, self.root, b(&sp)).unwrap();
        let mut out = Vec::new();
        while let Some((n, i)) = c.next(s, &self.nodes, b(&ep)).unwrap() {
            out.push((self.key_at(n, i), self.value_at(n, i)));
        }
        out
    }

    /// Checks the tree's invariants; returns its height.
    pub fn check(&self) -> u32 {
        let s = Self::S;
        if self.root == 0 {
            assert_eq!(self.len, 0);
            return 0;
        }
        let mut leaf_depth = None;
        let mut total = 0u64;
        let mut prev: Option<K> = None;
        self.walk(s, self.root, 1, true, &mut leaf_depth, &mut total, &mut prev);
        assert_eq!(total, self.len, "len");
        leaf_depth.unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        s: Typed<Shp<K, V>>,
        n: u64,
        depth: u32,
        is_root: bool,
        leaf_depth: &mut Option<u32>,
        total: &mut u64,
        prev: &mut Option<K>,
    ) {
        let leaf = tree::node::is_leaf(&self.nodes, n).unwrap();
        let cnt = tree::node::count(s, &self.nodes, n).unwrap();
        let cap = s.shape(leaf).cap;
        assert!(cnt <= cap);
        if is_root {
            assert!(cnt >= 1);
        } else {
            assert!(cnt >= cap / 2, "node below its minimum: {cnt} of {cap}");
        }
        for i in 0..=cnt {
            if !leaf {
                let c = tree::node::child(s, &self.nodes, n, i).unwrap();
                self.walk(s, c, depth + 1, false, leaf_depth, total, prev);
            }
            if i < cnt {
                let k = self.key_at(n, i);
                if let Some(p) = prev {
                    assert!(*p < k, "keys out of order");
                }
                *prev = Some(k);
                *total += 1;
            }
        }
        if leaf {
            match leaf_depth {
                Some(d) => assert_eq!(*d, depth, "leaves at different depths"),
                None => *leaf_depth = Some(depth),
            }
        }
    }

    /// The nodes reachable from the root, in pre-order.
    pub fn reachable(&self) -> Vec<u64> {
        let s = Self::S;
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(n) = stack.pop() {
            if n == 0 {
                continue;
            }
            out.push(n);
            if !tree::node::is_leaf(&self.nodes, n).unwrap() {
                let cnt = tree::node::count(s, &self.nodes, n).unwrap();
                for j in (0..=cnt).rev() {
                    stack.push(tree::node::child(s, &self.nodes, n, j).unwrap());
                }
            }
        }
        out
    }
}
