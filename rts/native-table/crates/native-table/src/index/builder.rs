//! The run builder: signed items (key, rid, row or tombstone) in (key, rid) order become one run image.
//!
//! The input is read three times: to measure (and to check its order), to count the entries, overflow
//! records and key bytes at the chosen (W, V), whose totals fix where the overflow key bytes and the filter
//! start, and to build. Per key, the build pushes its chunks in rid order, then its count entry when it has
//! at least `SENT_MIN` chunks (a count entry's payload is the key's total in the run, rows less tombstones,
//! and its value part is zero); in an index with key runs, every maximal stretch of keys of one row item
//! each, stepping by one in key field and rid, becomes key-run entries of up to 2^32 items at V ≥ 8 (a
//! stretch of one key is an ordinary posting chunk). When 2d ≤ items a count tree, `IndexShape(W, 0)` with
//! one entry (key, count field of its total) per distinct key, is built beside the main tree into the slots
//! after the main tree's; both trees share the overflow records, the main tree's first. Every distinct key
//! enters the filter once. The run stores the bytes its first and last keys share as its shared prefix (in
//! whole words, short of the shorter key's end; none in an index with key runs), and its entries, fences
//! and overflow records hold each key past it: the measure takes the keys' lengths past it, and the
//! prefix's bytes go to the header and, past its room, to the start of the key bytes. The output bytes are
//! a function of the input and the parameters only.
//!
//! Contributions: a row entry 1 and a tombstone −1 at V = 0; a posting chunk of rows m + 1, of tombstones
//! −(m + 1); a key run n; a count entry 0.

use crate::consts::{BLOCK, FORMAT_KEY, PAGE, RID_PAYLOAD, SENT_MIN, W_MIN};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::index::chunk::{encode_key_run, encode_posting, key_run_step, Chunker, KEY_RUN, KEY_RUN_MAX};
use crate::index::plan::{has_count_tree, Measure};
use crate::index::run::{
    at, filter_set, record_offset, shared_prefix, RunHeader, RunLayout, F_COUNT_TREE, F_FILTER, F_KEYS_RISE,
    F_KEY_RUNS, F_PRIMARY, F_RELATIVE, PREFIX_HEAD, RECORDS_PER_BLOCK, RECORD_BYTES,
};
use crate::key::probe::{count_field, ovf_field, payload, row_field, tomb_field, PART};
use crate::mem::{copy_bytes, put};
use crate::tree::bulk::{plan_shape, BulkState, RunGeometry, RunSink};
use crate::tree::IndexShape;

/// One input item: a key's full encoding, a rid as the run stores it, and its sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item<'a> {
    pub key: &'a [u8],
    pub rid: u64,
    pub tomb: bool,
}

/// The builder's input, read by position, in (key, rid) order: keys by memcmp, rids ascending within a key,
/// and items at one (key, rid) all of one sign.
pub trait RunInput {
    fn items(&self) -> u64;
    fn item(&self, i: u64) -> Result<Item<'_>, Fault>;
}

/// What the run is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunParams {
    pub index: u8,
    /// Rids are relative to the run's rid base.
    pub relative: bool,
    /// The declared width W₀ (0: none).
    pub w0: u64,
    /// The index's key-run field length (0: no key runs).
    pub key_run_l: u8,
    pub table_uid: u64,
    pub schema_hash: u64,
    pub key_spec_hash: u64,
    /// Build with this (W, V) instead of choosing.
    pub force: Option<(u64, u64)>,
}

/// The counts and places of the run to build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunPlan {
    pub shape: IndexShape,
    pub header: RunHeader,
    pub layout: RunLayout,
    pub main_records: u64,
    pub main_key_bytes: u64,
    pub items: u64,
}

impl RunPlan {
    /// Bytes of filter scratch the build needs.
    pub fn filter_scratch(&self) -> u64 {
        self.layout.filter_blocks.saturating_mul(BLOCK)
    }
}

fn rt(kind: Corruption, off: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, off)
}

fn add(a: u64, b: u64) -> Result<u64, Fault> {
    a.checked_add(b).ok_or(rt(Corruption::Count, a))
}

fn len64(b: &[u8]) -> u64 {
    // A slice's length fits u64 on every target.
    u64::try_from(b.len()).unwrap_or(u64::MAX)
}

/// The end (exclusive) of the key group starting at item i.
fn group_end<I: RunInput>(input: &I, i: u64) -> Result<u64, Fault> {
    let key = input.item(i)?.key;
    let n = input.items();
    let mut j = add(i, 1)?;
    while j < n && input.item(j)?.key == key {
        j = add(j, 1)?;
    }
    Ok(j)
}

/// Whether item i is a key of one row item.
fn lone_row<I: RunInput>(input: &I, i: u64) -> Result<bool, Fault> {
    Ok(!input.item(i)?.tomb && group_end(input, i)? == add(i, 1)?)
}

/// The end (exclusive) of the maximal key-run stretch starting at item i, a lone row key.
fn stretch<I: RunInput>(input: &I, i: u64, l: u8) -> Result<u64, Fault> {
    let n = input.items();
    let mut k = i;
    loop {
        let next = add(k, 1)?;
        if next >= n || !lone_row(input, next)? {
            return Ok(next);
        }
        let (a, b) = (input.item(k)?, input.item(next)?);
        if !key_run_step(a.key, b.key, l) || a.rid.checked_add(1) != Some(b.rid) {
            return Ok(next);
        }
        k = next;
    }
}

/// Receives the run's entries in order.
trait Visit {
    /// A main-tree entry: its key, its field before overflow, its value part and its contribution.
    fn main(&mut self, key: &[u8], field: u64, value: &[u8], contribution: i64) -> Result<(), Fault>;
    /// A distinct key and its total, after its main-tree entries.
    fn key_done(&mut self, key: &[u8], total: i64) -> Result<(), Fault>;
}

/// Turns the input into entries at (W, V); `l` is the key-run field length (0: none).
fn walk<I: RunInput, X: Visit>(input: &I, v: u64, l: u8, vis: &mut X) -> Result<(), Fault> {
    let n = input.items();
    let vu = usize::try_from(v).map_err(|_| rt(Corruption::Layout, v))?;
    let mut val = [0u8; 256];
    let zeros = [0u8; 256];
    let mut rids = [0u64; 256];
    let mut i = 0u64;
    while i < n {
        let first = input.item(i)?;
        if l > 0 && v >= 8 && lone_row(input, i)? {
            let e = stretch(input, i, l)?;
            let mut s = i;
            while s < e {
                let t = e.min(add(s, KEY_RUN_MAX)?);
                let last = input.item(t.wrapping_sub(1))?;
                let size = t.wrapping_sub(s);
                let vb = val.get_mut(..vu).ok_or(rt(Corruption::Layout, v))?;
                if size >= 2 {
                    encode_key_run(l, size, vb)?;
                } else {
                    encode_posting(&[last.rid], vb)?;
                }
                let c = i64::try_from(size).map_err(|_| rt(Corruption::Count, size))?;
                vis.main(last.key, row_field(last.rid), vb, c)?;
                let mut q = s;
                while q < t {
                    vis.key_done(input.item(q)?.key, 1)?;
                    q = add(q, 1)?;
                }
                s = t;
            }
            i = e;
            continue;
        }
        let j = group_end(input, i)?;
        let key = first.key;
        let mut chunker = Chunker::new(v);
        let mut held = 0usize;
        let mut held_tomb = false;
        let mut chunks = 0u64;
        let mut total = 0i64;
        let flush = |rids: &[u64], tomb: bool, val: &mut [u8; 256], vis: &mut X| -> Result<i64, Fault> {
            let last = *rids.last().ok_or(rt(Corruption::Count, i))?;
            let items = i64::try_from(rids.len()).map_err(|_| rt(Corruption::Count, i))?;
            let (field, c) = if tomb { (tomb_field(last), items.wrapping_neg()) } else { (row_field(last), items) };
            let vb = val.get_mut(..vu).ok_or(rt(Corruption::Layout, v))?;
            if v > 0 {
                encode_posting(rids, vb)?;
            }
            vis.main(key, field, vb, c)?;
            Ok(c)
        };
        let mut q = i;
        while q < j {
            let x = input.item(q)?;
            let joined = chunker.offer(x.rid, x.tomb);
            if !joined && held > 0 {
                let c = flush(rids.get(..held).unwrap_or(&[]), held_tomb, &mut val, vis)?;
                total = total.checked_add(c).ok_or(rt(Corruption::Count, q))?;
                chunks = add(chunks, 1)?;
                held = 0;
            }
            *rids.get_mut(held).ok_or(rt(Corruption::Count, q))? = x.rid;
            held = held.wrapping_add(1);
            held_tomb = x.tomb;
            q = add(q, 1)?;
        }
        if held > 0 {
            let c = flush(rids.get(..held).unwrap_or(&[]), held_tomb, &mut val, vis)?;
            total = total.checked_add(c).ok_or(rt(Corruption::Count, j))?;
            chunks = add(chunks, 1)?;
        }
        if chunks >= SENT_MIN {
            vis.main(key, count_field(total), zeros.get(..vu).unwrap_or(&[]), 0)?;
        }
        vis.key_done(key, total)?;
        i = j;
    }
    Ok(())
}

/// The run's shared prefix: the whole words the first and last items' keys share, short of the shorter key's
/// end; 0 in an index with key runs (`l` > 0), whose items' keys differ in their last bytes.
fn prefix_of<I: RunInput>(input: &I, l: u8) -> Result<u64, Fault> {
    let n = input.items();
    if l > 0 || n == 0 {
        return Ok(0);
    }
    let (a, b) = (input.item(0)?.key, input.item(n.wrapping_sub(1))?.key);
    let shared = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count();
    Ok(shared_prefix(len64(a.get(..shared).unwrap_or(&[])), len64(a).min(len64(b))))
}

/// A key past the run's shared prefix of `p` bytes (every key is longer).
fn stored(key: &[u8], p: usize) -> Result<&[u8], Fault> {
    key.get(p..).ok_or(rt(Corruption::Order, p as u64))
}

/// Totals of the measure pass beyond the candidates' counts.
struct Stats {
    rid_lo: u64,
    rid_hi: u64,
    rise: bool,
    stretches: bool,
}

/// The measure pass: checks the input's order and feeds `Measure`, with the keys' lengths past the shared
/// prefix of `p` bytes.
fn measure<I: RunInput>(input: &I, l: u8, p: u64) -> Result<(Measure, Stats), Fault> {
    let n = input.items();
    if n == 0 {
        return Err(rt(Corruption::Count, 0));
    }
    let mut m = Measure::new();
    let mut st = Stats { rid_lo: u64::MAX, rid_hi: 0, rise: true, stretches: false };
    // The order: keys by memcmp with no key a proper prefix of the next; rids ascending within a key; items at
    // one position of one sign.
    let mut i = 0u64;
    while i < n {
        let x = input.item(i)?;
        if x.rid > RID_PAYLOAD {
            return Err(rt(Corruption::Range, i));
        }
        st.rid_lo = st.rid_lo.min(x.rid);
        st.rid_hi = st.rid_hi.max(x.rid);
        if i > 0 {
            let p = input.item(i.wrapping_sub(1))?;
            let ok = match p.key.cmp(x.key) {
                core::cmp::Ordering::Less => !x.key.starts_with(p.key),
                core::cmp::Ordering::Equal => p.rid < x.rid || (p.rid == x.rid && p.tomb == x.tomb),
                core::cmp::Ordering::Greater => false,
            };
            if !ok {
                return Err(rt(Corruption::Order, i));
            }
            if x.rid < p.rid {
                st.rise = false;
            }
        }
        i = add(i, 1)?;
    }
    let mut i = 0u64;
    while i < n {
        let x = input.item(i)?;
        if l > 0 && lone_row(input, i)? {
            let e = stretch(input, i, l)?;
            let size = e.wrapping_sub(i);
            if size >= 2 {
                st.stretches = true;
            }
            m.key_run(size, len64(x.key).saturating_sub(p))?;
            i = e;
            continue;
        }
        let j = group_end(input, i)?;
        m.begin_key();
        let mut q = i;
        while q < j {
            let y = input.item(q)?;
            m.item(len64(y.key).saturating_sub(p), y.rid, y.tomb)?;
            q = add(q, 1)?;
        }
        m.end_key()?;
        i = j;
    }
    Ok((m, st))
}

/// The counting pass at the chosen (W, V).
struct Counter {
    /// The shared prefix's bytes.
    p: usize,
    w: u64,
    ct: bool,
    n: u64,
    main_records: u64,
    main_key_bytes: u64,
    ct_entries: u64,
    ct_records: u64,
    ct_key_bytes: u64,
    key_runs: bool,
    v: u64,
}

impl Visit for Counter {
    fn main(&mut self, key: &[u8], _field: u64, value: &[u8], _c: i64) -> Result<(), Fault> {
        self.n = add(self.n, 1)?;
        let len = len64(stored(key, self.p)?);
        if len > self.w {
            self.main_records = add(self.main_records, 1)?;
            self.main_key_bytes = add(self.main_key_bytes, len.wrapping_sub(self.w))?;
        }
        if self.v >= 8 && value.first() == Some(&KEY_RUN) {
            self.key_runs = true;
        }
        Ok(())
    }
    fn key_done(&mut self, key: &[u8], _total: i64) -> Result<(), Fault> {
        if self.ct {
            self.ct_entries = add(self.ct_entries, 1)?;
            let len = len64(stored(key, self.p)?);
            if len > self.w {
                self.ct_records = add(self.ct_records, 1)?;
                self.ct_key_bytes = add(self.ct_key_bytes, len.wrapping_sub(self.w))?;
            }
        }
        Ok(())
    }
}

/// Measures the input, chooses (W, V) (or takes the forced pair) and counts what the run will hold.
pub fn plan_run<I: RunInput>(input: &I, p: &RunParams) -> Result<RunPlan, Fault> {
    let prefix = prefix_of(input, p.key_run_l)?;
    let (m, st) = measure(input, p.key_run_l, prefix)?;
    let (w, v) = match p.force {
        Some((w, v)) => {
            if IndexShape::new(w, v).is_none() || w < p.w0.max(W_MIN) {
                return Err(rt(Corruption::Layout, w));
            }
            (w, v)
        }
        None => {
            let c = m.choose(p.w0)?;
            (c.w, c.v)
        }
    };
    let shape = IndexShape::new(w, v).ok_or(rt(Corruption::Layout, w))?;
    let ct = has_count_tree(m.t.d, m.t.items);
    let mut c = Counter {
        p: usize::try_from(prefix).map_err(|_| rt(Corruption::Layout, prefix))?,
        w,
        ct,
        n: 0,
        main_records: 0,
        main_key_bytes: 0,
        ct_entries: 0,
        ct_records: 0,
        ct_key_bytes: 0,
        key_runs: false,
        v,
    };
    walk(input, v, p.key_run_l, &mut c)?;
    let (nodes, main_height) = plan_shape(c.n, shape.leaf.cap, shape.internal.cap)?;
    let (ct_nodes, ct_height) = if ct {
        let cs = IndexShape::new(w, 0).ok_or(rt(Corruption::Layout, w))?;
        plan_shape(c.ct_entries, cs.leaf.cap, cs.internal.cap)?
    } else {
        (0, 0)
    };
    let records = add(c.main_records, c.ct_records)?;
    // The key bytes: the prefix's past the header's room, then the overflowed keys'.
    let key_bytes = add(add(c.main_key_bytes, c.ct_key_bytes)?, prefix.saturating_sub(PREFIX_HEAD))?;
    let layout = RunLayout::new(nodes, ct_nodes, records, key_bytes, m.t.d)?;
    let mut flags = 0u8;
    if p.relative {
        flags |= F_RELATIVE;
    }
    if layout.filter != 0 {
        flags |= F_FILTER;
    }
    if p.index == 0 {
        flags |= F_PRIMARY;
    }
    if c.key_runs && st.stretches {
        flags |= F_KEY_RUNS;
    }
    if st.rise {
        flags |= F_KEYS_RISE;
    }
    if ct {
        flags |= F_COUNT_TREE;
    }
    let rows = i64::try_from(m.t.rows).map_err(|_| rt(Corruption::Count, m.t.rows))?;
    let tombs = i64::try_from(m.t.tombs).map_err(|_| rt(Corruption::Count, m.t.tombs))?;
    let mut header = RunHeader {
        version: 0,
        flags,
        bytes: layout.len,
        table_uid: p.table_uid,
        schema_hash: p.schema_hash,
        index: p.index,
        w,
        v,
        format_key: FORMAT_KEY,
        n: c.n,
        nodes,
        height: main_height,
        root: BLOCK,
        records,
        oa: layout.oa,
        ok: layout.ok,
        filter: layout.filter,
        filter_blocks: layout.filter_blocks,
        lines: layout.lines,
        rid_lo: st.rid_lo,
        rid_hi: st.rid_hi,
        key_spec_hash: p.key_spec_hash,
        rows: m.t.rows,
        total: rows.checked_sub(tombs).ok_or(rt(Corruption::Count, m.t.rows))?,
        d: m.t.d,
        key_run_l: if flags & F_KEY_RUNS != 0 { p.key_run_l } else { 0 },
        tombs: m.t.tombs,
        ct_root: if ct { add(nodes, 1)?.checked_mul(BLOCK).ok_or(rt(Corruption::Count, nodes))? } else { 0 },
        ct_nodes,
        ct_height,
        ct_entries: c.ct_entries,
        prefix,
    };
    header.version = header.version_needed();
    Ok(RunPlan {
        shape,
        header,
        layout,
        main_records: c.main_records,
        main_key_bytes: c.main_key_bytes,
        items: m.t.items,
    })
}

/// The writing pass.
struct Writer<'s, K: RunSink + ?Sized> {
    sink: &'s mut K,
    cx: &'s mut K::Ctx,
    /// The shared prefix's bytes: every key is written past them.
    p: usize,
    w: u64,
    main: BulkState,
    ct: Option<BulkState>,
    oa: u64,
    main_rec: u64,
    main_key: u64,
    ct_rec: u64,
    ct_key: u64,
    filter: &'s mut [u8],
    lines: u64,
    first: [u8; PART],
    last: [u8; PART],
    have_first: bool,
}

impl<K: RunSink + ?Sized> Writer<'_, K> {
    /// Writes overflow record x for `key` with the field's payload, its key bytes beyond W at `*key_at`.
    fn record(&mut self, x: u64, field: u64, key: &[u8], key_at: u64) -> Result<u64, Fault> {
        let off = record_offset(self.oa, x).ok_or(rt(Corruption::Count, x))?;
        let len = len64(key);
        let mut rec = [0u8; 24];
        put(&mut rec, 0, &payload(field).to_le_bytes())?;
        put(&mut rec, 8, &key_at.to_le_bytes())?;
        put(&mut rec, 16, &len.to_le_bytes())?;
        self.sink.put(self.cx, off, &rec)?;
        let wu = usize::try_from(self.w).map_err(|_| rt(Corruption::Layout, self.w))?;
        let tail = key.get(wu..).ok_or(rt(Corruption::Layout, x))?;
        self.sink.put(self.cx, key_at, tail)?;
        add(key_at, len.wrapping_sub(self.w))
    }
}

impl<K: RunSink + ?Sized> Visit for Writer<'_, K> {
    fn main(&mut self, key: &[u8], field: u64, value: &[u8], c: i64) -> Result<(), Fault> {
        let key = stored(key, self.p)?;
        let mut f = field;
        if len64(key) > self.w {
            let x = self.main_rec;
            self.main_key = self.record(x, field, key, self.main_key)?;
            self.main_rec = add(x, 1)?;
            f = ovf_field(field, x);
        }
        // The fence form: the key part (area, then the field big-endian) and the value part.
        let wu = usize::try_from(self.w).map_err(|_| rt(Corruption::Layout, self.w))?;
        self.last.fill(0);
        let take = key.len().min(wu);
        copy_bytes(self.last.get_mut(..take).unwrap_or(&mut []), key.get(..take).unwrap_or(&[]));
        put(&mut self.last, self.w, &f.to_be_bytes())?;
        put(&mut self.last, add(self.w, 8)?, value)?;
        if !self.have_first {
            self.first = self.last;
            self.have_first = true;
        }
        self.main.push(self.sink, self.cx, key, f, value, c)
    }

    fn key_done(&mut self, key: &[u8], total: i64) -> Result<(), Fault> {
        if self.lines > 0 {
            filter_set(self.filter, self.lines, key)?;
        }
        let key = stored(key, self.p)?;
        if self.ct.is_some() {
            let field = count_field(total);
            let mut f = field;
            if len64(key) > self.w {
                let x = self.ct_rec;
                self.ct_key = self.record(x, field, key, self.ct_key)?;
                self.ct_rec = add(x, 1)?;
                f = ovf_field(field, x);
            }
            if let Some(ct) = self.ct.as_mut() {
                ct.push(self.sink, self.cx, key, f, &[], 0)?;
            }
        }
        Ok(())
    }
}

/// Builds the planned run through `sink`, covering every byte of the image [0, plan.layout.len) exactly
/// once. `filter` is scratch of at least `plan.filter_scratch()` bytes.
pub fn build_run<I: RunInput, K: RunSink + ?Sized>(
    input: &I,
    p: &RunParams,
    plan: &RunPlan,
    sink: &mut K,
    cx: &mut K::Ctx,
    filter: &mut [u8],
) -> Result<(), Fault> {
    let h = &plan.header;
    let lay = &plan.layout;
    let fneed = plan.filter_scratch();
    let fbuf = filter.get_mut(..usize::try_from(fneed).map_err(|_| Fault::Scratch { need: fneed })?);
    let fbuf = fbuf.ok_or(Fault::Scratch { need: fneed })?;
    fbuf.fill(0);
    let geo = RunGeometry { first_slot: 1, first_overflow: 0, slots: lay.slots, v: lay.records };
    let main = BulkState::new(plan.shape, h.n, geo)?;
    let ct = if h.has_count_tree() {
        let cs = IndexShape::new(h.w, 0).ok_or(rt(Corruption::Layout, h.w))?;
        let cg = RunGeometry {
            first_slot: add(h.nodes, 1)?,
            first_overflow: plan.main_records,
            slots: lay.slots,
            v: lay.records,
        };
        Some(BulkState::new(cs, h.ct_entries, cg)?)
    } else {
        None
    };
    let pu = usize::try_from(h.prefix).map_err(|_| rt(Corruption::Layout, h.prefix))?;
    let tail = h.prefix_tail();
    let mut wr = Writer {
        sink,
        cx,
        p: pu,
        w: h.w,
        main,
        ct,
        oa: lay.oa,
        main_rec: 0,
        main_key: add(lay.ok, tail)?,
        ct_rec: plan.main_records,
        ct_key: add(add(lay.ok, tail)?, plan.main_key_bytes)?,
        filter: fbuf,
        lines: lay.lines,
        first: [0; PART],
        last: [0; PART],
        have_first: false,
    };
    walk(input, h.v, p.key_run_l, &mut wr)?;
    wr.main.finish(wr.sink, wr.cx)?;
    if let Some(ct) = wr.ct.as_mut() {
        ct.finish(wr.sink, wr.cx)?;
    }
    if wr.main_rec != plan.main_records || wr.ct_rec != lay.records {
        return Err(rt(Corruption::Count, wr.main_rec));
    }
    // Slot 0: the header, the fences and the shared prefix's first bytes; the prefix's rest starts the key
    // bytes.
    let mut slot0 = [0u8; 4_096];
    h.write(&mut slot0)?;
    put(&mut slot0, at::FENCE_FIRST, &wr.first)?;
    put(&mut slot0, at::FENCE_LAST, &wr.last)?;
    if pu > 0 {
        let first = input.item(0)?.key;
        let head = usize::try_from(PREFIX_HEAD).unwrap_or(0).min(pu);
        put(&mut slot0, at::PREFIX, first.get(..head).ok_or(rt(Corruption::Order, 0))?)?;
        if tail > 0 {
            wr.sink.put(wr.cx, lay.ok, first.get(head..pu).ok_or(rt(Corruption::Order, 0))?)?;
        }
    }
    wr.sink.put(wr.cx, 0, &slot0)?;
    // Gaps: after the node slots, after each directory block's records, after the key bytes.
    let nodes_end = add(lay.slots, 1)?.checked_mul(BLOCK).ok_or(rt(Corruption::Count, lay.slots))?;
    if lay.oa > nodes_end {
        wr.sink.zeros(wr.cx, nodes_end, lay.oa.wrapping_sub(nodes_end))?;
    }
    let mut b = 0u64;
    while b < lay.records.div_ceil(RECORDS_PER_BLOCK) {
        let base = b.checked_mul(PAGE).and_then(|x| x.checked_add(lay.oa)).ok_or(rt(Corruption::Count, b))?;
        let in_block = lay.records.saturating_sub(b.saturating_mul(RECORDS_PER_BLOCK)).min(RECORDS_PER_BLOCK);
        let used = in_block.wrapping_mul(RECORD_BYTES);
        wr.sink.zeros(wr.cx, add(base, used)?, PAGE.wrapping_sub(used))?;
        b = add(b, 1)?;
    }
    let kend = add(lay.ok, lay.key_bytes)?;
    let tail_end = if lay.filter != 0 { lay.filter } else { lay.len };
    if tail_end > kend {
        wr.sink.zeros(wr.cx, kend, tail_end.wrapping_sub(kend))?;
    }
    if lay.filter != 0 {
        wr.sink.put(wr.cx, lay.filter, wr.filter)?;
    }
    Ok(())
}
