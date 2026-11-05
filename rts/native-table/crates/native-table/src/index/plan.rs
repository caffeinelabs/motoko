//! Measuring a run's input and choosing its key-area width W and value-part size V.
//!
//! **Measure.** One pass over the input's items, in (key, rid) order, feeds a greedy chunker per value-part
//! size of `V_CHOICES` and counts, per V, the entries the run would hold: its chunks, plus one count entry
//! for each key with at least `SENT_MIN` chunks. It also counts the distinct keys d, the items, and a
//! histogram of the items' key lengths in 128 buckets of 8 bytes: bucket k holds the keys whose smallest
//! width is 8(k + 1), that is 8k < length ≤ 8(k + 1) (bucket 0 also holds the empty key), and bucket 127 the
//! keys longer than 1,016. In an index with key runs, every maximal stretch of keys of one row item each,
//! stepping by one in key field and rid, is one entry per 2^32 items at every V ≥ 8.
//!
//! **Choose.** The candidate widths are W₀ (the declared width, or 8), the smallest widths that hold 50%,
//! 90%, 99% and 100% of the items' keys (at most 1,016), and 64 when some key is longer than 64 bytes, each
//! raised to W₀. The keys' lengths are taken past the run's shared prefix. Among the pairs (W, V) with
//! W + 8 + V ≤ `PART_MAX`, the choice is the pair whose image is smallest by this estimate:
//! 4,096 · (1 + nodes + count-tree nodes) + the overflow bytes rounded up to 4,096 + filter blocks · 4,096.
//! Nodes come from the bulk plan of the entries at V. The overflow bytes are 24 per item whose key is longer
//! than W plus its bytes beyond W, each key's length taken as its bucket's upper end (1,024 for the last
//! bucket, whose extra bytes are the same at every candidate width). A count tree exists when 2d ≤ items; a
//! filter when the main tree has more than one node. Ties go to the larger W, then to the smaller V.

use crate::consts::{
    BLOCK, CHUNK_RIDS_MAX, COUNT_TREE_DIV, FILTER_BITS, FILTER_LINES_PER_BLOCK, SENT_MIN, V_CHOICES, W_MAX, W_MIN,
};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::index::chunk::{Chunker, KEY_RUN_MAX};
use crate::tree::bulk::plan_nodes;
use crate::tree::IndexShape;

/// The fixed candidate width: keys of up to 64 bytes compare word by word in the key area alone.
pub const CANDIDATE_64: u64 = 64;

/// Buckets of the key-length histogram.
pub const HIST_BUCKETS: usize = 128;

// V_CHOICES has 7 entries.
const NV: usize = V_CHOICES.len();

fn ovf(off: u64) -> Fault {
    corrupt(ImageId::Runtime, Corruption::Count, off)
}

fn add(a: u64, b: u64) -> Result<u64, Fault> {
    a.checked_add(b).ok_or(ovf(a))
}

/// The histogram bucket of a key of `len` bytes.
pub fn bucket(len: u64) -> usize {
    if len <= 8 {
        return 0;
    }
    // (len − 1) / 8 ≥ 1 here; capped at 127.
    usize::try_from((len.wrapping_sub(1) / 8).min(127)).unwrap_or(127)
}

/// The key length a bucket stands for: its upper end, 8(k + 1).
pub fn bucket_len(k: usize) -> u64 {
    u64::try_from(k).unwrap_or(127).min(127).wrapping_add(1).wrapping_mul(8)
}

/// Filter lines for d distinct keys: max(64, ⌈10·d / 512⌉), rounded up to a multiple of 64.
pub fn filter_lines(d: u64) -> Result<u64, Fault> {
    let bitsum = d.checked_mul(FILTER_BITS).ok_or(ovf(d))?;
    let lines = bitsum.div_ceil(512).max(FILTER_LINES_PER_BLOCK);
    lines.div_ceil(FILTER_LINES_PER_BLOCK).checked_mul(FILTER_LINES_PER_BLOCK).ok_or(ovf(d))
}

/// Whether a run of d distinct keys over `items` items carries a count tree: d ≤ items / 2, compared
/// exactly (2d ≤ items).
pub fn has_count_tree(d: u64, items: u64) -> bool {
    match d.checked_mul(COUNT_TREE_DIV) {
        Some(x) => d > 0 && x <= items,
        None => false,
    }
}

/// One candidate value-part size's chunker and counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cand {
    pub chunker: Chunker,
    /// Entries so far: chunks and count entries.
    pub entries: u64,
    /// Chunks of the current key.
    pub key_chunks: u64,
}

/// What the measure pass counts besides the key-length histogram. Each method that takes items returns the
/// histogram bucket its key lands in; the caller raises it (by one, or by a key run's n).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally {
    pub cands: [Cand; NV],
    pub d: u64,
    pub items: u64,
    pub rows: u64,
    pub tombs: u64,
    /// A key is open: `begin_key` was called and `end_key` not yet.
    pub in_key: bool,
}

impl Default for Tally {
    fn default() -> Self {
        Self::new()
    }
}

impl Tally {
    pub fn new() -> Tally {
        let mut cands = [Cand::default(); NV];
        for (c, &v) in cands.iter_mut().zip(V_CHOICES.iter()) {
            c.chunker = Chunker::new(v);
        }
        Tally { cands, d: 0, items: 0, rows: 0, tombs: 0, in_key: false }
    }

    /// Starts a key.
    pub fn begin_key(&mut self) {
        self.in_key = true;
        for c in self.cands.iter_mut() {
            c.chunker.close();
            c.key_chunks = 0;
        }
    }

    /// The current key's next item; returns its key's bucket.
    pub fn item(&mut self, key_len: u64, rid: u64, tomb: bool) -> Result<usize, Fault> {
        if !self.in_key {
            return Err(ovf(rid));
        }
        self.items = add(self.items, 1)?;
        if tomb {
            self.tombs = add(self.tombs, 1)?;
        } else {
            self.rows = add(self.rows, 1)?;
        }
        // `Chunker::offer` at every candidate. Every open chunker's sign and last rid are the key's last
        // item's (each offer sets them), so when all are open with this item's sign, the gap's bits are found
        // once and each candidate only joins or starts a chunk.
        let all_open = self.cands.iter().all(|c| c.chunker.open);
        let (t0, last0) = self.cands.first().map_or((!tomb, 0), |c| (c.chunker.tomb, c.chunker.last));
        if all_open && t0 == tomb && rid >= last0 {
            let gb = crate::index::chunk::bits(rid.wrapping_sub(last0));
            for c in self.cands.iter_mut() {
                let ch = &mut c.chunker;
                let nb = ch.b.max(gb);
                let nm = ch.m.wrapping_add(1);
                // nm ≤ 256 and nb ≤ 64: 3 + ⌈nm·nb / 8⌉ ≤ v without an overflow check.
                if ch.v > 0
                    && nm < CHUNK_RIDS_MAX
                    && nb <= crate::index::chunk::B_MAX
                    && (nm.wrapping_mul(u64::from(nb)).wrapping_add(7) >> 3).wrapping_add(3) <= ch.v
                {
                    ch.m = nm;
                    ch.b = nb;
                } else {
                    ch.m = 0;
                    ch.b = 0;
                    c.entries = c.entries.saturating_add(1);
                    c.key_chunks = c.key_chunks.saturating_add(1);
                }
                ch.last = rid;
            }
            return Ok(bucket(key_len));
        }
        for c in self.cands.iter_mut() {
            if !c.chunker.offer(rid, tomb) {
                c.entries = c.entries.saturating_add(1);
                c.key_chunks = c.key_chunks.saturating_add(1);
            }
        }
        Ok(bucket(key_len))
    }

    /// Ends the current key: a key with at least `SENT_MIN` chunks gets a count entry.
    pub fn end_key(&mut self) -> Result<(), Fault> {
        if !self.in_key {
            return Err(ovf(self.d));
        }
        self.in_key = false;
        self.d = add(self.d, 1)?;
        for c in self.cands.iter_mut() {
            if c.key_chunks >= SENT_MIN {
                c.entries = add(c.entries, 1)?;
            }
            c.chunker.close();
            c.key_chunks = 0;
        }
        Ok(())
    }

    /// A stretch of n ≥ 1 lone keys (`Measure::key_run`); returns its keys' bucket, raised by n.
    pub fn key_run(&mut self, n: u64, key_len: u64) -> Result<usize, Fault> {
        if n == 0 || self.in_key {
            return Err(ovf(n));
        }
        self.items = add(self.items, n)?;
        self.rows = add(self.rows, n)?;
        self.d = add(self.d, n)?;
        let runs = n.div_ceil(KEY_RUN_MAX);
        for c in self.cands.iter_mut() {
            let e = if c.chunker.v >= 8 { runs } else { n };
            c.entries = add(c.entries, e)?;
        }
        Ok(bucket(key_len))
    }

    /// Entries at value-part size v, when v is a candidate.
    pub fn entries(&self, v: u64) -> Option<u64> {
        self.cands.iter().find(|c| c.chunker.v == v).map(|c| c.entries)
    }
}

/// The least count of keys that holds `percent`% of `total`: a count c does exactly when
/// min(100·c, 2^64 − 1) ≥ min(total·percent, 2^64 − 1), so c ≥ ⌈min(total·percent, 2^64 − 1) / 100⌉; one
/// product, and no product per bucket.
#[inline]
fn holding(total: u64, percent: u64) -> u64 {
    total.saturating_mul(percent).div_ceil(100)
}

/// The measure pass's state: its tally and the key-length histogram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measure {
    pub t: Tally,
    pub hist: [u64; HIST_BUCKETS],
}

impl Default for Measure {
    fn default() -> Self {
        Self::new()
    }
}

impl Measure {
    pub fn new() -> Measure {
        Measure { t: Tally::new(), hist: [0; HIST_BUCKETS] }
    }

    fn raise(&mut self, k: usize, n: u64) {
        if let Some(h) = self.hist.get_mut(k) {
            *h = h.saturating_add(n);
        }
    }

    /// Starts a key.
    pub fn begin_key(&mut self) {
        self.t.begin_key();
    }

    /// The current key's next item.
    pub fn item(&mut self, key_len: u64, rid: u64, tomb: bool) -> Result<(), Fault> {
        let k = self.t.item(key_len, rid, tomb)?;
        self.raise(k, 1);
        Ok(())
    }

    /// Ends the current key: a key with at least `SENT_MIN` chunks gets a count entry.
    pub fn end_key(&mut self) -> Result<(), Fault> {
        self.t.end_key()
    }

    /// A maximal stretch of n ≥ 1 keys of `key_len` bytes, one row item each, stepping by one in key field
    /// and rid, in an index with key runs: one entry per 2^32 items at V ≥ 8, one per key at V = 0.
    pub fn key_run(&mut self, n: u64, key_len: u64) -> Result<(), Fault> {
        let k = self.t.key_run(n, key_len)?;
        self.raise(k, n);
        Ok(())
    }

    /// Entries at value-part size v, when v is a candidate.
    pub fn entries(&self, v: u64) -> Option<u64> {
        self.t.entries(v)
    }

    /// The overflow bytes at width w by the estimate above.
    pub fn overflow_bytes(&self, w: u64) -> u64 {
        let mut t = 0u64;
        for (k, &x) in self.hist.iter().enumerate() {
            let len = bucket_len(k);
            if len > w {
                t = t.saturating_add(x.saturating_mul(len.wrapping_sub(w).saturating_add(24)));
            }
        }
        t
    }

    /// The candidate widths, each raised to W₀ (8 when none was declared): W₀, the widths holding 50%, 90%,
    /// 99% and 100% of the keys, and 64 when the widest key needs more (else W₀ again, which `choose` skips).
    pub fn candidate_widths(&self, w0: u64) -> [u64; 6] {
        let base = w0.max(W_MIN);
        // The four widths holding 50%, 90%, 99% and 100% of the keys, in one pass over the histogram.
        let total = self.hist.iter().fold(0u64, |a, &x| a.saturating_add(x));
        let need = [50u64, 90, 99, 100].map(|p| holding(total, p));
        let mut found = [W_MAX; 4];
        let mut done = [false; 4];
        let mut cum = 0u64;
        for (k, &x) in self.hist.iter().enumerate() {
            cum = cum.saturating_add(x);
            for ((f, d), &nd) in found.iter_mut().zip(done.iter_mut()).zip(need.iter()) {
                if !*d && cum >= nd {
                    *f = bucket_len(k).min(W_MAX);
                    *d = true;
                }
            }
            if done.iter().all(|&d| d) {
                break;
            }
        }
        let [f50, f90, f99, f100] = found;
        let w64 = if f100 > CANDIDATE_64 { CANDIDATE_64 } else { base };
        [base, f50, f90, f99, f100, w64].map(|w| w.max(base).min(W_MAX))
    }

    /// The image estimate of the pair (w, v), with its plan; `None` when the pair is not a shape.
    pub fn estimate(&self, w: u64, v: u64) -> Result<Option<Choice>, Fault> {
        let Some(cs) = IndexShape::new(w, 0) else { return Ok(None) };
        let ct_nodes = self.ct_nodes(&cs)?;
        let ob = self.overflow_bytes(w).checked_next_multiple_of(BLOCK).ok_or(ovf(w))?;
        self.estimate_at(w, v, ct_nodes, ob)
    }

    /// The count tree's nodes at width w (its shape `cs`), 0 when the run has none.
    fn ct_nodes(&self, cs: &IndexShape) -> Result<u64, Fault> {
        if has_count_tree(self.t.d, self.t.items) {
            plan_nodes(self.t.d, cs.leaf.cap, cs.internal.cap)
        } else {
            Ok(0)
        }
    }

    /// `estimate` with the parts that depend on the width alone given: the count tree's nodes and the
    /// overflow bytes rounded up to 4,096.
    fn estimate_at(&self, w: u64, v: u64, ct_nodes: u64, ob: u64) -> Result<Option<Choice>, Fault> {
        let (Some(shape), Some(entries)) = (IndexShape::new(w, v), self.entries(v)) else { return Ok(None) };
        let nodes = plan_nodes(entries, shape.leaf.cap, shape.internal.cap)?;
        let filter_blocks =
            if nodes > 1 { filter_lines(self.t.d)?.checked_div(FILTER_LINES_PER_BLOCK).ok_or(ovf(w))? } else { 0 };
        let slots = add(add(1, nodes)?, ct_nodes)?;
        let bytes =
            add(add(slots.checked_mul(BLOCK).ok_or(ovf(w))?, ob)?, filter_blocks.checked_mul(BLOCK).ok_or(ovf(w))?)?;
        Ok(Some(Choice { w, v, entries, nodes, ct_nodes, filter_blocks, bytes }))
    }

    /// The chosen pair (W, V) for a declared width w0 (0 when none). Each candidate width's count tree and
    /// overflow bytes are reckoned once, and a width already tried is not tried again (its pairs tie with
    /// themselves, and a tie keeps the first).
    pub fn choose(&self, w0: u64) -> Result<Choice, Fault> {
        let mut best: Option<Choice> = None;
        let widths = self.candidate_widths(w0);
        for (i, &w) in widths.iter().enumerate() {
            if widths.iter().take(i).any(|&x| x == w) {
                continue;
            }
            let Some(cs) = IndexShape::new(w, 0) else { continue };
            let ct_nodes = self.ct_nodes(&cs)?;
            let ob = self.overflow_bytes(w).checked_next_multiple_of(BLOCK).ok_or(ovf(w))?;
            for &v in V_CHOICES.iter() {
                let Some(c) = self.estimate_at(w, v, ct_nodes, ob)? else { continue };
                let better = match best {
                    None => true,
                    Some(b) => c.bytes < b.bytes || (c.bytes == b.bytes && (c.w > b.w || (c.w == b.w && c.v < b.v))),
                };
                if better {
                    best = Some(c);
                }
            }
        }
        best.ok_or(ovf(w0))
    }
}

/// A pair (W, V) with its estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub w: u64,
    pub v: u64,
    pub entries: u64,
    pub nodes: u64,
    pub ct_nodes: u64,
    pub filter_blocks: u64,
    pub bytes: u64,
}
