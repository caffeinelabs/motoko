//! Runs: the bulk build and its plan, the chunk codec, measuring and choosing (W, V), count trees, prefix
//! sums and rank selection, reading runs through the tree's find and cursors, believed runs, and the golden
//! run images.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound::{self, *};

use native_table::consts::{BLOCK, CHUNK_RIDS_MAX, NODE_SIZE, PAGE, RID_PAYLOAD, SENT_MIN, V_CHOICES, W_MAX};
use native_table::index::chunk::{self, Chunk, Chunker};
use native_table::index::nodes::StructureCheck;
use native_table::index::run::{at, check_header, stored_key, RunExpect};
use native_table::index::{plan_run, Finger, MemImage, Run, RunParams};
use native_table::key::enc_bytes;
use native_table::key::probe::{
    count_field, is_count, is_ovf, is_tomb, ovf_field, payload, payload_to_count, KeyProbe, Kind,
};
use native_table::tree::bulk::{Plan, TRAILER_UNIT};
use native_table::tree::{BulkState, Dir, Direction, Fwd, IndexShape, Rev, RunGeometry, Shape};
use native_table::{CorruptAt, Corruption, Fault, ImageId, Progress, RefusalCode};
use nt_testhost::runs::{build, model, model_posting, stored, Built, CoverSink, MEnt, VecItems};
use nt_testhost::{VecSink, XorShift, FILLS};

type Items = Vec<(Vec<u8>, u64, bool)>;

fn params(index: u8) -> RunParams {
    RunParams {
        index,
        relative: true,
        w0: 0,
        key_run_l: 0,
        table_uid: 0x1111,
        schema_hash: 0x2222,
        key_spec_hash: 0x3333,
        force: None,
    }
}

fn forced(w: u64, v: u64) -> RunParams {
    RunParams { force: Some((w, v)), ..params(1) }
}

/// A fixed-width key: 8 bytes big-endian.
fn k8(x: u64) -> Vec<u8> {
    x.to_be_bytes().to_vec()
}

/// A text key: the escaped, terminated encoding of `s`.
fn text(s: &[u8]) -> Vec<u8> {
    let mut v = VecSink::default();
    enc_bytes(&mut v, s).unwrap();
    v.0
}

const RUN_ID: ImageId = ImageId::Run { index: 1, start_seq: 1 };

fn is_run_fault(f: &Fault, id: ImageId) -> bool {
    matches!(f, Fault::Corrupt(CorruptAt { image, .. }) if *image == id)
}

/// One entry of a run as read back: its key (the run's shared prefix, then the stored key), kind (0 row, 1
/// tomb, 2 count), payload, value, contribution.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Got {
    key: Vec<u8>,
    kind: u8,
    payload: u64,
    value: Vec<u8>,
    contribution: i64,
}

/// A model entry as `decode` reads it back from a run of width w whose shared prefix is p bytes: the prefix,
/// then the key past it as the run stores it.
fn got_of(m: &MEnt, w: u64, p: usize) -> Got {
    let mut key = m.key[..p].to_vec();
    key.extend(stored(&m.key[p..], w));
    Got { key, kind: m.kind, payload: m.payload, value: m.value.clone(), contribution: m.contribution }
}

fn decode(run: &Run<MemImage<'_>>, n: u64, i: u64) -> Got {
    let t = run.main().unwrap();
    let e = t.entry(n, i).unwrap();
    let mut key = vec![0u8; 1 << 16];
    let p = run.prefix_into(&mut key).unwrap();
    let len = stored_key(&t, &e, &mut key[p..]).unwrap();
    key.truncate(p + len);
    let kind = if is_count(e.field) {
        2
    } else if is_tomb(e.field) {
        1
    } else {
        0
    };
    let p = if is_ovf(e.field) { run.record(e.field).unwrap().payload } else { payload(e.field) };
    let payload = if kind == 2 { payload_to_count(p) as u64 } else { p };
    Got { key, kind, payload, value: e.value.to_vec(), contribution: t.contribution(&e).unwrap() }
}

/// A probe of a key in full as the run's trees take it: past the shared prefix (`Err`: the key lies below,
/// `Less`, or above every key of the run).
fn stored_probe<'k>(run: &Run<MemImage<'_>>, p: &KeyProbe<'k>) -> Result<KeyProbe<'k>, Ordering> {
    run.strip(p.key).unwrap().map(|key| KeyProbe { key, kind: p.kind })
}

/// The entries between two bounds in direction D, the bounds' keys in full: a bound outside the run's prefix
/// bounds nothing (it lies past the run's end it faces) or leaves nothing (it lies before its own end).
fn scan<'k, D: Direction>(run: &Run<MemImage<'_>>, lo: Bound<&KeyProbe<'k>>, hi: Bound<&KeyProbe<'k>>) -> Vec<Got> {
    let fwd = !D::REV;
    // `start`: the bound the scan starts from (it faces the run's first key going forward, its last going back).
    let conv = |b: Bound<&KeyProbe<'k>>, start: bool| -> Option<Bound<KeyProbe<'k>>> {
        let p = match b {
            Unbounded => return Some(Unbounded),
            Included(p) | Excluded(p) => p,
        };
        match stored_probe(run, p) {
            Ok(q) => Some(if matches!(b, Included(_)) { Included(q) } else { Excluded(q) }),
            Err(o) => {
                let past_start = (o == Ordering::Less) == (start == fwd);
                if past_start {
                    Some(Unbounded)
                } else {
                    None
                }
            }
        }
    };
    let (Some(lo), Some(hi)) = (conv(lo, true), conv(hi, false)) else { return Vec::new() };
    let t = run.main().unwrap();
    let mut c = t.seek::<D>(lo.as_ref()).unwrap();
    let mut out = Vec::new();
    while let Some((n, i)) = t.next(&mut c, hi.as_ref()).unwrap() {
        out.push(decode(run, n, i));
    }
    out
}

/// Random items: `keys` text keys of random lengths in [min_len, max_len] sharing a prefix of half their
/// length, each with 1 to `per` distinct rids below `span`; with `tombs`, about one item in five a tombstone.
fn random_items(
    g: &mut XorShift,
    keys: usize,
    per: u64,
    span: u64,
    min_len: usize,
    max_len: usize,
    tombs: bool,
) -> Items {
    let mut m: BTreeMap<Vec<u8>, ()> = BTreeMap::new();
    while m.len() < keys {
        let len = min_len + g.below((max_len - min_len + 1) as u64) as usize;
        let raw: Vec<u8> = (0..len).map(|j| if j < len / 2 { 0x41 } else { (g.next_u64() % 250) as u8 + 1 }).collect();
        m.insert(text(&raw), ());
    }
    let mut items = Vec::new();
    for k in m.keys() {
        let n = 1 + g.below(per);
        let mut rids: Vec<u64> = (0..n).map(|_| g.below(span)).collect();
        rids.sort();
        rids.dedup();
        for r in rids {
            items.push((k.clone(), r, tombs && g.below(5) == 0));
        }
    }
    items.sort();
    items
}

// ---------------------------------------------------------------- the model's order

/// The comparator over a model entry (on the left) and a probe: key by memcmp, then rank, then rid, a
/// tombstone after a row at one rid.
fn cmp_model(e: &MEnt, p: &KeyProbe<'_>) -> Ordering {
    let o = e.key.as_slice().cmp(p.key);
    if o != Ordering::Equal {
        return o;
    }
    let er = if e.kind == 2 { 2 } else { 1 };
    let pr = match p.kind {
        Kind::Low => 0,
        Kind::Row(_) => 1,
        Kind::Count => 2,
    };
    if er != pr {
        return er.cmp(&pr);
    }
    match p.kind {
        Kind::Row(r) => match e.payload.cmp(&r) {
            Ordering::Equal if e.kind == 1 => Ordering::Greater,
            o => o,
        },
        _ => Ordering::Equal,
    }
}

fn in_range(e: &MEnt, lo: Bound<&KeyProbe<'_>>, hi: Bound<&KeyProbe<'_>>) -> bool {
    let above = match lo {
        Unbounded => true,
        Included(p) => cmp_model(e, p) != Ordering::Less,
        Excluded(p) => cmp_model(e, p) == Ordering::Greater,
    };
    let below = match hi {
        Unbounded => true,
        Included(p) => cmp_model(e, p) != Ordering::Greater,
        Excluded(p) => cmp_model(e, p) == Ordering::Less,
    };
    above && below
}

/// Decodes a posting chunk's rids forward and backward with the crate's codec; both must agree.
fn posting_rids(value: &[u8], last: u64) -> Vec<u64> {
    let Chunk::Posting { m, b } = chunk::header(value, RUN_ID, 0).unwrap() else { panic!("not a posting") };
    let mut r = chunk::first(value, m, b, last, RUN_ID, 0).unwrap();
    let mut fwd = vec![r];
    for k in 1..=m {
        r += chunk::code(value, m, b, k, RUN_ID, 0).unwrap();
        fwd.push(r);
    }
    assert_eq!(*fwd.last().unwrap(), last);
    let mut r = last;
    let mut rev = vec![r];
    for k in (1..=m).rev() {
        r -= chunk::code(value, m, b, k, RUN_ID, 0).unwrap();
        rev.push(r);
    }
    rev.reverse();
    assert_eq!(fwd, rev);
    fwd
}

// ---------------------------------------------------------------- bulk_plan

const CAPS: [(u64, u64); 4] = [(3, 3), (101, 84), (254, 169), (28, 26)];
/// The index shapes with those capacities.
const CAP_SHAPES: [(u64, u64); 4] = [(1_016, 0), (32, 0), (8, 0), (8, 128)];

#[test]
fn bulk_plan_invariants_every_n() {
    for (k, &(nl, ni)) in CAPS.iter().enumerate() {
        let (w, v) = CAP_SHAPES[k];
        let s = IndexShape::new(w, v).unwrap();
        assert_eq!((s.leaf.cap, s.internal.cap), (nl, ni), "shape of capacities ({nl}, {ni})");
        for n in 1..=(1u64 << 17) {
            let p = Plan::new(n, nl, ni).unwrap();
            let h = p.height as usize;
            assert!(h >= 1 && h <= 64);
            assert_eq!(p.levels[0].nodes, (n + 1).div_ceil(nl + 1), "n = {n}");
            assert_eq!(p.levels[h - 1].nodes, 1, "one root");
            let mut sum = 0u64;
            for l in 0..h {
                let lv = p.levels[l];
                let cap = if l == 0 { nl } else { ni };
                assert_eq!(lv.cap, cap);
                sum += lv.entries;
                if l > 0 {
                    assert_eq!(lv.entries + lv.nodes, p.levels[l - 1].nodes, "children of level {l}, n = {n}");
                }
                let nodes = lv.nodes;
                if nodes == 1 {
                    assert!(p.count(l as u64, 0) >= 1 && p.count(l as u64, 0) <= cap);
                    assert_eq!(p.count(l as u64, 0), lv.entries);
                    continue;
                }
                // Counts take at most three values: c everywhere but the last two nodes.
                let a = p.count(l as u64, nodes - 2);
                let b = p.count(l as u64, nodes - 1);
                if nodes >= 3 {
                    assert_eq!(p.count(l as u64, 0), cap);
                    assert_eq!(p.count(l as u64, nodes - 3), cap);
                }
                for c in [p.count(l as u64, 0), a, b] {
                    assert!(c >= cap / 2 && c <= cap, "min fill: n = {n}, level {l}, count {c}");
                }
                assert_eq!((nodes - 2) * cap + a + b, lv.entries, "level sum: n = {n}, level {l}");
                assert_eq!(p.count(l as u64, nodes), 0);
            }
            assert_eq!(sum, n);
            assert_eq!(native_table::tree::bulk::plan_nodes(n, nl, ni).unwrap(), p.nodes());
        }
    }
}

/// A bulk build of n synthetic entries: keys k8(i) (every seventh key 13 bytes long, overflowing W = 8),
/// contributions mixing 1 with others; returns the image and the pushed entries.
struct Synth {
    keys: Vec<Vec<u8>>,
    fields: Vec<u64>,
    values: Vec<Vec<u8>>,
    contribs: Vec<i64>,
}

fn synth(n: u64, shape: IndexShape, seed: u64) -> Synth {
    let mut g = XorShift(seed | 1);
    let mut s = Synth { keys: vec![], fields: vec![], values: vec![], contribs: vec![] };
    let mut x = 0u64;
    for i in 0..n {
        let mut k = (i * 3 + 1).to_be_bytes().to_vec();
        let long = shape.w == 8 && i % 7 == 3;
        if long {
            k.extend_from_slice(&[0xEE; 5]);
        }
        let base = row_field_of(i);
        let field = if long {
            x += 1;
            ovf_field(base, x - 1)
        } else {
            base
        };
        // Runs of contribution 1 make UNIT nodes; elsewhere mixed values.
        let c = if (i / 300) % 2 == 0 { 1 } else { [1, 2, 0, -1, 5][g.below(5) as usize] };
        s.keys.push(k);
        s.fields.push(field);
        s.values.push((0..shape.v).map(|j| (i as u8).wrapping_add(j as u8) | 1).collect());
        s.contribs.push(c);
    }
    s
}

fn row_field_of(i: u64) -> u64 {
    native_table::key::probe::row_field(i)
}

/// Checks a built tree in `image` from `root`: structure, plan fill, in-order entries, child words, trailers
/// (cumulative counts recounted, UNIT flags exact) and zeroed unused bytes.
fn check_tree(image: &[u8], shape: IndexShape, root: u64, s: &Synth, first_slot: u64, nodes: u64) {
    struct St<'a> {
        image: &'a [u8],
        shape: IndexShape,
        s: &'a Synth,
        next: usize,
        cum: i64,
        leaf_depth: Option<usize>,
        seen: u64,
    }
    fn walk(st: &mut St<'_>, n: u64, depth: usize, root: bool) {
        let b = &st.image[n as usize..n as usize + BLOCK as usize];
        let count = u16::from_le_bytes([b[0], b[1]]) as u64;
        let flags = u16::from_le_bytes([b[2], b[3]]);
        let layout = u32::from_le_bytes(b[4..8].try_into().unwrap());
        assert_eq!(layout, st.shape.layout());
        let leaf = flags == 3;
        assert!(flags == 3 || flags == 2);
        let sh = st.shape.shape(leaf);
        assert!(count <= sh.cap && (root || count >= sh.cap / 2), "fill of node {n}: {count}");
        st.seen += 1;
        let trailer_cum = i64::from_le_bytes(b[4080..4088].try_into().unwrap());
        let trailer_flags = u64::from_le_bytes(b[4088..4096].try_into().unwrap());
        assert_eq!(trailer_cum, st.cum, "cumulative count of node {n}");
        let ks = (st.shape.w + 8) as usize;
        let v = st.shape.v as usize;
        let mut unit = true;
        let check_entry = |st: &mut St<'_>, i: u64, unit: &mut bool| {
            let k = &b[8 + i as usize * ks..8 + (i as usize + 1) * ks];
            let e = st.next;
            let w = st.shape.w as usize;
            let key = &st.s.keys[e];
            let take = key.len().min(w);
            assert_eq!(&k[..take], &key[..take]);
            assert!(k[take..w].iter().all(|&x| x == 0));
            assert_eq!(u64::from_be_bytes(k[w..].try_into().unwrap()), st.s.fields[e]);
            let vo = sh.voff as usize + i as usize * v;
            assert_eq!(&b[vo..vo + v], &st.s.values[e][..]);
            st.cum += st.s.contribs[e];
            if st.s.contribs[e] != 1 {
                *unit = false;
            }
            st.next += 1;
        };
        let kend = 8 + count as usize * ks;
        assert!(b[kend..sh.voff as usize].iter().all(|&x| x == 0));
        if leaf {
            match st.leaf_depth {
                Some(d) => assert_eq!(d, depth),
                None => st.leaf_depth = Some(depth),
            }
            for i in 0..count {
                check_entry(st, i, &mut unit);
            }
            assert!(b[sh.voff as usize + count as usize * v..NODE_SIZE as usize].iter().all(|&x| x == 0));
        } else {
            for j in 0..=count {
                let c = u64::from_le_bytes(b[sh.coff as usize + 8 * j as usize..][..8].try_into().unwrap());
                walk(st, c, depth + 1, false);
                if j < count {
                    check_entry(st, j, &mut unit);
                }
            }
            assert!(b[sh.voff as usize + count as usize * v..sh.coff as usize].iter().all(|&x| x == 0));
            assert!(b[sh.coff as usize + 8 * (count as usize + 1)..NODE_SIZE as usize].iter().all(|&x| x == 0));
        }
        assert_eq!(trailer_flags, if unit { TRAILER_UNIT } else { 0 }, "UNIT of node {n}");
    }
    let mut st = St { image, shape, s, next: 0, cum: 0, leaf_depth: None, seen: 0 };
    assert_eq!(root, first_slot * BLOCK);
    walk(&mut st, root, 0, true);
    assert_eq!(st.next, s.keys.len());
    assert_eq!(st.seen, nodes);
}

/// Builds n synthetic entries with `BulkState`; checks every byte of its slots written once and returns the
/// slots' bytes.
fn bulk_build(n: u64, shape: IndexShape) -> Vec<u8> {
    let s = synth(n, shape, n.wrapping_mul(0x9E37) ^ shape.v);
    let plan = Plan::new(n, shape.leaf.cap, shape.internal.cap).unwrap();
    let nodes = plan.nodes();
    let first_slot = 3;
    let v = s.fields.iter().filter(|&&f| is_ovf(f)).count() as u64;
    let geo = RunGeometry { first_slot, first_overflow: 0, slots: first_slot - 1 + nodes, v };
    let len = ((first_slot + nodes) * BLOCK) as usize;
    let mut image = vec![0xA5u8; len];
    let mut sink = CoverSink { hits: vec![0; len] };
    let mut st = BulkState::new(shape, n, geo).unwrap();
    for i in 0..n as usize {
        st.push(&mut sink, &mut image[..], &s.keys[i], s.fields[i], &s.values[i], s.contribs[i]).unwrap();
    }
    let root = st.finish(&mut sink, &mut image[..]).unwrap();
    let lo = (first_slot * BLOCK) as usize;
    assert!(sink.hits[..lo].iter().all(|&h| h == 0), "nothing before the tree's slots");
    if let Some(p) = sink.hits[lo..].iter().position(|&h| h != 1) {
        panic!("n = {n}: byte {} written {} times", lo + p, sink.hits[lo + p]);
    }
    check_tree(&image, shape, root, &s, first_slot, nodes);
    image[lo..].to_vec()
}

fn sample_ns(nl: u64, ni: u64, max: u64) -> Vec<u64> {
    let mut ns: Vec<u64> = (1..=40).collect();
    for k in 0..4u32 {
        let full = (nl + 1) * (ni + 1).pow(k);
        for d in [full - 2, full - 1, full, full + 1, full + nl / 2] {
            ns.push(d);
        }
    }
    for x in [nl - 1, nl, nl + 1, 2 * nl, 2 * nl + 1, 2 * nl + 2, 3 * nl + nl / 2, max] {
        ns.push(x);
    }
    let mut g = XorShift(nl * 7 + ni);
    for _ in 0..12 {
        ns.push(1 + g.below(max));
    }
    ns.retain(|&n| n >= 1 && n <= max);
    ns.sort();
    ns.dedup();
    ns
}

/// Every index shape's layout word is `form_code` of its (W, V).
#[test]
fn index_shape_layout_words_are_their_form_codes() {
    for w in (0..=1_100u64).step_by(4) {
        for v in [0u64, 4, 8, 16, 32, 64, 100, 128, 256, 512] {
            match IndexShape::new(w, v) {
                Some(s) => assert_eq!(Some(s.layout()), native_table::consts::form_code(w, v), "({w}, {v})"),
                None => assert_eq!(native_table::consts::form_code(w, v), None, "({w}, {v})"),
            }
        }
    }
}

#[test]
fn bulk_plan_every_byte_once() {
    for (k, &(w, v)) in CAP_SHAPES.iter().enumerate() {
        let shape = IndexShape::new(w, v).unwrap();
        let (nl, ni) = CAPS[k];
        let max = if nl == 3 { 3_000 } else { 1 << 17 };
        for n in sample_ns(nl, ni, max) {
            bulk_build(n, shape);
        }
    }
}

#[test]
fn bulk_state_refuses_a_geometry_its_plan_does_not_fit() {
    // A geometry too small for the plan, a zero first slot, and no entries.
    let shape = IndexShape::new(8, 0).unwrap();
    let geo = RunGeometry { first_slot: 1, first_overflow: 0, slots: 10, v: 0 };
    assert!(BulkState::new(shape, 500, geo).is_ok());
    assert!(BulkState::new(shape, 500, RunGeometry { slots: 2, ..geo }).is_err());
    assert!(BulkState::new(shape, 500, RunGeometry { first_slot: 0, ..geo }).is_err());
    assert!(BulkState::new(shape, 0, geo).is_err());
}

/// A main tree and its count tree in one image, each built straight through, overflow records written by
/// hand: the node slots and the overflow area equal
/// the run builder's, which builds both trees at once.
#[test]
fn bulk_plan_main_and_count_tree_in_one_image() {
    let mut g = XorShift(77);
    let mut items: Items = Vec::new();
    for k in 0..400u64 {
        let key = if k % 5 == 0 { text(&[b'x'; 30][..10 + (k % 20) as usize]) } else { k8(k * 11) };
        let n = 1 + g.below(30);
        for j in 0..n {
            items.push((key.clone(), k * 1000 + j * 2, false));
        }
    }
    let items = VecItems::sorted(items);
    for &(w, v) in &[(8u64, 16u64), (8, 0), (16, 64)] {
        let b = build(&items, &forced(w, v), 0x5A, true).unwrap();
        let h = b.plan.header;
        assert!(h.has_count_tree() && h.records > 0 && h.ct_entries == h.d);
        let lay = b.plan.layout;
        let (ents, keys) = model(&items.0, v, 0);
        let mut image = vec![0u8; lay.len as usize];
        let mut sink = native_table::tree::MemRunSink;
        let mut rec = 0u64;
        let mut key_at = lay.ok;
        let mut write_rec = |image: &mut Vec<u8>, payload: u64, key: &[u8]| -> u64 {
            let off = native_table::index::run::record_offset(lay.oa, rec).unwrap() as usize;
            image[off..off + 8].copy_from_slice(&payload.to_le_bytes());
            image[off + 8..off + 16].copy_from_slice(&key_at.to_le_bytes());
            image[off + 16..off + 24].copy_from_slice(&(key.len() as u64).to_le_bytes());
            let tail = &key[w as usize..];
            image[key_at as usize..key_at as usize + tail.len()].copy_from_slice(tail);
            key_at += tail.len() as u64;
            rec += 1;
            rec - 1
        };
        let geo = RunGeometry { first_slot: 1, first_overflow: 0, slots: lay.slots, v: lay.records };
        let mut st = BulkState::new(IndexShape::new(w, v).unwrap(), h.n, geo).unwrap();
        for e in &ents {
            let base = match e.kind {
                0 => native_table::key::probe::row_field(e.payload),
                1 => native_table::key::probe::tomb_field(e.payload),
                _ => count_field(e.payload as i64),
            };
            let field = if e.key.len() as u64 > w {
                ovf_field(base, write_rec(&mut image, payload(base), &e.key))
            } else {
                base
            };
            st.push(&mut sink, &mut image[..], &e.key, field, &e.value, e.contribution).unwrap();
        }
        assert_eq!(st.finish(&mut sink, &mut image[..]).unwrap(), BLOCK);
        let cg =
            RunGeometry { first_slot: h.nodes + 1, first_overflow: st.overflowed, slots: lay.slots, v: lay.records };
        let mut ct = BulkState::new(IndexShape::new(w, 0).unwrap(), keys.len() as u64, cg).unwrap();
        for (k, total) in &keys {
            let base = count_field(*total);
            let field =
                if k.len() as u64 > w { ovf_field(base, write_rec(&mut image, payload(base), k)) } else { base };
            ct.push(&mut sink, &mut image[..], k, field, &[], 0).unwrap();
        }
        assert_eq!(ct.finish(&mut sink, &mut image[..]).unwrap(), h.ct_root);
        assert_eq!(rec, lay.records);
        let nodes_end = ((lay.slots + 1) * BLOCK) as usize;
        assert_eq!(image[BLOCK as usize..nodes_end], b.image[BLOCK as usize..nodes_end], "({w}, {v}) node slots");
        // Records and key bytes (the builder zeroes the gaps; so did the zero-filled image here).
        let kend = (lay.ok + lay.key_bytes) as usize;
        assert_eq!(image[lay.oa as usize..kend], b.image[lay.oa as usize..kend], "({w}, {v}) overflow area");
        assert_eq!(lay.oa, (nodes_end as u64).next_multiple_of(PAGE));
        assert_eq!(lay.ok, lay.oa + lay.records.div_ceil(2_730) * PAGE);
    }
}

// ---------------------------------------------------------------- chunk_codec

#[test]
fn chunk_codec_round_trips_every_v() {
    let mut g = XorShift(0x9E37_79B9_7F4A_7C15);
    for &v in &V_CHOICES[1..] {
        for gapmax in [0u64, 1, 3, 300, 1 << 20, 1 << 40, 1 << 48] {
            let mut rid = g.below(1000);
            let mut rids = Vec::new();
            for _ in 0..5_000 {
                rid += g.below(gapmax + 1);
                rids.push(rid);
            }
            assert!(rid <= RID_PAYLOAD);
            for &tomb in &[false, true] {
                let mut ch = Chunker::new(v);
                let mut chunks: Vec<Vec<u64>> = Vec::new();
                for &r in &rids {
                    if ch.offer(r, tomb) {
                        chunks.last_mut().unwrap().push(r);
                    } else {
                        chunks.push(vec![r]);
                    }
                }
                let mut decoded = Vec::new();
                for c in &chunks {
                    assert!(c.len() as u64 <= CHUNK_RIDS_MAX);
                    let mut out = vec![0xA5u8; v as usize];
                    chunk::encode_posting(c, &mut out).unwrap();
                    assert_eq!(out, model_posting(c, v as usize), "v = {v}");
                    decoded.extend(posting_rids(&out, *c.last().unwrap()));
                }
                assert_eq!(decoded, rids, "v = {v}, gapmax = {gapmax}");
                // Greedy: a chunk is closed only when the next rid does not fit it.
                for pair in chunks.windows(2) {
                    let mut grown = pair[0].clone();
                    grown.push(pair[1][0]);
                    let mut out = vec![0u8; v as usize];
                    assert!(grown.len() as u64 > CHUNK_RIDS_MAX || chunk::encode_posting(&grown, &mut out).is_err());
                }
            }
        }
    }
    // Codes of 57 to 61 bits, whose packing crosses 64 bits past a partial byte: the model's bytes.
    for b in 57u32..=61 {
        for lead in 0..8u64 {
            let mut rids = vec![g.below(1000)];
            let mut gaps = vec![lead];
            while rids.len() < 12 {
                let gap = (1u64 << (b - 1)) | (g.next_u64() & ((1u64 << (b - 1)) - 1)) >> g.below(4);
                if rids.last().unwrap() + gaps.iter().sum::<u64>() + gap > RID_PAYLOAD {
                    break;
                }
                gaps.push(gap);
            }
            for &gap in &gaps {
                rids.push(rids.last().unwrap() + gap);
            }
            for &v in &V_CHOICES[1..] {
                let mut out = vec![0xA5u8; v as usize];
                let max_b = rids.windows(2).map(|w| 64 - (w[1] - w[0]).leading_zeros() as u64).max().unwrap();
                if 3 + ((rids.len() as u64 - 1) * max_b).div_ceil(8) > v {
                    assert!(chunk::encode_posting(&rids, &mut out).is_err());
                    continue;
                }
                chunk::encode_posting(&rids, &mut out).unwrap();
                assert_eq!(out, model_posting(&rids, v as usize), "b = {b}, v = {v}");
                assert_eq!(posting_rids(&out, *rids.last().unwrap()), rids);
            }
        }
    }
    // Tombstones and rows never share a chunk.
    let mut ch = Chunker::new(256);
    assert!(!ch.offer(5, false));
    assert!(ch.offer(6, false));
    assert!(!ch.offer(7, true));
    assert!(ch.offer(8, true));
}

#[test]
fn chunk_codec_corrupt_headers_fault_never_panic() {
    let mut g = XorShift(12345);
    let mut corrupt_seen = 0u64;
    let mut valid_seen = 0u64;
    while corrupt_seen < 100_000 {
        let v = V_CHOICES[1 + g.below(6) as usize] as usize;
        let mut value: Vec<u8> = (0..v).map(|_| g.next_u64() as u8).collect();
        value[0] = match g.below(4) {
            0 | 1 => 0,
            2 => 1,
            _ => g.next_u64() as u8,
        };
        // Half the headers near the limits: small m and b, b around 62, L around 1..=8.
        if g.below(2) == 0 {
            if value[0] == 0 {
                value[1] = g.below(2 * v as u64) as u8;
                value[2] = if g.below(4) == 0 { 58 + g.below(8) as u8 } else { g.below(16) as u8 };
            } else {
                value[1] = g.below(11) as u8;
            }
        }
        let model_ok = match value[0] {
            0 => value[2] <= 62 && 3 + (value[1] as u64 * value[2] as u64).div_ceil(8) <= v as u64,
            1 => (1..=8).contains(&value[1]),
            _ => false,
        };
        match chunk::header(&value, RUN_ID, 42) {
            Ok(h) => {
                assert!(model_ok, "accepted {:?}", &value[..3]);
                valid_seen += 1;
                if let Chunk::Posting { m, b } = h {
                    for k in 1..=m {
                        chunk::code(&value, m, b, k, RUN_ID, 42).unwrap();
                    }
                    match chunk::first(&value, m, b, g.next_u64() >> g.below(64), RUN_ID, 42) {
                        Ok(_) => {}
                        Err(f) => {
                            assert!(matches!(f, Fault::Corrupt(CorruptAt { kind: Corruption::Range, offset: 42, .. })))
                        }
                    }
                    assert!(chunk::code(&value, m, b, m.wrapping_add(1), RUN_ID, 42).is_err() || m == 255);
                    assert!(chunk::code(&value, m, b, 0, RUN_ID, 42).is_err());
                }
            }
            Err(f) => {
                assert!(!model_ok, "refused {:?}", &value[..3]);
                assert!(is_run_fault(&f, RUN_ID));
                corrupt_seen += 1;
            }
        }
    }
    assert!(valid_seen > 5_000, "{valid_seen} valid headers");
    // Rids made absolute stay below 2^62.
    assert!(chunk::abs_rid(1 << 61, (1 << 61) - 1, RUN_ID, 0).is_ok());
    assert!(chunk::abs_rid(1 << 61, 1 << 61, RUN_ID, 0).is_err());
    assert!(chunk::abs_rid(u64::MAX, 1, RUN_ID, 0).is_err());
}

/// Items of a key-run index: keys = prefix ‖ an l-byte big-endian field; stretches of consecutive keys and
/// rids, broken at random, with some keys of several items between.
fn key_run_items(g: &mut XorShift, prefix: &[u8], l: usize, n: usize) -> Items {
    let max = if l == 8 { u64::MAX } else { (1u64 << (8 * l)) - 1 };
    let mut items = Vec::new();
    let mut f = g.below(5);
    let mut rid = 100;
    let key = |f: u64| {
        let mut k = prefix.to_vec();
        k.extend_from_slice(&f.to_be_bytes()[8 - l..]);
        k
    };
    while items.len() < n && f < max - 3 {
        match g.below(12) {
            0 => {
                f += 1 + g.below(3);
            }
            1 => rid += 1 + g.below(3),
            2 => {
                for j in 0..3 {
                    items.push((key(f), rid + j, false));
                }
                rid += 3;
                f += 1;
            }
            _ => {
                items.push((key(f), rid, false));
                rid += 1;
                f += 1;
            }
        }
    }
    items.sort();
    items
}

#[test]
fn chunk_codec_key_runs_at_every_l() {
    let mut g = XorShift(99);
    for l in 1..=8usize {
        for (prefix, w, v) in [(b"pfx".to_vec(), 16u64, 8u64), (b"pfx".to_vec(), 16, 32), (vec![0x55; 20], 8, 16)] {
            let n = if l == 1 { 200 } else { 3_000 };
            let items = VecItems(key_run_items(&mut g, &prefix, l, n));
            let p = RunParams { key_run_l: l as u8, ..forced(w, v) };
            let b = build(&items, &p, 0xA5, true).unwrap();
            let run = b.open();
            let (ents, keys) = model(&items.0, v, l);
            assert!(ents.iter().any(|e| e.value.first() == Some(&1)), "key runs formed at l = {l}");
            assert_eq!(b.plan.header.key_run_l as usize, l);
            assert!(b.plan.header.flags & native_table::index::run::F_KEY_RUNS != 0);
            let want: Vec<Got> = ents.iter().map(|m| got_of(m, w, run.hdr.prefix as usize)).collect();
            assert_eq!(scan::<Fwd>(&run, Unbounded, Unbounded), want, "l = {l}, w = {w}");
            // Every key-run item decodes back to its key and rid.
            for e in ents.iter().filter(|e| e.value.first() == Some(&1)) {
                let Chunk::KeyRun { l: ll, n_minus_1 } = chunk::header(&e.value, RUN_ID, 0).unwrap() else { panic!() };
                assert_eq!(ll as usize, l);
                assert_eq!(n_minus_1 as usize + 1, e.items.len());
                let lf = chunk::key_field(&e.key, ll).unwrap();
                for (k, (key, rid)) in e.items.iter().enumerate() {
                    let (f, r) = chunk::key_run_item(lf, e.payload, n_minus_1, k as u32, RUN_ID, 0).unwrap();
                    assert_eq!(f, chunk::key_field(key, ll).unwrap());
                    assert_eq!(r, *rid);
                }
            }
            // Counts: every key of a key run counts 1; absent keys 0.
            let klen = (prefix.len() + l) as u64;
            for (k, total) in keys.iter().step_by(7) {
                assert_eq!(run.count_key(k).unwrap(), *total, "count of {k:?}");
            }
            let mut absent = prefix.clone();
            absent.extend_from_slice(&vec![0xFF; l]);
            assert_eq!(run.count_key(&absent).unwrap(), 0);
            // Keys in both directions.
            let listed = list_keys(&run, Dir::Fwd, klen);
            let want_keys: Vec<(Vec<u8>, i64)> = keys.iter().map(|(k, t)| (stored(k, w), *t)).collect();
            assert_eq!(listed, want_keys, "l = {l}");
            let mut back = list_keys(&run, Dir::Rev, klen);
            back.reverse();
            assert_eq!(back, want_keys);
        }
    }
}

#[test]
fn chunk_codec_tombstones_never_key_runs() {
    let items: Items = (0..500u64).map(|k| (k8(k), 1000 + k, true)).collect();
    let items = VecItems::sorted(items);
    let p = RunParams { key_run_l: 8, ..forced(8, 16) };
    let b = build(&items, &p, 0x5A, true).unwrap();
    let run = b.open();
    let got = scan::<Fwd>(&run, Unbounded, Unbounded);
    assert_eq!(got.len(), 500);
    assert!(got.iter().all(|e| e.kind == 1 && e.value[0] == 0 && e.contribution == -1));
    assert_eq!(b.plan.header.flags & native_table::index::run::F_KEY_RUNS, 0);
    // The same keys as rows become one key run.
    let rows = VecItems(items.0.iter().map(|(k, r, _)| (k.clone(), *r, false)).collect());
    let b = build(&rows, &p, 0x5A, true).unwrap();
    let got = scan::<Fwd>(&b.open(), Unbounded, Unbounded);
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].value[0], got[0].contribution), (1, 500));
}

/// Every distinct key of a run in direction `dir`, with its total, through `key_step`.
fn list_keys(run: &Run<MemImage<'_>>, dir: Dir, klen: u64) -> Vec<(Vec<u8>, i64)> {
    let mut out: Vec<(Vec<u8>, i64)> = Vec::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let after = out.last().map(|(k, _)| k.clone());
        match run.key_step(after.as_deref(), dir, klen, &mut buf).unwrap() {
            None => return out,
            Some((n, t)) => out.push((buf[..n].to_vec(), t)),
        }
    }
}

// ---------------------------------------------------------------- choose_v and widths: the model of the choice

/// The bulk plan's node count, written out again.
fn model_nodes(n: u64, nl: u64, ni: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut c = (n + 1).div_ceil(nl + 1);
    let mut t = c;
    while c > 1 {
        c = c.div_ceil(ni + 1);
        t += c;
    }
    t
}

/// The choice of (W, V) by its stated rule, from the model's entries.
fn model_choice(items: &Items, l: usize, w0: u64) -> (u64, u64) {
    let mut hist = [0u64; 128];
    // The keys' lengths past the run's shared prefix.
    let p = nt_testhost::runs::run_prefix(items, l) as u64;
    for (k, _, _) in items {
        let len = k.len() as u64 - p;
        let b = if len <= 8 { 0 } else { ((len - 1) / 8).min(127) as usize };
        hist[b] += 1;
    }
    let total: u64 = hist.iter().sum();
    let holding = |p: u64| {
        let mut cum = 0;
        for (k, &x) in hist.iter().enumerate() {
            cum += x;
            if cum * 100 >= total * p {
                return ((k as u64 + 1) * 8).min(W_MAX);
            }
        }
        W_MAX
    };
    let base = w0.max(8);
    let w64 = if holding(100) > 64 { 64 } else { base };
    let widths = [base, holding(50), holding(90), holding(99), holding(100), w64].map(|w| w.max(base));
    let (_, keys) = model(items, 0, l);
    let d = keys.len() as u64;
    let n_items = items.len() as u64;
    let entries: Vec<(u64, u64)> = V_CHOICES.iter().map(|&v| (v, model(items, v, l).0.len() as u64)).collect();
    let mut best: Option<(u64, u64, u64)> = None;
    for &w in &widths {
        for &(v, e) in &entries {
            if w + 8 + v > 1024 {
                continue;
            }
            let s = IndexShape::new(w, v).unwrap();
            let nodes = model_nodes(e, s.leaf.cap, s.internal.cap);
            let cs = IndexShape::new(w, 0).unwrap();
            let ct = if 2 * d <= n_items { model_nodes(d, cs.leaf.cap, cs.internal.cap) } else { 0 };
            let fb = if nodes > 1 { ((d * 10).div_ceil(512).max(64).div_ceil(64) * 64) / 64 } else { 0 };
            let mut ovf = 0u64;
            for (k, &x) in hist.iter().enumerate() {
                let len = (k as u64 + 1) * 8;
                if len > w {
                    ovf += x * (24 + len - w);
                }
            }
            let bytes = 4096 * (1 + nodes + ct) + ovf.next_multiple_of(4096) + fb * 4096;
            let better = match best {
                None => true,
                Some((bb, bw, bv)) => bytes < bb || (bytes == bb && (w > bw || (w == bw && v < bv))),
            };
            if better {
                best = Some((bytes, w, v));
            }
        }
    }
    let (_, w, v) = best.unwrap();
    (w, v)
}

/// The shapes of the density model, scaled down: (name, items, key-run field length).
fn den_shapes() -> Vec<(&'static str, Items, usize)> {
    let mut g = XorShift(20261013);
    let mut shapes = Vec::new();
    let uniform = |g: &mut XorShift, keys: u64, per: u64, span: u64| -> Items {
        let mut items = Vec::new();
        for k in 0..keys {
            let mut rids: Vec<u64> = (0..per).map(|_| g.below(span)).collect();
            rids.sort();
            rids.dedup();
            for r in rids {
                items.push((k8(k * 7919), r, false));
            }
        }
        items.sort();
        items
    };
    let mut perm: Vec<u64> = (0..50_000).collect();
    for i in (1..perm.len()).rev() {
        perm.swap(i, g.below(i as u64 + 1) as usize);
    }
    let mut unique: Items = perm.iter().enumerate().map(|(i, &r)| (k8(i as u64 * 13), r, false)).collect();
    unique.sort();
    shapes.push(("unique u64 keys", unique, 0));
    shapes.push(("positional, key runs", (0..50_000u64).map(|i| (k8(i), i, false)).collect(), 8));
    for rpk in [2u64, 10, 100] {
        shapes.push(("rows per key, span 10^8", uniform(&mut g, 20_000 / rpk.min(20), rpk, 100_000_000), 0));
    }
    for (p, win) in [(3u64, 200_000u64), (2, 200_000), (10, 400_000)] {
        let mut items = Vec::new();
        for r in 0..win {
            if g.below(p) == 0 {
                items.push((k8(42), r, false));
            }
        }
        shapes.push(("one value of a few", items, 0));
    }
    let clustered: Items = (0..30u64).flat_map(|k| (0..3_000u64).map(move |i| (k8(k), k * 3_000 + i, false))).collect();
    shapes.push(("clustered, 3,000 consecutive rows per key", clustered, 0));
    let mut texts: Items = (0..20_000u64).map(|i| (text(format!("user-{:014}", i * 7).as_bytes()), i, false)).collect();
    texts.sort();
    shapes.push(("unique text keys of 20 bytes", texts, 0));
    let mut zipf = Vec::new();
    for k in 0..3_000u64 {
        let c = (30_000 / (k + 1)).max(1);
        let mut rids: Vec<u64> = (0..c).map(|_| g.below(100_000_000)).collect();
        rids.sort();
        rids.dedup();
        for r in rids {
            zipf.push((k8(k), r, false));
        }
    }
    shapes.push(("zipf over 3,000 keys", zipf, 0));
    shapes
}

#[test]
fn choose_v_by_shape() {
    // Unique keys: V = 0.
    let unique: Items = (0..20_000u64).map(|k| (k8(k * 3), k * 7 + 3, false)).collect();
    let p = plan_run(&VecItems(unique), &params(1)).unwrap();
    assert_eq!(p.header.v, 0);
    // One key holding about every third rid: a wide value part.
    let mut g = XorShift(7);
    let mut r = 0u64;
    let dense: Items = (0..200_000)
        .map(|_| {
            r += 1 + g.below(5);
            (k8(1), r, false)
        })
        .collect();
    let p = plan_run(&VecItems(dense), &params(1)).unwrap();
    assert!(p.header.v >= 64, "dense chose {}", p.header.v);
    // Consecutive rids per key: V ≥ 8.
    let clustered: Items =
        (0..100u64).flat_map(|k| (0..3_000u64).map(move |i| (k8(k), k * 3_000 + i, false))).collect();
    let p = plan_run(&VecItems(clustered), &params(1)).unwrap();
    assert!(p.header.v >= 8);
    assert!(p.layout.len < 3_000 * 100 / 2);
    // The density model's shapes: the builder's choice equals the model's.
    for (name, items, l) in den_shapes() {
        let items = VecItems::sorted(items);
        let pr = RunParams { key_run_l: l as u8, ..params(1) };
        let p = plan_run(&items, &pr).unwrap();
        assert_eq!((p.header.w, p.header.v), model_choice(&items.0, l, 0), "{name}");
        let b = build(&items, &pr, 0xA5, true).unwrap();
        let run = b.open();
        let t = run.main().unwrap();
        assert_eq!(t.shape().v, p.header.v);
    }
}

#[test]
fn widths_follow_the_keys() {
    let mut g = XorShift(4242);
    let mixes: [(usize, usize, usize, u64); 5] =
        [(1, 6, 4_000, 3), (10, 30, 3_000, 2), (40, 140, 2_000, 4), (3, 300, 1_500, 6), (900, 1_400, 150, 9)];
    for (lo, hi, keys, per) in mixes {
        let items = VecItems(random_items(&mut g, keys, per, 1 << 40, lo, hi, true));
        for w0 in [0u64, 16, 64] {
            let p = RunParams { w0, ..params(2) };
            let plan = plan_run(&items, &p).unwrap();
            let (w, v) = (plan.header.w, plan.header.v);
            assert!(w >= w0.max(8) && w % 8 == 0 && w + 8 + v <= 1024);
            assert_eq!((w, v), model_choice(&items.0, 0, w0), "mix {lo}..{hi}, w0 {w0}");
            let b = build(&items, &p, 0x5A, true).unwrap();
            let run = b.open();
            let (ents, keys) = model(&items.0, v, 0);
            for (k, total) in keys.iter().step_by(3) {
                assert_eq!(run.count_key(k).unwrap(), *total);
            }
            let want: Vec<Got> = ents.iter().map(|m| got_of(m, w, run.hdr.prefix as usize)).collect();
            assert_eq!(scan::<Fwd>(&run, Unbounded, Unbounded), want);
        }
    }
    // The measure's histogram buckets: bucket k holds the keys whose smallest width is 8(k + 1).
    for (len, b) in
        [(0u64, 0usize), (1, 0), (8, 0), (9, 1), (16, 1), (17, 2), (1_016, 126), (1_017, 127), (99_999, 127)]
    {
        assert_eq!(native_table::index::plan::bucket(len), b);
    }
    assert_eq!(native_table::index::plan::bucket_len(126), 1_016);
    // The measure alone: one key of 300 bytes among short ones overflows at every candidate width but 1,016.
    let mut m = native_table::index::plan::Measure::new();
    for i in 0..99u64 {
        m.begin_key();
        m.item(5, i, false).unwrap();
        m.end_key().unwrap();
    }
    m.begin_key();
    m.item(300, 500, false).unwrap();
    m.end_key().unwrap();
    assert_eq!(m.candidate_widths(0), [8, 8, 8, 8, 304, 64]);
    assert_eq!(m.candidate_widths(24), [24, 24, 24, 24, 304, 64]);
    assert_eq!(m.candidate_widths(128), [128, 128, 128, 128, 304, 128]);
    // No key longer than 64 bytes: no candidate of 64.
    let mut m = native_table::index::plan::Measure::new();
    m.begin_key();
    m.item(64, 1, false).unwrap();
    m.end_key().unwrap();
    assert_eq!(m.candidate_widths(0), [8, 64, 64, 64, 64, 8]);
}

// ---------------------------------------------------------------- reading a run through the tree

fn probe_of(m: &MEnt) -> KeyProbe<'_> {
    KeyProbe { key: &m.key, kind: if m.kind == 2 { Kind::Count } else { Kind::Row(m.payload) } }
}

#[test]
fn runs_read_through_find_and_cursors_equal_a_model() {
    let mut g = XorShift(31337);
    let configs: [(usize, u64, usize, usize, Option<(u64, u64)>); 6] = [
        (600, 12, 1, 12, None),
        (300, 40, 4, 40, Some((8, 16))),
        (200, 30, 20, 120, Some((16, 128))),
        (2_000, 3, 1, 8, Some((8, 0))),
        (150, 300, 6, 20, Some((8, 256))),
        (40, 10, 1_000, 2_500, None),
    ];
    for (keys, per, lo, hi, f) in configs {
        let items = VecItems(random_items(&mut g, keys, per, 1 << 30, lo, hi, true));
        let p = match f {
            Some((w, v)) => forced(w, v),
            None => params(1),
        };
        let mut images = Vec::new();
        for &fill in &FILLS {
            let b = build(&items, &p, fill, true).unwrap();
            let (w, v) = (b.plan.header.w, b.plan.header.v);
            let run = b.open();
            let (ents, _) = model(&items.0, v, 0);
            let want: Vec<Got> = ents.iter().map(|m| got_of(m, w, run.hdr.prefix as usize)).collect();
            assert_eq!(scan::<Fwd>(&run, Unbounded, Unbounded), want);
            let mut back = scan::<Rev>(&run, Unbounded, Unbounded);
            back.reverse();
            assert_eq!(back, want);
            // Each row entry and count entry is found by its own probe.
            let t = run.main().unwrap();
            for (m, wnt) in ents.iter().zip(&want).step_by(3) {
                if m.kind == 1 {
                    continue;
                }
                let (n, i) = t.find(&stored_probe(&run, &probe_of(m)).unwrap()).unwrap().expect("found");
                assert_eq!(&decode(&run, n, i), wnt);
            }
            // Items decode back from the chunks.
            let mut items_back: Vec<(Vec<u8>, u64, bool)> = Vec::new();
            for (m, gt) in ents.iter().zip(&want) {
                if m.kind == 2 {
                    continue;
                }
                let rids = if v == 0 { vec![gt.payload] } else { posting_rids(&gt.value, gt.payload) };
                for r in rids {
                    items_back.push((m.key.clone(), r, m.kind == 1));
                }
            }
            assert_eq!(items_back, items.0);
            // Random bounds in both directions.
            for _ in 0..60 {
                let pick = |g: &mut XorShift| -> (Vec<u8>, Kind) {
                    let e = &ents[g.below(ents.len() as u64) as usize];
                    let mut key = e.key.clone();
                    if g.below(4) == 0 {
                        *key.last_mut().unwrap() ^= 1;
                    }
                    let kind = match g.below(3) {
                        0 => Kind::Low,
                        1 => Kind::Count,
                        _ => Kind::Row(e.payload.wrapping_add(g.below(3)).wrapping_sub(1)),
                    };
                    (key, kind)
                };
                let (ka, kinda) = pick(&mut g);
                let (kb, kindb) = pick(&mut g);
                let pa = KeyProbe { key: &ka, kind: kinda };
                let pb = KeyProbe { key: &kb, kind: kindb };
                let bound = |g: &mut XorShift, p| match g.below(3) {
                    0 => Unbounded,
                    1 => Included(p),
                    _ => Excluded(p),
                };
                let lo = bound(&mut g, &pa);
                let hi = bound(&mut g, &pb);
                let model_fwd: Vec<Got> =
                    ents.iter().zip(&want).filter(|(m, _)| in_range(m, lo, hi)).map(|(_, x)| x.clone()).collect();
                assert_eq!(scan::<Fwd>(&run, lo, hi), model_fwd);
                // Reverse from the upper bound down to the lower one.
                let rev: Vec<Got> = scan::<Rev>(&run, hi, lo);
                let model_rev: Vec<Got> =
                    ents.iter().zip(&want).rev().filter(|(m, _)| in_range(m, lo, hi)).map(|(_, x)| x.clone()).collect();
                assert_eq!(rev, model_rev);
            }
            images.push(b.image);
        }
        assert_eq!(images[0], images[1], "the same input gives the same image under both fill patterns");
    }
}

// ---------------------------------------------------------------- prefix sums and select_rank

#[test]
fn prefix_sums_and_select_rank_equal_a_model() {
    let mut g = XorShift(555);
    for (keys, per, tombs, f) in [
        (3_000usize, 1u64, false, Some((8, 0))),
        (800, 30, false, Some((8, 16))),
        (500, 200, false, Some((8, 128))),
        (400, 25, true, Some((8, 32))),
        (2_000, 4, true, Some((8, 0))),
        (300, 50, false, None),
    ] {
        let items = VecItems(random_items(&mut g, keys, per, 1 << 35, 2, 30, tombs));
        let p = match f {
            Some((w, v)) => forced(w, v),
            None => params(1),
        };
        let b = build(&items, &p, 0xA5, true).unwrap();
        let run = b.open();
        let (ents, keys_t) = model(&items.0, b.plan.header.v, 0);
        // prefix(E) counts every entry whose key is below E.
        let mut probes: Vec<Vec<u8>> = keys_t.iter().map(|(k, _)| k.clone()).collect();
        for _ in 0..200 {
            let mut k = probes[g.below(probes.len() as u64) as usize].clone();
            *k.last_mut().unwrap() = k.last().unwrap().wrapping_add(1);
            probes.push(k);
        }
        probes.push(vec![]);
        probes.push(vec![0xFF; 40]);
        for k in probes.iter().step_by(1 + probes.len() / 400) {
            let want: i64 = ents.iter().filter(|e| e.key.as_slice() < k.as_slice()).map(|e| e.contribution).sum();
            assert_eq!(run.prefix(k).unwrap(), want, "prefix of {k:?}");
        }
        let total: i64 = ents.iter().map(|e| e.contribution).sum();
        assert_eq!(run.hdr.total, total);
        if tombs {
            continue;
        }
        // select_rank: the entry whose contributions span the rank, with what lies before it.
        let mut spans = Vec::new();
        let mut acc = 0i64;
        for (i, e) in ents.iter().enumerate() {
            if e.contribution > 0 {
                spans.push((acc, acc + e.contribution, i));
            }
            acc += e.contribution;
        }
        let w = b.plan.header.w;
        let mut finger = Finger::default();
        let mut ranks: Vec<i64> = (0..300).map(|_| g.below(total as u64) as i64).collect();
        ranks.extend([0, total - 1]);
        let mut sorted = ranks.clone();
        sorted.sort();
        for rank in ranks.iter().chain(sorted.iter()) {
            let pos = run.select_rank(*rank, &mut finger).unwrap().expect("in range");
            let s = spans.iter().find(|s| s.0 <= *rank && *rank < s.1).unwrap();
            assert_eq!(pos.before, s.0, "rank {rank}");
            assert_eq!(decode(&run, pos.node, pos.index), got_of(&ents[s.2], w, run.hdr.prefix as usize));
        }
        assert_eq!(run.select_rank(total, &mut finger).unwrap(), None);
        assert_eq!(run.select_rank(-1, &mut finger).unwrap(), None);
    }
}

// ---------------------------------------------------------------- count_trees

#[test]
fn count_trees_and_counts_equal_a_model() {
    let mut g = XorShift(808);
    for (keys, per, tombs, f, null_keys) in [
        (300usize, 40u64, false, None, true),
        (300, 40, true, Some((8, 16)), true),
        (2_000, 1, false, Some((8, 0)), false),
        (2_000, 2, true, Some((16, 8)), true),
        (150, 60, false, Some((8, 64)), false),
        (100, 20, true, Some((8, 0)), true),
    ] {
        let mut raw = random_items(&mut g, keys, per, 1 << 33, 1, 40, tombs);
        if null_keys {
            // Keys starting with the null tag 00 are skipped by the minimum and maximum.
            for (k, _, _) in raw.iter_mut().take(30) {
                k.insert(0, 0);
            }
            raw.sort();
        }
        let items = VecItems(raw);
        let p = match f {
            Some((w, v)) => forced(w, v),
            None => params(1),
        };
        let b = build(&items, &p, 0x5A, true).unwrap();
        let h = b.plan.header;
        let (_, keys_t) = model(&items.0, h.v, 0);
        assert_eq!(h.has_count_tree(), 2 * keys_t.len() <= items.0.len());
        assert_eq!(h.d, keys_t.len() as u64);
        let run = b.open();
        for (k, t) in &keys_t {
            assert_eq!(run.count_key(k).unwrap(), *t);
        }
        for _ in 0..200 {
            let mut k = keys_t[g.below(keys_t.len() as u64) as usize].0.clone();
            k.push(0x77);
            assert_eq!(run.count_key(&k).unwrap(), 0);
        }
        // Group listing in both directions, and the extremes.
        let want: Vec<(Vec<u8>, i64)> = keys_t.iter().map(|(k, t)| (stored(k, h.w), *t)).collect();
        assert_eq!(list_keys(&run, Dir::Fwd, 0), want);
        let mut back = list_keys(&run, Dir::Rev, 0);
        back.reverse();
        assert_eq!(back, want);
        let live: Vec<&(Vec<u8>, i64)> = want.iter().filter(|(k, t)| *t > 0 && k[0] != 0).collect();
        let mut scratch = vec![0u8; 1 << 17];
        for largest in [false, true] {
            let got = run.extreme_key(largest, 0, &mut scratch).unwrap();
            let m = if largest { live.last() } else { live.first() };
            match (got, m) {
                (Some((n, t)), Some((k, mt))) => {
                    assert_eq!(&scratch[..n], &k[..]);
                    assert_eq!(t, *mt);
                }
                (None, None) => {}
                (a, b) => panic!("extreme {a:?} against {b:?}"),
            }
        }
        // The bytes are a function of the entries.
        let again = build(&items, &p, 0xA5, true).unwrap();
        assert_eq!(again.image, b.image);
    }
    // The count-tree rule's boundary: 2d = items has one, 2d = items + 1 has none.
    let with: Items = (0..50u64).flat_map(|k| [(k8(k), 2 * k, false), (k8(k), 2 * k + 1, false)]).collect();
    assert!(plan_run(&VecItems(with.clone()), &params(1)).unwrap().header.has_count_tree());
    let mut without = with;
    without.push((k8(99), 500, false));
    assert!(!plan_run(&VecItems(without), &params(1)).unwrap().header.has_count_tree());
}

#[test]
fn count_trees_believed_totals_are_served() {
    let items: Items = (0..200u64).flat_map(|k| (0..10).map(move |j| (k8(k), k * 100 + j, false))).collect();
    let items = VecItems(items);
    let b = build(&items, &forced(8, 16), 0xA5, true).unwrap();
    assert!(b.plan.header.has_count_tree());
    let mut image = b.image.clone();
    // Rewrite every count-tree entry's total as the key's number + 1000.
    let (shape, nodes) = {
        let run = b.open();
        let ct = run.count_tree().unwrap().unwrap();
        (ct.shape(), b.plan.header.ct_nodes)
    };
    for s in 0..nodes {
        let base = ((b.plan.header.nodes + 1 + s) * BLOCK) as usize;
        let count = u16::from_le_bytes([image[base], image[base + 1]]) as usize;
        for i in 0..count {
            let ko = base + 8 + i * (shape.w as usize + 8);
            let key = u64::from_be_bytes(image[ko..ko + 8].try_into().unwrap());
            let f = count_field(key as i64 + 1000);
            image[ko + 8..ko + 16].copy_from_slice(&f.to_be_bytes());
        }
    }
    let run = Run::open(MemImage { bytes: &image, id: RUN_ID }, 0, 0, u64::MAX).unwrap();
    for k in 0..200u64 {
        assert_eq!(run.count_key(&k8(k)).unwrap(), k as i64 + 1000);
    }
    let listed = list_keys(&run, Dir::Fwd, 0);
    assert_eq!(listed.len(), 200);
    assert!(listed.iter().enumerate().all(|(k, (_, t))| *t == k as i64 + 1000));
    // A count-tree entry that is not a count entry faults, never panics.
    let base = ((b.plan.header.nodes + 1) * BLOCK) as usize;
    image[base + 8 + 8..base + 16 + 8].copy_from_slice(&5u64.to_be_bytes());
    let run = Run::open(MemImage { bytes: &image, id: RUN_ID }, 0, 0, u64::MAX).unwrap();
    let mut buf = vec![0u8; 4096];
    let mut after: Option<Vec<u8>> = None;
    let mut faults = 0;
    for _ in 0..400 {
        match run.key_step(after.as_deref(), Dir::Fwd, 0, &mut buf) {
            Ok(Some((n, _))) => after = Some(buf[..n].to_vec()),
            Ok(None) => break,
            Err(f) => {
                assert!(is_run_fault(&f, RUN_ID));
                faults += 1;
                break;
            }
        }
    }
    assert!(faults <= 1);
}

// ---------------------------------------------------------------- deterministic bytes

#[test]
fn deterministic_bytes_under_both_fill_patterns() {
    let mut g = XorShift(2024);
    let cases: Vec<(Items, RunParams)> = vec![
        (random_items(&mut g, 500, 30, 1 << 40, 4, 60, true), params(1)),
        (random_items(&mut g, 3_000, 1, 1 << 20, 4, 9, false), params(0)),
        ((0..20_000u64).map(|i| (k8(i), i, false)).collect(), RunParams { key_run_l: 8, ..params(0) }),
        (random_items(&mut g, 60, 20, 1 << 40, 1_100, 3_000, false), forced(1_016, 0)),
    ];
    for (items, p) in cases {
        let items = VecItems(items);
        let a = build(&items, &p, FILLS[0], true).unwrap();
        let b = build(&items, &p, FILLS[1], false).unwrap();
        assert_eq!(a.image, b.image);
        assert_eq!(a.image.len() as u64 % BLOCK, 0);
        // The header check accepts what the builder wrote.
        let h = a.plan.header;
        let x = RunExpect {
            table_uid: h.table_uid,
            schema_hash: h.schema_hash,
            index: h.index,
            key_spec_hash: h.key_spec_hash,
            w: h.w,
            key_run_l: if h.key_run_l != 0 { h.key_run_l } else { p.key_run_l },
        };
        assert_eq!(check_header(&a.image[..4096], 3, &x).unwrap(), h);
        let mut sc = StructureCheck::new();
        let img = a.image(1);
        while sc.step(&img, &h, 3, 50).unwrap() == Progress::More {}
    }
}

// ---------------------------------------------------------------- believed runs (without merges)

fn sample_run() -> Built {
    let mut g = XorShift(9);
    let mut items = random_items(&mut g, 400, 30, 1 << 30, 4, 30, true);
    // Long keys: overflow records.
    for k in 0..30u64 {
        let key = text(&vec![b'q'; 20 + k as usize]);
        for j in 0..3 {
            items.push((key.clone(), k * 10 + j, false));
        }
    }
    items.sort();
    build(&VecItems(items), &forced(16, 32), 0xA5, true).unwrap()
}

fn expect_of(b: &Built) -> RunExpect {
    let h = b.plan.header;
    RunExpect {
        table_uid: h.table_uid,
        schema_hash: h.schema_hash,
        index: h.index,
        key_spec_hash: h.key_spec_hash,
        w: h.w,
        key_run_l: 0,
    }
}

fn check_all(image: &[u8], h: &native_table::index::RunHeader) -> Result<(), native_table::Refusal> {
    let img = MemImage { bytes: image, id: RUN_ID };
    let mut sc = StructureCheck::new();
    while sc.step(&img, h, 7, 64)? == Progress::More {}
    Ok(())
}

#[test]
fn believed_runs_structure_check_refuses_bad_structure() {
    let b = sample_run();
    let h = b.plan.header;
    assert!(h.nodes > 3 && h.records > 0);
    check_all(&b.image, &h).unwrap();
    let shape = IndexShape::new(h.w, h.v).unwrap();
    let root = BLOCK as usize;
    // Find a leaf slot and an internal slot.
    let leaf_slot = (1..=h.nodes).find(|&s| b.image[(s * BLOCK) as usize + 2] == 3).unwrap();
    let ls = (leaf_slot * BLOCK) as usize;
    let refused = |image: &[u8], what: &str| -> u64 {
        let r = check_all(image, &h).expect_err(what);
        assert_eq!(r.code, RefusalCode::BadStructure);
        assert_eq!(r.image, 7);
        r.field
    };
    let mut x = b.image.clone();
    x[ls + 2] = 1;
    assert_eq!(refused(&x, "a bad node flag"), ls as u64 + 2, "a bad node flag");
    let mut x = b.image.clone();
    x[ls + 4] ^= 0x10;
    assert_eq!(refused(&x, "a bad layout word"), ls as u64 + 4, "a bad layout word");
    let mut x = b.image.clone();
    x[ls..ls + 2].copy_from_slice(&((shape.leaf.cap + 1) as u16).to_le_bytes());
    assert_eq!(refused(&x, "an entry count above the capacity"), ls as u64, "an entry count above the capacity");
    let mut x = b.image.clone();
    let coff = shape.internal.coff as usize;
    x[root + coff..root + coff + 8].copy_from_slice(&((h.nodes + 9) * BLOCK).to_le_bytes());
    assert_eq!(refused(&x, "a child outside the tree"), (root + coff) as u64, "a child outside the tree");
    let mut x = b.image.clone();
    x[root + coff..root + coff + 8].copy_from_slice(&(BLOCK + 8).to_le_bytes());
    assert_eq!(refused(&x, "a child not on a slot"), (root + coff) as u64, "a child not on a slot");
    let mut x = b.image.clone();
    let rec = h.oa as usize;
    x[rec + 8..rec + 16].copy_from_slice(&h.bytes.to_le_bytes());
    assert_eq!(refused(&x, "an overflow key past the image"), h.oa, "an overflow key past the image");
    let mut x = b.image.clone();
    x[rec + 16..rec + 24].copy_from_slice(&(h.w).to_le_bytes());
    assert_eq!(refused(&x, "an overflow key no longer than W"), h.oa, "an overflow key no longer than W");
    // A chunk header: b = 63 in the leaf's first entry (a posting chunk unless it is a count entry).
    let mut x = b.image.clone();
    let vo = ls + shape.leaf.voff as usize;
    let field = u64::from_be_bytes(x[ls + 8 + h.w as usize..ls + 16 + h.w as usize].try_into().unwrap());
    assert!(!is_count(field));
    x[vo + 2] = 63;
    assert_eq!(refused(&x, "a chunk header"), vo as u64, "a chunk header");
    let mut x = b.image.clone();
    x[vo] = 2;
    assert_eq!(refused(&x, "a chunk kind"), vo as u64, "a chunk kind");
}

#[test]
fn believed_runs_header_check_refuses_each_rule() {
    let b = sample_run();
    let x = expect_of(&b);
    let head = &b.image[..4096];
    check_header(head, 1, &x).unwrap();
    let refuse = |h: &[u8], x: &RunExpect| check_header(h, 1, x).unwrap_err();
    let patch = |off: usize, bytes: &[u8]| {
        let mut h = head.to_vec();
        h[off..off + bytes.len()].copy_from_slice(bytes);
        h
    };
    assert_eq!(refuse(&patch(0, b"NTIX"), &x).code, RefusalCode::BadMagic);
    assert_eq!(refuse(&patch(4, &9u16.to_le_bytes()), &x).code, RefusalCode::BadVersion);
    assert_eq!(refuse(&patch(6, &[1]), &x).code, RefusalCode::BadKind);
    assert_eq!(refuse(&patch(8, &1_000u64.to_le_bytes()), &x).code, RefusalCode::BadSizes);
    assert_eq!(refuse(&patch(8, &(b.plan.header.bytes + 4).to_le_bytes()), &x).code, RefusalCode::BadSizes);
    assert_eq!(refuse(head, &RunExpect { table_uid: 9, ..x }).code, RefusalCode::WrongTable);
    assert_eq!(refuse(head, &RunExpect { schema_hash: 9, ..x }).code, RefusalCode::SchemaChanged);
    assert_eq!(refuse(head, &RunExpect { index: 2, ..x }).code, RefusalCode::WrongIndex);
    assert_eq!(refuse(head, &RunExpect { key_spec_hash: 9, ..x }).code, RefusalCode::WrongIndex);
    assert_eq!(refuse(head, &RunExpect { w: 24, ..x }).code, RefusalCode::WrongIndex);
    assert_eq!(refuse(&patch(36, &12u16.to_le_bytes()), &x).code, RefusalCode::WrongIndex);
    assert_eq!(refuse(&patch(7, &[b.plan.header.flags & !1]), &x).code, RefusalCode::WrongIndex);
    let r = refuse(&patch(40, &0u64.to_le_bytes()), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 40));
    let r = refuse(&patch(56, &[65]), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 40));
    let r = refuse(&patch(72, &u64::MAX.to_le_bytes()), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 40));
    let r = refuse(&patch(80, &(b.plan.header.oa + PAGE).to_le_bytes()), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 80));
    let r = refuse(&patch(112, &(b.plan.header.lines + 64).to_le_bytes()), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 96));
    let r = refuse(&patch(7, &[b.plan.header.flags & !2]), &x);
    assert_eq!((r.code, r.field), (RefusalCode::BadSizes, 96));
    // Opening re-checks the geometry against the image's own length.
    assert!(Run::open(MemImage { bytes: &b.image[..b.image.len() - 4096], id: RUN_ID }, 0, 0, u64::MAX).is_err());
}

/// A run that stores a shared prefix: its header check refuses a prefix at image version 1, one not a whole
/// number of words, one whose rest past the header's room would reach past the filter or the image, and
/// fences whose stored keys share a further word (the prefix shorter than they give); counts, prefix sums and key steps read the
/// keys whole, every key the run holds with its prefix re-attached.
#[test]
fn runs_with_a_shared_prefix_read_and_check_it() {
    let mut items = Vec::new();
    for k in 0..300u64 {
        let key = text(format!("a/long/start/that/all/keys/share/{k:06}/tail").as_bytes());
        for j in 0..(1 + k % 3) {
            items.push((key.clone(), k * 10 + j, false));
        }
    }
    for force in [Some((8, 0)), Some((32, 8)), None] {
        let p = match force {
            Some((w, v)) => forced(w, v),
            None => params(1),
        };
        let b = build(&VecItems(items.clone()), &p, 0xA5, true).unwrap();
        let h = b.plan.header;
        // "a/long/start/that/all/keys/share/000" is 36 bytes: 32 of them, whole words.
        assert_eq!((h.prefix, h.version), (32, 2));
        let x = expect_of(&b);
        let head = &b.image[..4096];
        assert_eq!(check_header(head, 1, &x), Ok(h));
        let patch = |off: usize, bytes: &[u8]| {
            let mut v = head.to_vec();
            v[off..off + bytes.len()].copy_from_slice(bytes);
            v
        };
        let refused = |v: &[u8]| check_header(v, 1, &x).map(|_| ()).map_err(|r| (r.code, r.field));
        let pl = at::PREFIX_LEN as usize;
        assert_eq!(refused(&patch(4, &1u16.to_le_bytes())), Err((RefusalCode::BadSizes, at::PREFIX_LEN)));
        assert_eq!(refused(&patch(pl, &28u64.to_le_bytes())), Err((RefusalCode::BadSizes, at::PREFIX_LEN)));
        // Fences whose stored keys share their first word: the prefix is shorter than they give.
        let (ff, fl) = (at::FENCE_FIRST as usize, at::FENCE_LAST as usize);
        assert_eq!(refused(&patch(ff, &head[fl..fl + 8])), Err((RefusalCode::BadSizes, at::PREFIX_LEN)));
        let past = (h.filter.max(h.ok) + 4_096 + 1_824).next_multiple_of(8);
        assert_eq!(refused(&patch(pl, &past.to_le_bytes())), Err((RefusalCode::BadSizes, at::PREFIX_LEN)));
        let run = b.open();
        let (_, keys) = model(&items, h.v, 0);
        for (k, total) in &keys {
            assert_eq!(run.count_key(k).unwrap(), *total);
        }
        // Keys outside the prefix: below, above, and the prefix itself or a part of it.
        for k in [text(b"a/long/start/that/all/keys/shard"), text(b"b"), b"a/long/start/that/all/keys/sh".to_vec()] {
            assert_eq!(run.count_key(&k).unwrap(), 0);
        }
        assert_eq!(run.prefix(&text(b"a")).unwrap(), 0);
        assert_eq!(run.prefix(&text(b"b")).unwrap(), h.total);
        let listed = list_keys(&run, Dir::Fwd, 0);
        let want: Vec<(Vec<u8>, i64)> =
            keys.iter().map(|(k, t)| ([&k[..32], &stored(&k[32..], h.w)[..]].concat(), *t)).collect();
        assert_eq!(listed, want);
        let mut back = list_keys(&run, Dir::Rev, 0);
        back.reverse();
        assert_eq!(back, want);
        let mut scratch = vec![0u8; 1 << 12];
        let (n, _) = run.extreme_key(true, 0, &mut scratch).unwrap().unwrap();
        assert_eq!(&scratch[..n], &want.last().unwrap().0[..]);
    }
}

/// A run whose keys share 17,000 bytes stores them all as its shared prefix, the rest of it past the header's
/// room at the start of the key bytes, and counts every key it holds; a header claiming a prefix of a part of
/// a word, or one whose rest would pass the image's filter, is refused by the header check and by a reader's
/// open.
#[test]
fn a_prefix_past_16_kib_is_stored_whole() {
    let mut items = Vec::new();
    for k in 0..40u64 {
        let mut raw = vec![b'p'; 17_000];
        raw.extend_from_slice(format!("{k:04}").as_bytes());
        items.push((text(&raw), k + 1, false));
    }
    let b = build(&VecItems(items.clone()), &params(1), 0xA5, true).unwrap();
    let h = b.plan.header;
    // 17,002 bytes shared ("p…p00"): their whole words.
    assert_eq!((h.prefix, h.version), (17_000, 2));
    let x = expect_of(&b);
    let head = &b.image[..4096];
    assert_eq!(check_header(head, b.image.len() as u64, &x), Ok(h));
    let run = b.open();
    for (k, _, _) in &items {
        assert_eq!(run.count_key(k).unwrap(), 1);
    }
    let mut prefix = vec![0u8; 17_000];
    assert_eq!(run.prefix_into(&mut prefix).unwrap(), 17_000);
    assert!(prefix.iter().all(|&c| c == b'p'));
    let pl = at::PREFIX_LEN as usize;
    let past = (h.filter.max(h.ok) + 4_096 + 1_824).next_multiple_of(8);
    for p in [h.prefix + 4, past] {
        let mut img = b.image.clone();
        img[pl..pl + 8].copy_from_slice(&p.to_le_bytes());
        let r = check_header(&img[..4096], b.image.len() as u64, &x).map(|_| ()).map_err(|r| (r.code, r.field));
        assert_eq!(r, Err((RefusalCode::BadSizes, at::PREFIX_LEN)), "a prefix of {p}");
        assert!(Run::open(MemImage { bytes: &img, id: RUN_ID }, 0, 0, u64::MAX).is_err(), "a prefix of {p}");
    }
}

/// Every lookup of a run: finds, counts, prefixes, ranks, bounded scans and key steps, as a digest; errors
/// must be faults of this run.
fn lookups(image: &[u8], keys: &[Vec<u8>]) -> Vec<String> {
    let mut out = Vec::new();
    let run = match Run::open(MemImage { bytes: image, id: RUN_ID }, 0, 0, u64::MAX) {
        Ok(r) => r,
        Err(f) => {
            assert!(is_run_fault(&f, RUN_ID), "{f:?}");
            return vec![format!("{f:?}")];
        }
    };
    let mut note = |r: Result<String, Fault>| match r {
        Ok(s) => out.push(s),
        Err(f) => {
            assert!(is_run_fault(&f, RUN_ID), "{f:?}");
            out.push(format!("{f:?}"));
        }
    };
    for k in keys {
        note(run.count_key(k).map(|c| c.to_string()));
        note(run.prefix(k).map(|c| c.to_string()));
        note(run.main().and_then(|t| t.find(&KeyProbe { key: k, kind: Kind::Count })).map(|x| format!("{x:?}")));
    }
    let mut finger = Finger::default();
    for r in [0i64, 7, 100, 1_000] {
        note(run.select_rank(r, &mut finger).map(|x| format!("{x:?}")));
    }
    note((|| {
        let t = run.main()?;
        let mut c = t.seek::<Fwd>(Included(&KeyProbe { key: &keys[0], kind: Kind::Low }))?;
        let mut s = String::new();
        for _ in 0..40 {
            match t.next(&mut c, Unbounded)? {
                Some((n, i)) => s.push_str(&format!("{n}:{i},")),
                None => break,
            }
        }
        Ok(s)
    })());
    note((|| {
        let t = run.main()?;
        let mut c = t.seek::<Rev>(Unbounded)?;
        let mut s = String::new();
        for _ in 0..40 {
            match t.next(&mut c, Unbounded)? {
                Some((n, i)) => s.push_str(&format!("{n}:{i},")),
                None => break,
            }
        }
        Ok(s)
    })());
    let mut buf = vec![0u8; 1 << 14];
    note(run.key_step(Some(&keys[1]), Dir::Fwd, 0, &mut buf).map(|x| format!("{x:?}")));
    note(run.key_step(Some(&keys[1]), Dir::Rev, 0, &mut buf).map(|x| format!("{x:?}")));
    let mut sc = StructureCheck::new();
    out.push(format!("{:?}", sc.step(&MemImage { bytes: image, id: RUN_ID }, &run.hdr, 1, u64::MAX)));
    out
}

#[test]
fn believed_runs_wrong_but_valid_runs_are_deterministic() {
    let b = sample_run();
    let h = b.plan.header;
    let keys: Vec<Vec<u8>> = {
        let run = b.open();
        list_keys(&run, Dir::Fwd, 0).into_iter().map(|(k, _)| k).step_by(9).collect()
    };
    let shape = IndexShape::new(h.w, h.v).unwrap();
    let root = BLOCK as usize;
    let coff = shape.internal.coff as usize;
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    // A cycle: the root's first child is the root.
    let mut x = b.image.clone();
    x[root + coff..root + coff + 8].copy_from_slice(&BLOCK.to_le_bytes());
    {
        let run = Run::open(MemImage { bytes: &x, id: RUN_ID }, 0, 0, u64::MAX).unwrap();
        let t = run.main().unwrap();
        let low = KeyProbe { key: &[], kind: Kind::Low };
        let f = t.find(&low).unwrap_err();
        assert!(matches!(f, Fault::Corrupt(CorruptAt { image: RUN_ID, kind: Corruption::Depth, .. })), "{f:?}");
        assert!(matches!(t.seek::<Fwd>(Unbounded), Err(Fault::Corrupt(CorruptAt { kind: Corruption::Depth, .. }))));
    }
    cases.push(("cycle", x));
    // Shared subtrees: every child of the root is its first child.
    let mut x = b.image.clone();
    let c0 = x[root + coff..root + coff + 8].to_vec();
    let rc = u16::from_le_bytes([x[root], x[root + 1]]) as usize;
    for j in 1..=rc {
        x[root + coff + 8 * j..root + coff + 8 * j + 8].copy_from_slice(&c0);
    }
    cases.push(("shared subtrees", x));
    // Shuffled: two leaf entries' key parts swapped.
    let mut x = b.image.clone();
    let ls = (2 * BLOCK) as usize;
    let ks = h.w as usize + 8;
    let (a, bb) = (ls + 8, ls + 8 + ks);
    let tmp = x[a..a + ks].to_vec();
    x.copy_within(bb..bb + ks, a);
    x[bb..bb + ks].copy_from_slice(&tmp);
    cases.push(("shuffled", x));
    // Wrong cumulative counts, fences, filter, totals and rid range.
    let mut x = b.image.clone();
    for s in 1..=h.nodes {
        let t = (s * BLOCK + NODE_SIZE) as usize;
        x[t..t + 8].copy_from_slice(&(-(s as i64) * 1_000).to_le_bytes());
        x[t + 8] = (s % 2) as u8;
    }
    cases.push(("trailers", x));
    let mut x = b.image.clone();
    x[at::FENCE_FIRST as usize..at::FENCE_LAST as usize + 1024].fill(0xFF);
    cases.push(("fences", x));
    let mut x = b.image.clone();
    x[h.filter as usize..(h.filter + h.filter_blocks * BLOCK) as usize].fill(0);
    cases.push(("filter", x));
    let mut x = b.image.clone();
    x[at::TOTAL as usize..at::TOTAL as usize + 8].copy_from_slice(&i64::MIN.to_le_bytes());
    cases.push(("total", x));
    for (name, img) in &cases {
        let first = lookups(img, &keys);
        let second = lookups(img, &keys);
        assert_eq!(first, second, "{name}: lookups are deterministic");
    }
    // Rids outside the runtime-recorded range are not contained.
    assert!(native_table::index::run::contains(5, 5, 9) && !native_table::index::run::contains(10, 5, 9));
}

#[test]
fn believed_runs_fuzzed_never_panic() {
    let bases = [
        sample_run(),
        build(
            &VecItems((0..3_000u64).map(|i| (k8(i), i, false)).collect()),
            &RunParams { key_run_l: 8, ..forced(8, 8) },
            0xA5,
            false,
        )
        .unwrap(),
        build(&VecItems(random_items(&mut XorShift(5), 300, 50, 1 << 30, 1, 12, true)), &forced(8, 0), 0xA5, false)
            .unwrap(),
    ];
    let mut g = XorShift(0xF00D);
    let mut faults = 0u64;
    for case in 0..100_000u64 {
        let b = &bases[(case % 3) as usize];
        let mut image = b.image.clone();
        let h = b.plan.header;
        let flips = 1 + g.below(6);
        for _ in 0..flips {
            // Mostly the header and the node slots, sometimes the overflow area and the filter.
            let off = match g.below(4) {
                0 => g.below(at::END),
                1 | 2 => BLOCK + g.below(h.slots() * BLOCK),
                _ => g.below(image.len() as u64),
            } as usize;
            image[off] = match g.below(3) {
                0 => g.next_u64() as u8,
                1 => image[off] ^ (1 << g.below(8)),
                _ => [0, 0xFF, 1, 2, 3][g.below(5) as usize],
            };
        }
        let run = match Run::open(MemImage { bytes: &image, id: RUN_ID }, 0, 0, u64::MAX) {
            Ok(r) => r,
            Err(f) => {
                assert!(is_run_fault(&f, RUN_ID));
                faults += 1;
                continue;
            }
        };
        let key = match case % 3 {
            1 => k8(g.below(3_100)),
            _ => text(&[b'A', b'A', (g.below(250) + 1) as u8]),
        };
        let mut check = |r: Result<(), Fault>| {
            if let Err(f) = r {
                assert!(is_run_fault(&f, RUN_ID), "case {case}: {f:?}");
                faults += 1;
            }
        };
        check(run.count_key(&key).map(|_| ()));
        if case % 4 == 0 {
            check(run.prefix(&key).map(|_| ()));
            let mut finger = Finger::default();
            check(run.select_rank(g.below(500) as i64, &mut finger).map(|_| ()));
        }
        if case % 4 == 1 {
            check((|| {
                let t = run.main()?;
                let mut c = t.seek::<Fwd>(Included(&KeyProbe { key: &key, kind: Kind::Low }))?;
                for _ in 0..16 {
                    if t.next(&mut c, Unbounded)?.is_none() {
                        break;
                    }
                }
                Ok(())
            })());
        }
        if case % 4 == 2 {
            let mut buf = vec![0u8; 4096];
            check(run.key_step(Some(&key), if g.below(2) == 0 { Dir::Fwd } else { Dir::Rev }, 8, &mut buf).map(|_| ()));
        }
        if case % 16 == 3 {
            let mut sc = StructureCheck::new();
            let _ = sc.step(&MemImage { bytes: &image, id: RUN_ID }, &run.hdr, 1, u64::MAX);
        }
    }
    assert!(faults > 1_000);
}

// ---------------------------------------------------------------- golden_runs

struct GoldenCase {
    name: &'static str,
    items: Items,
    params: RunParams,
}

fn golden_cases() -> Vec<GoldenCase> {
    let mut g = XorShift(0x601D);
    let mut cases = Vec::new();
    let rel = |index: u8, w: u64, v: u64| RunParams { index, ..forced(w, v) };
    cases.push(GoldenCase { name: "w8_v0_n1", items: vec![(k8(7), 3, false)], params: rel(1, 8, 0) });
    cases.push(GoldenCase {
        name: "w8_v0_nl",
        items: (0..254u64).map(|i| (k8(i * 5), i, false)).collect(),
        params: rel(1, 8, 0),
    });
    cases.push(GoldenCase {
        name: "w8_v0_nl_plus_1",
        items: (0..255u64).map(|i| (k8(i * 5), i, false)).collect(),
        params: rel(1, 8, 0),
    });
    let mut big: Items = (0..100_000u64).map(|i| (k8(i.wrapping_mul(0x9E37_79B9) % 1_000_003), i, false)).collect();
    big.sort();
    big.dedup_by(|a, b| a.0 == b.0);
    cases.push(GoldenCase { name: "w8_v0_1e5_primary", items: big, params: rel(0, 8, 0) });
    let mut dense = Vec::new();
    for k in 0..300u64 {
        for j in 0..(1 + k % 40) {
            dense.push((k8(k), k * 1_000 + j * (1 + k % 3), false));
        }
    }
    // Gaps of about 2^20: two rids per 8-byte chunk, so keys of 16 rows and more get count entries.
    let sparse: Items =
        (0..300u64).flat_map(|k| (0..(1 + k % 40)).map(move |j| (k8(k), (k << 30) + j * 1_000_003, false))).collect();
    cases.push(GoldenCase { name: "w32_v8_counts", items: sparse, params: rel(2, 32, 8) });
    cases.push(GoldenCase { name: "w8_v128_postings", items: dense, params: rel(2, 8, 128) });
    let mut wide = Vec::new();
    for k in 0..40u64 {
        let mut r = k;
        for _ in 0..(200 + 37 * k) {
            r += 1 + g.below(9);
            wide.push((k8(k), r, false));
        }
    }
    cases.push(GoldenCase { name: "w8_v256_postings", items: wide, params: rel(2, 8, 256) });
    let mut long = Vec::new();
    for k in 0..60u64 {
        let key = text(&vec![b'a' + (k % 26) as u8; 1_000 + 17 * k as usize]);
        for j in 0..(1 + k % 12) {
            long.push((key.clone(), k * 100 + j, false));
        }
    }
    long.sort();
    cases.push(GoldenCase { name: "w1016_v0_long_keys", items: long, params: rel(3, 1_016, 0) });
    cases.push(GoldenCase {
        name: "w8_v8_key_runs",
        items: (0..5_000u64).filter(|i| i % 777 != 5).map(|i| (k8(i), i + 10, false)).collect(),
        params: RunParams { key_run_l: 8, ..rel(0, 8, 8) },
    });
    let mut signed = Vec::new();
    for k in 0..200u64 {
        for j in 0..(1 + k % 20) {
            signed.push((text(format!("key-{k:05}").as_bytes()), k * 50 + j, j % 3 == 1));
        }
    }
    signed.sort();
    cases.push(GoldenCase { name: "w32_v16_rows_and_tombstones", items: signed, params: rel(4, 32, 16) });
    let mut ovf = Vec::new();
    for k in 0..500u64 {
        let key = text(format!("a-shared-prefix/{k:06}-overflow").as_bytes());
        for j in 0..(2 + k % 14) {
            ovf.push((key.clone(), k * 64 + j, false));
        }
    }
    ovf.sort();
    cases.push(GoldenCase { name: "w8_v16_overflowed_count_tree", items: ovf, params: rel(5, 8, 16) });
    let mut fits = Vec::new();
    for k in 0..300u64 {
        let key = text(format!("shared-start-of-24-bytes/{k:05}").as_bytes());
        for j in 0..(1 + k % 5) {
            fits.push((key.clone(), k * 16 + j, false));
        }
    }
    cases.push(GoldenCase { name: "w16_v8_prefix_fits", items: fits, params: rel(6, 16, 8) });
    let mut tail = Vec::new();
    for k in 0..100u64 {
        let mut raw = vec![b'p'; 1_900];
        raw.extend_from_slice(format!("{k:04}").as_bytes());
        tail.push((text(&raw), k + 1, false));
    }
    cases.push(GoldenCase { name: "w8_v0_prefix_tail", items: tail, params: rel(7, 8, 0) });
    cases
}

#[test]
fn golden_runs() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../golden/runs");
    let write = std::env::var("NT_GOLDEN_WRITE").is_ok();
    let mut seen = Vec::new();
    for c in golden_cases() {
        let items = VecItems(c.items);
        let b = build(&items, &c.params, 0xA5, true).unwrap();
        let other = build(&items, &c.params, 0x5A, false).unwrap();
        assert_eq!(b.image, other.image);
        let path = dir.join(format!("{}.bin", c.name));
        if write {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &b.image).unwrap();
            let h = b.plan.header;
            println!(
                "| `{}.bin` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                c.name,
                h.index,
                h.w,
                h.v,
                items.0.len(),
                h.n,
                h.nodes,
                h.height,
                h.d,
                h.records,
                if h.has_count_tree() { h.ct_nodes.to_string() } else { "–".into() },
                if h.has_filter() { h.filter_blocks.to_string() } else { "–".into() },
                h.bytes
            );
        }
        let want =
            std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e} (write with NT_GOLDEN_WRITE=1)", path.display()));
        assert!(want == b.image, "{} differs from the golden image", c.name);
        // Every golden run reads back: its keys and totals equal the model.
        let run = b.open();
        let (_, keys) = model(&items.0, b.plan.header.v, c.params.key_run_l as usize);
        let klen = if c.params.key_run_l > 0 { 8 } else { 0 };
        let listed = list_keys(&run, Dir::Fwd, klen);
        let p = b.plan.header.prefix as usize;
        assert_eq!(p, nt_testhost::runs::run_prefix(&items.0, c.params.key_run_l as usize), "{}", c.name);
        let want_keys: Vec<(Vec<u8>, i64)> =
            keys.iter().map(|(k, t)| ([&k[..p], &stored(&k[p..], b.plan.header.w)[..]].concat(), *t)).collect();
        assert_eq!(listed, want_keys, "{}", c.name);
        // The structure check and the header check of an upload pass every golden run.
        let x = RunExpect {
            table_uid: 0x1111,
            schema_hash: 0x2222,
            index: c.params.index,
            key_spec_hash: 0x3333,
            w: b.plan.header.w,
            key_run_l: c.params.key_run_l,
        };
        assert_eq!(check_header(&b.image[..BLOCK as usize], 1, &x), Ok(b.plan.header), "{}", c.name);
        let mut sc = StructureCheck::new();
        assert_eq!(sc.step(&b.image(c.params.index), &b.plan.header, 1, u64::MAX), Ok(Progress::Done), "{}", c.name);
        seen.push(c.name);
    }
    // Every file in the directory is a case.
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".bin"))
        .collect();
    files.sort();
    let mut names: Vec<String> = seen.iter().map(|n| format!("{n}.bin")).collect();
    names.sort();
    assert_eq!(files, names);
}

// ---------------------------------------------------------------- the filter and the header helpers

#[test]
fn filters_admit_every_key_and_few_others() {
    let mut g = XorShift(17);
    let items = VecItems(random_items(&mut g, 5_000, 2, 1 << 30, 3, 20, false));
    let b = build(&items, &params(1), 0xA5, true).unwrap();
    let h = b.plan.header;
    assert!(h.has_filter() && h.nodes > 1);
    assert_eq!(h.lines, filter_lines_of(h.d));
    let run = b.open();
    let (_, keys) = model(&items.0, h.v, 0);
    assert!(keys.iter().all(|(k, _)| run.filter_admits(k).unwrap()));
    let mut admitted = 0;
    for i in 0..20_000u64 {
        let k = text(format!("absent-{i}").as_bytes());
        if run.filter_admits(&k).unwrap() {
            admitted += 1;
        }
    }
    // About 0.8% false positives at 10 bits per key and 7 bits tested.
    assert!(admitted < 600, "{admitted} false positives in 20,000");
    // A one-node run has no filter.
    let small = build(&VecItems(vec![(k8(1), 1, false), (k8(2), 2, false)]), &params(1), 0xA5, true).unwrap();
    assert!(!small.plan.header.has_filter() && small.plan.header.filter == 0);
    assert_eq!(small.image.len() as u64, PAGE);
}

fn filter_lines_of(d: u64) -> u64 {
    ((d * 10).div_ceil(512).max(64)).div_ceil(64) * 64
}

#[test]
fn builder_refuses_input_out_of_order() {
    let bad: [Items; 4] = [
        vec![(k8(2), 1, false), (k8(1), 2, false)],
        vec![(k8(1), 5, false), (k8(1), 4, false)],
        vec![(k8(1), 5, false), (k8(1), 5, true)],
        vec![(b"ab".to_vec(), 1, false), (b"abc".to_vec(), 2, false)],
    ];
    for items in bad {
        let r = plan_run(&VecItems(items), &params(1));
        assert!(matches!(r, Err(Fault::Corrupt(CorruptAt { image: ImageId::Runtime, kind: Corruption::Order, .. }))));
    }
    assert!(plan_run(&VecItems(vec![(k8(1), RID_PAYLOAD + 1, false)]), &params(1)).is_err());
    assert!(plan_run(&VecItems(vec![]), &params(1)).is_err());
    // A position held twice is kept: two items, one chunk.
    let twice = VecItems(vec![(k8(1), 5, false), (k8(1), 5, false), (k8(1), 9, false)]);
    let b = build(&twice, &forced(8, 8), 0xA5, true).unwrap();
    let got = scan::<Fwd>(&b.open(), Unbounded, Unbounded);
    assert_eq!(got.len(), 1);
    assert_eq!(posting_rids(&got[0].value, got[0].payload), vec![5, 5, 9]);
    assert_eq!(SENT_MIN, 8);
}

// ---------------------------------------------------------------- identical bytes on every target

/// The probe's run builds and reads, natively and in the wasm64 probe under wasmtime: equal results (each
/// `nt_run` result folds in a hash of the whole image). Set `NT_PROBE_WASM` to the built probe
/// (scripts/check.sh does) to run the wasm side.
#[test]
fn runs_are_identical_on_every_target() {
    let calls: Vec<(&str, Vec<u64>, u64)> = vec![
        ("nt_run", vec![11, 80, 8, 16], nt_probe::nt_run(11, 80, 8, 16)),
        ("nt_run", vec![12, 95, 0, 0], nt_probe::nt_run(12, 95, 0, 0)),
        ("nt_run", vec![13, 60, 16, 0], nt_probe::nt_run(13, 60, 16, 0)),
        ("nt_run", vec![14, 90, 8, 128], nt_probe::nt_run(14, 90, 8, 128)),
        ("nt_bulk", vec![5, 599], nt_probe::nt_bulk(5, 599)),
        ("nt_run_believed", vec![3, 9, 17], nt_probe::nt_run_believed(3, 9, 17)),
        ("nt_plan", vec![5, 90, 24], nt_probe::nt_plan(5, 90, 24)),
    ];
    for (f, args, native) in &calls {
        if *f != "nt_run_believed" {
            assert!(*native < u64::MAX - 64, "{f}{args:?} faulted natively");
        }
    }
    let Ok(wasm) = std::env::var("NT_PROBE_WASM") else {
        eprintln!("NT_PROBE_WASM not set: the wasm side is checked by scripts/check.sh");
        return;
    };
    for (f, args, native) in calls {
        let out = std::process::Command::new("wasmtime")
            .args(["run", "-W", "memory64=y", "--invoke", f])
            .arg(&wasm)
            .args(args.iter().map(|a| a.to_string()))
            .output()
            .expect("wasmtime runs");
        let got: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().expect("a number");
        assert_eq!(got as u64, native, "{f}{args:?}");
    }
}

// ---------------------------------------------------------------- the measure's tally

/// The tally of a key's items one `Chunker::offer` at a time, as the measure counted them first.
fn offer_items(t: &mut native_table::index::plan::Tally, rids: &[u64], tomb: bool) {
    t.items += rids.len() as u64;
    if tomb {
        t.tombs += rids.len() as u64;
    } else {
        t.rows += rids.len() as u64;
    }
    for &rid in rids {
        for c in t.cands.iter_mut() {
            if !c.chunker.offer(rid, tomb) {
                c.entries += 1;
                c.key_chunks += 1;
            }
        }
    }
}

/// A key's rids: gaps of a few bits, of many, repeats, a falling rid now and then.
fn tally_rids(g: &mut XorShift, n: usize) -> Vec<u64> {
    let mut r = g.below(1 << 40);
    let shape = g.below(4);
    (0..n)
        .map(|_| {
            let gap = match shape {
                0 => g.below(8),
                1 => g.below(1 << 20),
                2 => {
                    if g.below(10) == 0 {
                        0
                    } else {
                        let bits = g.below(40);
                        g.below(1 << bits)
                    }
                }
                _ => g.below(1 << 12),
            };
            if g.below(50) == 0 && r > 100 {
                r -= g.below(100);
            } else {
                r = (r + gap).min(RID_PAYLOAD);
            }
            r
        })
        .collect()
}

#[test]
fn tally_items_equal_offers_one_at_a_time() {
    use native_table::index::plan::Tally;
    let mut g = XorShift(0x7A11_7A11);
    for round in 0..400 {
        let (mut a, mut b) = (Tally::new(), Tally::new());
        for _key in 0..1 + g.below(6) {
            for t in [&mut a, &mut b] {
                t.begin_key();
            }
            let n = 1 + g.below(300) as usize;
            let rids = tally_rids(&mut g, n);
            // Runs of one sign, as a key's items come.
            let mut i = 0;
            while i < n {
                let k = (1 + g.below(80) as usize).min(n - i);
                let tomb = g.below(3) == 0;
                let part = &rids[i..i + k];
                offer_items(&mut a, part, tomb);
                for &rid in part {
                    b.item(8, rid, tomb).unwrap();
                }
                i += k;
            }
            for t in [&mut a, &mut b] {
                t.end_key().unwrap();
            }
        }
        assert_eq!(a, b, "round {round}: item one at a time");
    }
}
