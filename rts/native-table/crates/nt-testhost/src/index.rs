//! An index-form test host for the one B-tree: nodes of 4,096-byte slots in a `Vec` (block references), a
//! key log for overflowed keys, and `ITree`, a front end that inserts and removes (key, kind, rid) entries
//! the way an index's tree in table memory does.

use std::cmp::Ordering;

use native_table::consts::NODE_SIZE;
use native_table::host::NodeMem;
use native_table::key::probe::{
    count_field, is_ovf, ovf_field, payload, row_field, tomb_field, IndexPending, KeyProbe, Kind, ResolveKey,
};
use native_table::tree::{
    self, descend, insert_at, insert_first, remove_found, Cursor, Direction, IndexShape, Path, Search,
};
use native_table::Fault;

pub const BASE: u64 = 0x20_0000;
pub const SLOT: u64 = 4_096;

pub struct IndexNodes {
    pub shape: IndexShape,
    pub mem: Vec<u8>,
    free: Vec<u64>,
    pub live: std::collections::BTreeSet<u64>,
    pub fill: u8,
    /// Key log: (payload proper, full key).
    pub log: Vec<(u64, Vec<u8>)>,
    pub calls: Vec<(char, u64)>,
    pub rid_base: u64,
}

impl IndexNodes {
    pub fn new(shape: IndexShape, fill: u8) -> Self {
        IndexNodes {
            shape,
            mem: Vec::new(),
            free: Vec::new(),
            live: Default::default(),
            fill,
            log: Vec::new(),
            calls: Vec::new(),
            rid_base: 0,
        }
    }

    fn at(&self, n: u64, off: u64, len: u64) -> Result<std::ops::Range<usize>, Fault> {
        if n < BASE || (n - BASE) % SLOT != 0 || !self.live.contains(&n) || off + len > NODE_SIZE {
            return Err(Fault::BadRef);
        }
        let s = (n - BASE + off) as usize;
        Ok(s..s + len as usize)
    }
}

impl NodeMem for IndexNodes {
    type Entry<'k> = IndexPending<'k>;
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
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault> {
        let r = self.at(n, off, len)?;
        Ok(&self.mem[r])
    }
    fn copy(&mut self, s: u64, so: u64, d: u64, doff: u64, len: u64) -> Result<(), Fault> {
        let a = self.at(s, so, len)?;
        let b = self.at(d, doff, len)?;
        self.mem.copy_within(a, b.start);
        Ok(())
    }
    fn put_entry(&mut self, n: u64, koff: u64, voff: u64, p: &IndexPending<'_>) -> Result<(), Fault> {
        let ks = self.shape.w + 8;
        let vs = self.shape.v;
        let mut k = vec![0u8; ks as usize];
        let mut v = vec![0u8; vs as usize];
        p.write(self.shape.w, &mut k, &mut v)?;
        let a = self.at(n, koff, ks)?;
        self.mem[a].copy_from_slice(&k);
        let b = self.at(n, voff, vs)?;
        self.mem[b].copy_from_slice(&v);
        Ok(())
    }
    fn take_entry(&mut self, n: u64, koff: u64, voff: u64, p: &mut IndexPending<'_>) -> Result<(), Fault> {
        let a = self.at(n, koff, self.shape.w + 8)?;
        let b = self.at(n, voff, self.shape.v)?;
        let (k, v) = (self.mem[a].to_vec(), self.mem[b].to_vec());
        p.take(&k, &v)
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
        let flags: u16 = if leaf { 3 } else { 2 };
        self.mem[s..s + 2].copy_from_slice(&0u16.to_le_bytes());
        self.mem[s + 2..s + 4].copy_from_slice(&flags.to_le_bytes());
        self.mem[s + 4..s + 8].copy_from_slice(&self.shape.layout().to_le_bytes());
        // Area nodes are not filled at allocation: a reused block keeps its previous bytes.
        self.calls.push(('a', n));
        Ok(n)
    }
    fn hand_back(&mut self, n: u64) -> Result<(), Fault> {
        if !self.live.remove(&n) {
            return Err(Fault::BadRef);
        }
        self.free.push(n);
        self.calls.push(('h', n));
        Ok(())
    }
    fn release(&mut self, root: u64) -> Result<(), Fault> {
        self.calls.push(('r', root));
        Ok(())
    }
}

impl ResolveKey for IndexNodes {
    fn width(&self) -> u64 {
        self.shape.w
    }
    fn record(&self, field: u64) -> Result<(u64, u64), Fault> {
        let (p, k) = self.log.get(payload(field) as usize).ok_or(Fault::BadRef)?;
        Ok((*p, k.len() as u64))
    }
    fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault> {
        let (_, k) = self.log.get(payload(field) as usize).ok_or(Fault::BadRef)?;
        let s = (at as usize).min(k.len());
        let n = out.len().min(k.len() - s);
        out[..n].copy_from_slice(&k[s..s + n]);
        Ok(n)
    }
    fn rid_base(&self) -> u64 {
        self.rid_base
    }
}

/// An entry as the model sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ent {
    pub key: Vec<u8>,
    /// 1 row or tombstone, 2 count.
    pub rank: u8,
    pub rid: u64,
    pub tomb: bool,
    pub value: Vec<u8>,
}

impl Ent {
    /// The key as a node stores it for width w: the full key when longer than w, else zero-padded to w.
    pub fn stored(&self, w: u64) -> Ent {
        let mut e = self.clone();
        if (e.key.len() as u64) <= w {
            e.key.resize(w as usize, 0);
        }
        e
    }
    pub fn order(&self, o: &Ent) -> Ordering {
        self.key.cmp(&o.key).then(self.rank.cmp(&o.rank)).then(self.rid.cmp(&o.rid)).then(self.tomb.cmp(&o.tomb))
    }
    pub fn probe(&self) -> KeyProbe<'_> {
        KeyProbe { key: &self.key, kind: if self.rank == 2 { Kind::Count } else { Kind::Row(self.rid) } }
    }
}

pub struct ITree {
    pub root: u64,
    pub nodes: IndexNodes,
}

impl ITree {
    pub fn new(shape: IndexShape, fill: u8) -> Self {
        ITree { root: 0, nodes: IndexNodes::new(shape, fill) }
    }

    fn field(&mut self, e: &Ent) -> u64 {
        let base = if e.rank == 2 {
            count_field(e.rid as i64)
        } else if e.tomb {
            tomb_field(e.rid)
        } else {
            row_field(e.rid)
        };
        if e.key.len() as u64 > self.nodes.shape.w {
            self.nodes.log.push((native_table::key::probe::payload(base), e.key.clone()));
            ovf_field(base, self.nodes.log.len() as u64 - 1)
        } else {
            base
        }
    }

    /// Inserts an entry not present; returns false when an equal entry exists.
    pub fn insert(&mut self, e: &Ent) -> bool {
        let s = self.nodes.shape;
        let probe = e.probe();
        let field = self.field(e);
        if self.root == 0 {
            let p = IndexPending::Borrowed { key: &e.key, field, value: &e.value, child: 0 };
            self.root = insert_first(s, &mut self.nodes, &p).unwrap();
            return true;
        }
        let mut path = Path::new();
        match descend(s, &self.nodes, self.root, &probe, &mut path).unwrap() {
            Search::Found(..) => false,
            Search::Vacant(n, j) => {
                let mut p = IndexPending::Borrowed { key: &e.key, field, value: &e.value, child: 0 };
                let mut root = self.root;
                insert_at(s, &mut self.nodes, &mut root, n, j, &mut p, &mut path).unwrap();
                self.root = root;
                true
            }
        }
    }

    /// Removes an entry; a tombstone probe finds a tombstone via its own kind.
    pub fn remove(&mut self, e: &Ent) -> bool {
        let s = self.nodes.shape;
        if self.root == 0 {
            return false;
        }
        let mut path = Path::new();
        // Tombstones are Greater than a Row probe at their rid; find them by scanning the next entry.
        let probe = e.probe();
        match descend(s, &self.nodes, self.root, &probe, &mut path).unwrap() {
            Search::Found(n, i) => {
                let mut root = self.root;
                remove_found(s, &mut self.nodes, &mut root, n, i, &mut path).unwrap();
                self.root = root;
                true
            }
            Search::Vacant(..) => false,
        }
    }

    /// The entry at (n, i), decoded.
    pub fn entry(&self, n: u64, i: u64) -> Ent {
        let s = self.nodes.shape;
        let leaf = tree::node::is_leaf(&self.nodes, n).unwrap();
        let sh = native_table::tree::Shape::shape(s, leaf);
        let ko = tree::node::ko(s, &sh, i);
        let vo = tree::node::vo(s, &sh, i);
        let w = s.w as usize;
        let part = self.nodes.bytes(n, ko, s.w + 8).unwrap();
        let field = u64::from_be_bytes(part[w..w + 8].try_into().unwrap());
        // The stored form: the full key when it overflows, else the area zero-padded to W.
        let key = if is_ovf(field) { self.nodes.log[payload(field) as usize].1.clone() } else { part[..w].to_vec() };
        let pl = if is_ovf(field) { self.nodes.log[payload(field) as usize].0 } else { payload(field) };
        let rank = if native_table::key::probe::is_count(field) { 2 } else { 1 };
        let rid = if rank == 2 { native_table::key::probe::payload_to_count(pl) as u64 } else { pl };
        let value = self.nodes.bytes(n, vo, s.v).unwrap().to_vec();
        Ent { key, rank, rid, tomb: native_table::key::probe::is_tomb(field), value }
    }

    /// All entries from `lo` in the cursor's direction, up to `hi`.
    pub fn scan<D: Direction>(
        &self,
        lo: std::ops::Bound<&KeyProbe<'_>>,
        hi: std::ops::Bound<&KeyProbe<'_>>,
    ) -> Vec<Ent> {
        let s = self.nodes.shape;
        let mut c: Cursor<D> = Cursor::seek(s, &self.nodes, self.root, lo).unwrap();
        let mut out = Vec::new();
        while let Some((n, i)) = c.next(s, &self.nodes, hi).unwrap() {
            out.push(self.entry(n, i));
        }
        out
    }
}
