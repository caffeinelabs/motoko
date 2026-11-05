//! Reading a run: its image through a checked byte reader, its trees as read-only node memory, and the
//! structure check an uploaded run gets.
//!
//! A run's bytes may be believed (uploaded), so every read is checked and every failed check is
//! `Corrupt { image: the run's id, kind, offset }`, never a panic and never a read outside the image:
//! - node references are multiples of 4,096 inside the slots of the tree being read; each node read checks
//!   flags 3 (leaf) or 2 (internal) and the layout word of the tree's (W, V); counts and in-node indices are
//!   clamped by the tree's algorithm; a descent deeper than 64 levels is `Corrupt(Depth)`;
//! - an overflow record's number is below the header's record count, and its key bytes lie inside the image;
//! - the shared prefix's bytes are read through the header and the key bytes (the trees hold stored keys, the
//!   keys past the prefix, and every read path of the one B-tree runs on them);
//! - chunk headers are checked before any rid is decoded;
//! - the header's geometry is re-checked against the image's own length when a run is opened.
//!
//! `RunImage` is the one thing a reader needs from an image: bytes [o, o + len) lying inside the image and
//! inside one 64 KiB image block. `MemImage` serves a run held in memory; an installed run's image reader
//! serves the same calls.

use core::cell::Cell;
use core::cmp::Ordering;
use core::ops::Bound;

use crate::consts::{BLOCK, FLAG_INDEX, FLAG_LEAF, NODE_SIZE, PAGE};
use crate::fault::{corrupt, Corruption, Fault, ImageId, Refusal, RefusalCode};
use crate::host::{NodeMem, Progress};
use crate::index::chunk::{header as chunk_header, Chunk};
use crate::index::run::{at, record_offset, RunHeader, KIND_RUN, RECORD_BYTES};
use crate::key::probe::{is_count, is_ovf, is_tomb, payload, payload_to_count, IndexPending, KeyProbe, ResolveKey};
use crate::mem::{rd_u16, rd_u32, rd_u64, rd_u8};
use crate::tree::node::{ko, vo};
use crate::tree::{descend, find, Cursor, Direction, IndexShape, Path, Probe, Search, Shape};

/// The bytes of one run image.
pub trait RunImage {
    /// The image's id, named in every fault.
    fn id(&self) -> ImageId;
    /// The image's length in bytes, as the runtime recorded it.
    fn image_len(&self) -> u64;
    /// Bytes [off, off + len) when they lie inside the image and inside one 64 KiB image block; otherwise
    /// `Corrupt(id, Bounds, off)`.
    fn read(&self, off: u64, len: u64) -> Result<&[u8], Fault>;
}

impl<T: RunImage + ?Sized> RunImage for &T {
    fn id(&self) -> ImageId {
        (**self).id()
    }
    fn image_len(&self) -> u64 {
        (**self).image_len()
    }
    fn read(&self, off: u64, len: u64) -> Result<&[u8], Fault> {
        (**self).read(off, len)
    }
}

/// A run image held in memory, read under the same rules as an installed one.
#[derive(Clone, Copy, Debug)]
pub struct MemImage<'a> {
    pub bytes: &'a [u8],
    pub id: ImageId,
}

impl RunImage for MemImage<'_> {
    fn id(&self) -> ImageId {
        self.id
    }
    fn image_len(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(0)
    }
    fn read(&self, off: u64, len: u64) -> Result<&[u8], Fault> {
        let bad = corrupt(self.id, Corruption::Bounds, off);
        let end = off.checked_add(len).ok_or(bad)?;
        if end > self.image_len() || (off % PAGE).checked_add(len).is_none_or(|e| e > PAGE) {
            return Err(bad);
        }
        let s = usize::try_from(off).map_err(|_| bad)?;
        let e = usize::try_from(end).map_err(|_| bad)?;
        self.bytes.get(s..e).ok_or(bad)
    }
}

/// An overflow record: the payload proper, where the key's bytes beyond W lie, the key's full length and
/// the record's own offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub payload: u64,
    pub key_at: u64,
    pub len: u64,
    pub at: u64,
}

/// An opened run: its image, its checked header, the rid base its stored rids are relative to, and the
/// runtime-recorded range [rid_lo, rid_hi] every absolute rid read from it must lie in.
pub struct Run<I: RunImage> {
    pub img: I,
    pub hdr: RunHeader,
    pub rid_base: u64,
    pub rid_lo: u64,
    pub rid_hi: u64,
}

/// Which of a run's two trees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tree {
    Main,
    Count,
}

impl<I: RunImage> Run<I> {
    /// Opens a run: slot 0 is read and its geometry re-checked against the image's length.
    pub fn open(img: I, rid_base: u64, rid_lo: u64, rid_hi: u64) -> Result<Run<I>, Fault> {
        let id = img.id();
        let head = img.read(0, BLOCK)?;
        let bad = |off: u64| corrupt(id, Corruption::Header, off);
        if rd_u32(head, at::MAGIC).ok() != Some(crate::consts::magic::NTIM) {
            return Err(bad(at::MAGIC));
        }
        if rd_u8(head, at::KIND).ok() != Some(KIND_RUN) {
            return Err(bad(at::KIND));
        }
        let hdr = RunHeader::read(head).map_err(|_| bad(0))?;
        hdr.geometry(img.image_len()).map_err(bad)?;
        Ok(Run { img, hdr, rid_base, rid_lo, rid_hi })
    }

    pub fn id(&self) -> ImageId {
        self.img.id()
    }

    /// The main tree.
    pub fn main(&self) -> Result<RunNodes<'_, I>, Fault> {
        let shape = self.hdr.shape().ok_or(corrupt(self.id(), Corruption::Header, at::W))?;
        Ok(RunNodes::new(self, Tree::Main, shape, 1, self.hdr.nodes, self.hdr.root))
    }

    /// The count tree, when the run carries one.
    pub fn count_tree(&self) -> Result<Option<RunNodes<'_, I>>, Fault> {
        if !self.hdr.has_count_tree() {
            return Ok(None);
        }
        let shape = IndexShape::new(self.hdr.w, 0).ok_or(corrupt(self.id(), Corruption::Header, at::W))?;
        let first = self.hdr.nodes.saturating_add(1);
        let last = self.hdr.nodes.saturating_add(self.hdr.ct_nodes);
        Ok(Some(RunNodes::new(self, Tree::Count, shape, first, last, self.hdr.ct_root)))
    }

    /// Overflow record x of a field carrying `F_OVF`.
    pub fn record(&self, field: u64) -> Result<Record, Fault> {
        let id = self.id();
        let x = payload(field);
        if x >= self.hdr.records {
            return Err(corrupt(id, Corruption::Bounds, self.hdr.oa));
        }
        let off = record_offset(self.hdr.oa, x).ok_or(corrupt(id, Corruption::Bounds, self.hdr.oa))?;
        let r = self.img.read(off, RECORD_BYTES)?;
        let bad = corrupt(id, Corruption::Bounds, off);
        let rec = Record {
            payload: rd_u64(r, 0).map_err(|_| bad)?,
            key_at: rd_u64(r, 8).map_err(|_| bad)?,
            len: rd_u64(r, 16).map_err(|_| bad)?,
            at: off,
        };
        let tail = rec.len.checked_sub(self.hdr.w).filter(|&t| t > 0).ok_or(bad)?;
        let end = rec.key_at.checked_add(tail).ok_or(bad)?;
        if end > self.img.image_len() {
            return Err(bad);
        }
        Ok(rec)
    }

    /// Bytes [at, at + out.len()) of an overflowed key, at ≥ W, within one image block; fewer at the key's
    /// end or the block's; returns how many.
    pub fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault> {
        let rec = self.record(field)?;
        let id = self.id();
        let beyond = at.checked_sub(self.hdr.w).ok_or(corrupt(id, Corruption::Bounds, rec.at))?;
        if at >= rec.len {
            return Ok(0);
        }
        let start = rec.key_at.checked_add(beyond).ok_or(corrupt(id, Corruption::Bounds, rec.at))?;
        let room = PAGE.wrapping_sub(start % PAGE);
        let want = u64::try_from(out.len()).unwrap_or(u64::MAX);
        let n = want.min(rec.len.wrapping_sub(at)).min(room);
        let src = self.img.read(start, n)?;
        let nu = usize::try_from(n).map_err(|_| corrupt(id, Corruption::Bounds, start))?;
        crate::mem::copy_bytes(out.get_mut(..nu).unwrap_or(&mut []), src);
        Ok(nu)
    }

    /// Bytes [at, …) of an overflowed key, W ≤ at < its length, where the image holds them: to the end of
    /// their image block or of the key.
    pub fn key_bytes(&self, field: u64, at: u64) -> Result<&[u8], Fault> {
        let rec = self.record(field)?;
        let id = self.id();
        let beyond = at.checked_sub(self.hdr.w).ok_or(corrupt(id, Corruption::Bounds, rec.at))?;
        if at >= rec.len {
            return Err(corrupt(id, Corruption::Bounds, rec.at));
        }
        let start = rec.key_at.checked_add(beyond).ok_or(corrupt(id, Corruption::Bounds, rec.at))?;
        let room = PAGE.wrapping_sub(start % PAGE);
        self.img.read(start, rec.len.wrapping_sub(at).min(room))
    }

    /// Bytes [at, …) of the run's shared prefix, at < p, where the image holds them: the header's to the end
    /// of slot 0, the rest (at the start of the key bytes) to the end of its image block or of the prefix.
    pub fn prefix_bytes(&self, at: u64) -> Result<&[u8], Fault> {
        let p = self.hdr.prefix;
        let bad = corrupt(self.id(), Corruption::Bounds, crate::index::run::at::PREFIX_LEN);
        if at >= p {
            return Err(bad);
        }
        let head = crate::index::run::PREFIX_HEAD;
        if at < head {
            let off = crate::index::run::at::PREFIX.checked_add(at).ok_or(bad)?;
            return self.img.read(off, p.min(head).wrapping_sub(at));
        }
        let off = self.hdr.ok.checked_add(at.wrapping_sub(head)).ok_or(bad)?;
        let room = PAGE.wrapping_sub(off % PAGE);
        self.img.read(off, p.wrapping_sub(at).min(room))
    }

    /// How a key (its full encoding) stands against the run's shared prefix: `Ok(rest)` when it starts with
    /// it, its bytes after the prefix; else `Err(o)`, its order against every key of the run (`Less` for a
    /// key the prefix starts with, or one below it).
    pub fn strip<'k>(&self, key: &'k [u8]) -> Result<Result<&'k [u8], Ordering>, Fault> {
        let p = self.hdr.prefix;
        if p == 0 {
            return Ok(Ok(key));
        }
        let klen = u64::try_from(key.len()).unwrap_or(u64::MAX);
        let mut at = 0u64;
        while at < p {
            let b = self.prefix_bytes(at)?;
            let a = usize::try_from(at).unwrap_or(usize::MAX);
            let mine = key.get(a..a.saturating_add(b.len()).min(key.len())).unwrap_or(&[]);
            let (o, _) = crate::key::words::bytes_cmp_any(mine, b.get(..mine.len()).unwrap_or(&[]));
            if o != Ordering::Equal {
                return Ok(Err(o));
            }
            if mine.len() < b.len() {
                // The key ends inside the prefix: it is a proper prefix of every key of the run.
                return Ok(Err(Ordering::Less));
            }
            at = at.saturating_add(u64::try_from(b.len()).unwrap_or(u64::MAX));
        }
        let pu = usize::try_from(p).unwrap_or(usize::MAX);
        match key.get(pu..) {
            Some(rest) if klen > p => Ok(Ok(rest)),
            _ => Ok(Err(Ordering::Less)),
        }
    }

    /// Writes the shared prefix into `out[..p]`; `Scratch` when `out` is shorter.
    pub fn prefix_into(&self, out: &mut [u8]) -> Result<usize, Fault> {
        let p = self.hdr.prefix;
        let pu = usize::try_from(p).map_err(|_| Fault::Scratch { need: p })?;
        let dst = out.get_mut(..pu).ok_or(Fault::Scratch { need: p })?;
        let mut at = 0usize;
        while at < pu {
            let b = self.prefix_bytes(at as u64)?;
            let n = b.len().min(pu.wrapping_sub(at));
            crate::mem::copy_bytes(dst.get_mut(at..at.wrapping_add(n)).unwrap_or(&mut []), b.get(..n).unwrap_or(&[]));
            at = at.wrapping_add(n);
        }
        Ok(pu)
    }

    /// The first (`last` false) or last fence: the key part and value part of the main tree's first or last
    /// entry, as slot 0 holds it (the stored key, past the shared prefix).
    pub fn fence(&self, last: bool) -> Result<(&[u8], u64, &[u8]), Fault> {
        let off = if last { at::FENCE_LAST } else { at::FENCE_FIRST };
        let ks = self.hdr.w.saturating_add(8);
        let part = self.img.read(off, ks.saturating_add(self.hdr.v))?;
        let bad = corrupt(self.id(), Corruption::Bounds, off);
        let wu = usize::try_from(self.hdr.w).map_err(|_| bad)?;
        let ksu = usize::try_from(ks).map_err(|_| bad)?;
        let field = u64::from_be_bytes(crate::mem::rd_array::<8>(part, self.hdr.w).map_err(|_| bad)?);
        Ok((part.get(..wu).ok_or(bad)?, field, part.get(ksu..).ok_or(bad)?))
    }
}

/// One of a run's trees as read-only node memory: it implements `NodeMem` (every write is `BadRef`) and
/// `ResolveKey` through the run's overflow records, so every read path of the one B-tree runs on it.
pub struct RunNodes<'r, I: RunImage> {
    run: &'r Run<I>,
    tree: Tree,
    shape: IndexShape,
    layout: u32,
    first: u64,
    last: u64,
    root: u64,
    /// The last node a read examined: the offset a depth fault names.
    seen: Cell<u64>,
    /// The last slot read and checked, which a search reads again and again.
    cache: Cell<(u64, &'r [u8])>,
}

/// A run held in memory, read as node memory.
pub type MemRunNodes<'r, 'a> = RunNodes<'r, MemImage<'a>>;

/// An entry of a run tree as stored: its node and index, its key area, its field and its value part.
#[derive(Clone, Copy, Debug)]
pub struct EntryRef<'a> {
    pub node: u64,
    pub index: u64,
    pub area: &'a [u8],
    pub field: u64,
    pub value: &'a [u8],
}

impl<'r, I: RunImage> RunNodes<'r, I> {
    fn new(run: &'r Run<I>, tree: Tree, shape: IndexShape, first: u64, last: u64, root: u64) -> Self {
        RunNodes {
            run,
            tree,
            shape,
            layout: shape.layout(),
            first,
            last,
            root,
            seen: Cell::new(root),
            cache: Cell::new((0, &[])),
        }
    }

    pub fn run(&self) -> &'r Run<I> {
        self.run
    }
    pub fn tree(&self) -> Tree {
        self.tree
    }
    pub fn shape(&self) -> IndexShape {
        self.shape
    }
    pub fn root(&self) -> u64 {
        self.root
    }

    fn bad(&self, kind: Corruption, off: u64) -> Fault {
        corrupt(self.run.id(), kind, off)
    }

    /// The 4,096-byte slot of node n, checked.
    pub fn slot(&self, n: u64) -> Result<&'r [u8], Fault> {
        // The slot read last passed every check below; only a non-zero offset is ever cached.
        let (cn, cs) = self.cache.get();
        if cn == n && n != 0 {
            self.seen.set(n);
            return Ok(cs);
        }
        let lo = self.first.saturating_mul(BLOCK);
        let hi = self.last.saturating_mul(BLOCK);
        if n % BLOCK != 0 || n < lo || n > hi || self.first == 0 {
            return Err(self.bad(Corruption::Bounds, n));
        }
        self.seen.set(n);
        let s = self.run.img.read(n, BLOCK)?;
        let flags = rd_u16(s, 2).map_err(|_| self.bad(Corruption::Bounds, n))?;
        if flags != FLAG_INDEX && flags != FLAG_INDEX | FLAG_LEAF {
            return Err(self.bad(Corruption::Flags, n));
        }
        if rd_u32(s, 4).map_err(|_| self.bad(Corruption::Bounds, n))? != self.layout {
            return Err(self.bad(Corruption::Layout, n));
        }
        self.cache.set((n, s));
        Ok(s)
    }

    /// Turns the tree's `PathOverflow` into `Corrupt(Depth)` at the node last examined.
    pub fn depth<T>(&self, r: Result<T, Fault>) -> Result<T, Fault> {
        r.map_err(|f| if f == Fault::PathOverflow { self.bad(Corruption::Depth, self.seen.get()) } else { f })
    }

    /// The node's trailer: the cumulative count before its subtree and its UNIT flag.
    pub fn trailer(&self, n: u64) -> Result<(i64, bool), Fault> {
        let s = self.slot(n)?;
        let bad = self.bad(Corruption::Bounds, n);
        let cum = i64::from_le_bytes(crate::mem::rd_array::<8>(s, NODE_SIZE).map_err(|_| bad)?);
        let flags = rd_u64(s, NODE_SIZE.wrapping_add(8)).map_err(|_| bad)?;
        Ok((cum, flags & crate::tree::bulk::TRAILER_UNIT != 0))
    }

    /// Whether node n is a leaf, and its count clamped to its capacity.
    pub fn node_info(&self, n: u64) -> Result<(bool, u64), Fault> {
        let s = self.slot(n)?;
        let leaf = rd_u16(s, 2).map_err(|_| self.bad(Corruption::Bounds, n))? & FLAG_LEAF != 0;
        let c = u64::from(rd_u16(s, 0).map_err(|_| self.bad(Corruption::Bounds, n))?);
        Ok((leaf, c.min(self.shape.shape(leaf).cap)))
    }

    /// Entry i of node n (i is clamped to the node's capacity).
    pub fn entry(&self, n: u64, i: u64) -> Result<EntryRef<'r>, Fault> {
        let s = self.slot(n)?;
        let (leaf, _) = self.node_info(n)?;
        let sh = self.shape.shape(leaf);
        let koff = ko(self.shape, &sh, i);
        let voff = vo(self.shape, &sh, i);
        let w = self.shape.w;
        let bad = self.bad(Corruption::Bounds, n);
        let wu = usize::try_from(w).map_err(|_| bad)?;
        let k = usize::try_from(koff).map_err(|_| bad)?;
        let area = s.get(k..k.wrapping_add(wu)).ok_or(bad)?;
        let field = u64::from_be_bytes(crate::mem::rd_array::<8>(s, koff.wrapping_add(w)).map_err(|_| bad)?);
        let v0 = usize::try_from(voff).map_err(|_| bad)?;
        let vu = usize::try_from(self.shape.v).map_err(|_| bad)?;
        let value = s.get(v0..v0.wrapping_add(vu)).ok_or(bad)?;
        if voff.wrapping_add(self.shape.v) > NODE_SIZE {
            return Err(bad);
        }
        Ok(EntryRef { node: n, index: i, area, field, value })
    }

    /// The contribution of an entry: a count entry 0; at V = 0 a row 1 and a tombstone −1; a posting chunk
    /// ±(m + 1); a key run n (a tombstone key run is `Corrupt(Encoding)`).
    pub fn contribution(&self, e: &EntryRef<'_>) -> Result<i64, Fault> {
        if is_count(e.field) {
            return Ok(0);
        }
        let sign: i64 = if is_tomb(e.field) { -1 } else { 1 };
        if self.shape.v == 0 {
            return Ok(sign);
        }
        let at = e.node;
        match chunk_header(e.value, self.run.id(), at)? {
            Chunk::Posting { m, .. } => Ok(sign.wrapping_mul(i64::from(m).wrapping_add(1))),
            Chunk::KeyRun { n_minus_1, .. } => {
                if sign < 0 {
                    return Err(self.bad(Corruption::Encoding, at));
                }
                Ok(i64::from(n_minus_1).wrapping_add(1))
            }
        }
    }

    /// The total a count entry holds (from its record when its key overflows).
    pub fn total(&self, e: &EntryRef<'_>) -> Result<i64, Fault> {
        let p = if is_ovf(e.field) { self.run.record(e.field)?.payload } else { payload(e.field) };
        Ok(payload_to_count(p))
    }

    /// `find` from the root, with depth faults named.
    pub fn find<P: Probe<Self>>(&self, p: &P) -> Result<Option<(u64, u64)>, Fault> {
        self.depth(find(self.shape, self, self.root, p))
    }

    /// `descend` from the root.
    pub fn descend<P: Probe<Self>>(&self, p: &P, path: &mut Path) -> Result<Search, Fault> {
        self.depth(descend(self.shape, self, self.root, p, path))
    }

    /// The probe's position, as `descend` finds it, without recording the path: for a reader that stays in
    /// the leaf. A tree deeper than `PATH_STACK` internal levels is `Corrupt(Depth)`, as for `descend`.
    pub fn locate<P: Probe<Self>>(&self, p: &P) -> Result<Search, Fault> {
        let mut n = self.root;
        let mut sh = crate::tree::Shared::default();
        let mut levels = 0u64;
        loop {
            match crate::tree::search(self.shape, self, n, p, &mut sh)? {
                Ok(i) => return Ok(Search::Found(n, i)),
                Err(j) => {
                    if crate::tree::node::is_leaf(self, n)? {
                        return Ok(Search::Vacant(n, j));
                    }
                    levels = levels.wrapping_add(1);
                    if levels >= crate::consts::PATH_STACK {
                        return Err(self.bad(Corruption::Depth, n));
                    }
                    n = crate::tree::node::child(self.shape, self, n, j)?;
                }
            }
        }
    }

    /// A cursor positioned at `start`.
    pub fn seek<D: Direction>(&self, start: Bound<&KeyProbe<'_>>) -> Result<Cursor<D>, Fault> {
        self.depth(Cursor::seek(self.shape, self, self.root, start))
    }

    /// The cursor's next entry before `end`.
    pub fn next<D: Direction>(
        &self,
        c: &mut Cursor<D>,
        end: Bound<&KeyProbe<'_>>,
    ) -> Result<Option<(u64, u64)>, Fault> {
        self.depth(c.next(self.shape, self, end))
    }
}

impl<I: RunImage> NodeMem for RunNodes<'_, I> {
    type Entry<'k> = IndexPending<'k>;
    type Bytes<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn count(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.slot(n)?, 0).map_err(|_| self.bad(Corruption::Bounds, n))
    }
    fn set_count(&mut self, _n: u64, _c: u16) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn flags(&self, n: u64) -> Result<u16, Fault> {
        rd_u16(self.slot(n)?, 2).map_err(|_| self.bad(Corruption::Bounds, n))
    }
    fn word(&self, n: u64, off: u64) -> Result<u64, Fault> {
        if off.checked_add(8).is_none_or(|e| e > NODE_SIZE) {
            return Err(self.bad(Corruption::Bounds, n.wrapping_add(off)));
        }
        rd_u64(self.slot(n)?, off).map_err(|_| self.bad(Corruption::Bounds, n))
    }
    fn set_word(&mut self, _n: u64, _off: u64, _w: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn bytes(&self, n: u64, off: u64, len: u64) -> Result<&[u8], Fault> {
        let bad = self.bad(Corruption::Bounds, n.wrapping_add(off));
        let end = off.checked_add(len).ok_or(bad)?;
        if end > NODE_SIZE {
            return Err(bad);
        }
        let s = self.slot(n)?;
        let a = usize::try_from(off).map_err(|_| bad)?;
        let b = usize::try_from(end).map_err(|_| bad)?;
        s.get(a..b).ok_or(bad)
    }
    fn copy(&mut self, _s: u64, _so: u64, _d: u64, _doff: u64, _len: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn put_entry(&mut self, _n: u64, _k: u64, _v: u64, _p: &IndexPending<'_>) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn take_entry(&mut self, _n: u64, _k: u64, _v: u64, _p: &mut IndexPending<'_>) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn alloc(&mut self, _leaf: bool) -> Result<u64, Fault> {
        Err(Fault::BadRef)
    }
    fn hand_back(&mut self, _n: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
    fn release(&mut self, _root: u64) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

impl<I: RunImage> ResolveKey for RunNodes<'_, I> {
    fn width(&self) -> u64 {
        self.shape.w
    }
    fn record(&self, field: u64) -> Result<(u64, u64), Fault> {
        let r = self.run.record(field)?;
        Ok((r.payload, r.len))
    }
    fn key_piece(&self, field: u64, at: u64, out: &mut [u8]) -> Result<usize, Fault> {
        self.run.key_piece(field, at, out)
    }
    fn key_bytes(&self, field: u64, at: u64, _len: u64) -> Result<Option<&[u8]>, Fault> {
        self.run.key_bytes(field, at).map(Some)
    }
    fn rid_base(&self) -> u64 {
        self.run.rid_base
    }
}

// ---------------------------------------------------------------- the structure check

/// The structure check of an uploaded run, in steps: every node slot of both trees (flags, layout word,
/// entry count within the capacity, every child a slot of its own tree, every chunk header of the main
/// tree), then every overflow record (its key bytes inside the image). Key order, rids, counts, totals,
/// trailers, fences, the filter and the count tree's totals are never checked: they are the producer's word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructureCheck {
    pub next_slot: u64,
    pub next_record: u64,
}

impl Default for StructureCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureCheck {
    pub const fn new() -> Self {
        StructureCheck { next_slot: 1, next_record: 0 }
    }

    /// Examines node slots, chunk headers and overflow records until `limit` of them are examined (a slot
    /// is finished once started); `Done` when the whole run passed. A failure refuses the image with
    /// `BadStructure` at the offset of what failed.
    pub fn step<I: RunImage>(&mut self, img: &I, h: &RunHeader, image: u64, limit: u64) -> Result<Progress, Refusal> {
        let refuse = |field: u64| Refusal { code: RefusalCode::BadStructure, image, field };
        let main = h.shape().ok_or(refuse(at::W))?;
        let count_shape = IndexShape::new(h.w, 0).ok_or(refuse(at::W))?;
        let slots = h.slots();
        let mut seen = 0u64;
        while self.next_slot <= slots {
            if seen >= limit {
                return Ok(Progress::More);
            }
            let s = self.next_slot;
            let off = s.checked_mul(BLOCK).ok_or(refuse(at::NODES))?;
            let (shape, lo, hi) =
                if s <= h.nodes { (main, 1, h.nodes) } else { (count_shape, h.nodes.saturating_add(1), slots) };
            let b = img.read(off, BLOCK).map_err(|_| refuse(off))?;
            let flags = rd_u16(b, 2).map_err(|_| refuse(off))?;
            if flags != FLAG_INDEX && flags != FLAG_INDEX | FLAG_LEAF {
                return Err(refuse(off.saturating_add(2)));
            }
            if rd_u32(b, 4).map_err(|_| refuse(off))? != shape.layout() {
                return Err(refuse(off.saturating_add(4)));
            }
            let leaf = flags & FLAG_LEAF != 0;
            let sh = shape.shape(leaf);
            let count = u64::from(rd_u16(b, 0).map_err(|_| refuse(off))?);
            if count > sh.cap {
                return Err(refuse(off));
            }
            seen = seen.saturating_add(1);
            if !leaf {
                let mut j = 0u64;
                while j <= count {
                    let at = sh.coff.wrapping_add(j.wrapping_mul(8));
                    let c = rd_u64(b, at).map_err(|_| refuse(off))?;
                    if c % BLOCK != 0 || c < lo.saturating_mul(BLOCK) || c > hi.saturating_mul(BLOCK) {
                        return Err(refuse(off.saturating_add(at)));
                    }
                    j = j.wrapping_add(1);
                }
            }
            if shape.v > 0 {
                let mut i = 0u64;
                while i < count {
                    let kf = ko(shape, &sh, i).wrapping_add(shape.w);
                    let field = u64::from_be_bytes(crate::mem::rd_array::<8>(b, kf).map_err(|_| refuse(off))?);
                    if !is_count(field) {
                        let v0 = vo(shape, &sh, i);
                        let value = crate::mem::sub(b, v0, shape.v).map_err(|_| refuse(off))?;
                        if chunk_header(value, ImageId::Runtime, 0).is_err() {
                            return Err(refuse(off.saturating_add(v0)));
                        }
                    }
                    seen = seen.saturating_add(1);
                    i = i.wrapping_add(1);
                }
            }
            self.next_slot = s.saturating_add(1);
        }
        while self.next_record < h.records {
            if seen >= limit {
                return Ok(Progress::More);
            }
            let off = record_offset(h.oa, self.next_record).ok_or(refuse(at::OA))?;
            let r = img.read(off, RECORD_BYTES).map_err(|_| refuse(off))?;
            let key_at = rd_u64(r, 8).map_err(|_| refuse(off))?;
            let len = rd_u64(r, 16).map_err(|_| refuse(off))?;
            let end = len.checked_sub(h.w).filter(|&t| t > 0).and_then(|t| key_at.checked_add(t));
            if end.is_none_or(|e| e > img.image_len()) {
                return Err(refuse(off));
            }
            seen = seen.saturating_add(1);
            self.next_record = self.next_record.saturating_add(1);
        }
        Ok(Progress::Done)
    }
}

/// An installed run, read through its image's block entries.
impl<H: crate::mem::PageHost + ?Sized> RunImage for crate::image::ImageReader<'_, H> {
    fn id(&self) -> ImageId {
        crate::image::ImageReader::id(self)
    }
    fn image_len(&self) -> u64 {
        self.length()
    }
    fn read(&self, off: u64, len: u64) -> Result<&[u8], Fault> {
        self.bytes(off, len)
    }
}
