//! Runs for tests: an input of items in a `Vec`, a build into a `Vec` image prefilled with a fill pattern,
//! a sink that counts how often each byte is written, and an independent model of the entries a run holds.

use native_table::consts::{CHUNK_RIDS_MAX, SENT_MIN};
use native_table::index::{build_run, plan_run, Item, MemImage, Run, RunInput, RunParams, RunPlan};
use native_table::tree::{MemRunSink, RunSink};
use native_table::{Fault, ImageId};

/// Items (key, rid, tombstone) in (key, rid) order.
#[derive(Clone, Debug, Default)]
pub struct VecItems(pub Vec<(Vec<u8>, u64, bool)>);

impl RunInput for VecItems {
    fn items(&self) -> u64 {
        self.0.len() as u64
    }
    fn item(&self, i: u64) -> Result<Item<'_>, Fault> {
        let (k, r, t) = self.0.get(i as usize).ok_or(Fault::BadRef)?;
        Ok(Item { key: k, rid: *r, tomb: *t })
    }
}

impl VecItems {
    /// Sorts the items into run order: by key, then rid, then sign.
    pub fn sorted(mut items: Vec<(Vec<u8>, u64, bool)>) -> VecItems {
        items.sort();
        VecItems(items)
    }
}

/// A sink that writes like `MemRunSink` and counts the writes of each byte.
pub struct CoverSink {
    pub hits: Vec<u8>,
}

impl RunSink for CoverSink {
    type Ctx = [u8];
    fn put(&mut self, cx: &mut [u8], offset: u64, bytes: &[u8]) -> Result<(), Fault> {
        for h in &mut self.hits[offset as usize..offset as usize + bytes.len()] {
            *h = h.saturating_add(1);
        }
        MemRunSink.put(cx, offset, bytes)
    }
    fn zeros(&mut self, cx: &mut [u8], offset: u64, len: u64) -> Result<(), Fault> {
        for h in &mut self.hits[offset as usize..(offset + len) as usize] {
            *h = h.saturating_add(1);
        }
        MemRunSink.zeros(cx, offset, len)
    }
}

/// A built run image.
pub struct Built {
    pub image: Vec<u8>,
    pub plan: RunPlan,
}

impl Built {
    pub fn image(&self, index: u8) -> MemImage<'_> {
        MemImage { bytes: &self.image, id: ImageId::Run { index, start_seq: 1 } }
    }
    /// Opens the run with rid base 0 and every rid contained.
    pub fn open(&self) -> Run<MemImage<'_>> {
        Run::open(self.image(self.plan.header.index), 0, 0, u64::MAX).expect("open")
    }
}

/// Builds a run into an image prefilled with `fill`; with `cover`, asserts that every byte was written once.
pub fn build(items: &VecItems, p: &RunParams, fill: u8, cover: bool) -> Result<Built, Fault> {
    let plan = plan_run(items, p)?;
    let len = plan.layout.len as usize;
    let mut image = vec![fill; len];
    let mut filter = vec![fill; plan.filter_scratch() as usize];
    if cover {
        let mut sink = CoverSink { hits: vec![0; len] };
        build_run(items, p, &plan, &mut sink, &mut image[..], &mut filter)?;
        if let Some(bad) = sink.hits.iter().position(|&h| h != 1) {
            panic!("byte {bad} written {} times (image of {len})", sink.hits[bad]);
        }
    } else {
        build_run(items, p, &plan, &mut MemRunSink, &mut image[..], &mut filter)?;
    }
    Ok(Built { image, plan })
}

// ---------------------------------------------------------------- the model

/// The shared prefix a run of `items` (in run order) stores: the whole words its first and last keys share,
/// short of the shorter key's end; none in an index with key runs (`l` > 0).
pub fn run_prefix(items: &[(Vec<u8>, u64, bool)], l: usize) -> usize {
    let (Some(a), Some(b)) = (items.first(), items.last()) else { return 0 };
    if l > 0 {
        return 0;
    }
    let shared = a.0.iter().zip(&b.0).take_while(|(x, y)| x == y).count() as u64;
    native_table::index::run::shared_prefix(shared, a.0.len().min(b.0.len()) as u64) as usize
}

/// An entry as the model forms it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MEnt {
    /// The full key.
    pub key: Vec<u8>,
    /// 0 row, 1 tombstone, 2 count.
    pub kind: u8,
    /// The last rid of a chunk, or the count of a count entry (as u64 bits).
    pub payload: u64,
    pub value: Vec<u8>,
    pub contribution: i64,
    /// The items the entry holds: (key, rid).
    pub items: Vec<(Vec<u8>, u64)>,
}

impl MEnt {
    /// The key as a run stores it for width w: the full key when longer than w, else zero-padded to w.
    pub fn stored(&self, w: u64) -> Vec<u8> {
        stored(&self.key, w)
    }
}

/// A key's stored form for width w.
pub fn stored(key: &[u8], w: u64) -> Vec<u8> {
    let mut k = key.to_vec();
    if (k.len() as u64) <= w {
        k.resize(w as usize, 0);
    }
    k
}

fn bits(x: u64) -> u32 {
    64 - x.leading_zeros()
}

/// The posting chunk of `rids` in v bytes, packed bit by bit.
pub fn model_posting(rids: &[u64], v: usize) -> Vec<u8> {
    let mut out = vec![0u8; v];
    let m = rids.len() - 1;
    let b = rids.windows(2).map(|w| bits(w[1] - w[0])).max().unwrap_or(0);
    out[0] = 0;
    out[1] = m as u8;
    out[2] = b as u8;
    let mut bit = 0usize;
    for w in rids.windows(2) {
        let g = w[1] - w[0];
        for k in 0..b as usize {
            if g >> k & 1 == 1 {
                out[3 + (bit + k) / 8] |= 1 << ((bit + k) % 8);
            }
        }
        bit += b as usize;
    }
    out
}

/// Whether `b` follows `a` in a key run of field length l.
pub fn model_step(a: &[u8], b: &[u8], l: usize) -> bool {
    if a.len() != b.len() || a.len() < l || a[..a.len() - l] != b[..b.len() - l] {
        return false;
    }
    let f = |k: &[u8]| k[k.len() - l..].iter().fold(0u128, |x, &y| x << 8 | y as u128);
    f(a) + 1 == f(b)
}

/// The main-tree entries of a run of `items` at value-part size v with key-run field length l, and every
/// distinct key with its total.
pub fn model(items: &[(Vec<u8>, u64, bool)], v: u64, l: usize) -> (Vec<MEnt>, Vec<(Vec<u8>, i64)>) {
    let mut out = Vec::new();
    let mut keys = Vec::new();
    // Groups of one key.
    let mut groups: Vec<&[(Vec<u8>, u64, bool)]> = Vec::new();
    let mut s = 0;
    for i in 1..=items.len() {
        if i == items.len() || items[i].0 != items[s].0 {
            groups.push(&items[s..i]);
            s = i;
        }
    }
    let lone = |g: &[(Vec<u8>, u64, bool)]| g.len() == 1 && !g[0].2;
    let mut gi = 0;
    while gi < groups.len() {
        let g = groups[gi];
        if l > 0 && v >= 8 && lone(g) {
            let mut e = gi + 1;
            while e < groups.len()
                && lone(groups[e])
                && model_step(&groups[e - 1][0].0, &groups[e][0].0, l)
                && groups[e - 1][0].1 + 1 == groups[e][0].1
            {
                e += 1;
            }
            for seg in groups[gi..e].chunks(1 << 32) {
                let last = &seg[seg.len() - 1][0];
                let n = seg.len();
                let value = if n >= 2 {
                    let mut x = vec![0u8; v as usize];
                    x[0] = 1;
                    x[1] = l as u8;
                    x[2..6].copy_from_slice(&((n - 1) as u32).to_le_bytes());
                    x
                } else {
                    model_posting(&[last.1], v as usize)
                };
                out.push(MEnt {
                    key: last.0.clone(),
                    kind: 0,
                    payload: last.1,
                    value,
                    contribution: n as i64,
                    items: seg.iter().map(|g| (g[0].0.clone(), g[0].1)).collect(),
                });
                for g in seg {
                    keys.push((g[0].0.clone(), 1));
                }
            }
            gi = e;
            continue;
        }
        let key = &g[0].0;
        let mut chunks: Vec<(bool, Vec<u64>)> = Vec::new();
        for &(_, rid, tomb) in g {
            let join = match chunks.last() {
                Some((t, rids)) if v > 0 && *t == tomb => {
                    // Joining makes rids.len() codes.
                    let m = rids.len() as u64;
                    let gaps = rids.windows(2).map(|w| bits(w[1] - w[0])).max().unwrap_or(0);
                    let b = gaps.max(bits(rid - rids[rids.len() - 1])) as u64;
                    m < CHUNK_RIDS_MAX && 3 + (m * b).div_ceil(8) <= v
                }
                _ => false,
            };
            if join {
                chunks.last_mut().unwrap().1.push(rid);
            } else {
                chunks.push((tomb, vec![rid]));
            }
        }
        let mut total = 0i64;
        for (tomb, rids) in &chunks {
            let c = rids.len() as i64 * if *tomb { -1 } else { 1 };
            total += c;
            out.push(MEnt {
                key: key.clone(),
                kind: *tomb as u8,
                payload: *rids.last().unwrap(),
                value: if v == 0 { vec![] } else { model_posting(rids, v as usize) },
                contribution: c,
                items: rids.iter().map(|&r| (key.clone(), r)).collect(),
            });
        }
        if chunks.len() as u64 >= SENT_MIN {
            out.push(MEnt {
                key: key.clone(),
                kind: 2,
                payload: total as u64,
                value: vec![0; v as usize],
                contribution: 0,
                items: vec![],
            });
        }
        keys.push((key.clone(), total));
        gi += 1;
    }
    (out, keys)
}
