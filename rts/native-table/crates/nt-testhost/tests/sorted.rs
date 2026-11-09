//! The sorted walk of a secondary index (`index::secondary::sorted`) against a model of its rows, and the rows
//! of one key in a run (`Run::row_after`) against the items the run was built from.
//!
//! A run's rows of a key: text keys of one to hundreds of rows (posting chunks), shorter and longer than the
//! key area, at several (W, V), with absolute and relative rids, and 8-byte keys whose lone rows form key runs;
//! every key's rows in rid order, from the start and from rids inside and outside them, and a tombstone refused.
//!
//! The walk: a tree of random inserts and deletes beside two uploaded segments, under rids that interleave with
//! the segments' and sometimes equal them, under both fill patterns. Forward and backward, from an end, from
//! every key held and keys between them, and after rows: the rows handed out equal the model's, keys in order
//! and the rows of one key in rid order; a walk in pieces of a few rows, each resumed after the last row,
//! equals the whole walk; the smallest and largest keys equal the model's. Also with no segment, and with the
//! segments alone.

use std::collections::{BTreeMap, BTreeSet};

use native_table::area::new_root;
use native_table::index::secondary::{Secondary, Start};
use native_table::index::{MemImage, Run};
use native_table::key::{enc_bytes, KeyKind, KeyPart, KeySpec};
use native_table::tree::{Dir, Path};
use native_table::{CorruptAt, Corruption, Fault};
use nt_builder::RunParams;
use nt_testhost::runs::{build, VecItems};
use nt_testhost::uploads::TTable;
use nt_testhost::{VecHost, VecSink, XorShift, FILLS};

type Items = Vec<(Vec<u8>, u64, bool)>;

/// The index's key spec: one Text part.
const PARTS: [KeyPart; 1] = [KeyPart { leaf: 0, kind: KeyKind::Text, nullable: false }];
const SPEC: KeySpec<'static> = KeySpec { parts: &PARTS };

/// A text key's encoding.
fn text(t: &[u8]) -> Vec<u8> {
    let mut s = VecSink::default();
    enc_bytes(&mut s, t).unwrap();
    s.0
}

/// `n` distinct text keys of 0 to `max` letters (with zero bytes, which the encoding escapes), in key order.
fn pool(r: &mut XorShift, n: usize, max: u64) -> Vec<Vec<u8>> {
    let mut keys = BTreeSet::new();
    while keys.len() < n {
        let len = r.below(max + 1) as usize;
        let t: Vec<u8> = (0..len).map(|_| [b'a', b'b', b'c', 0][r.below(4) as usize]).collect();
        keys.insert(text(&t));
    }
    keys.into_iter().collect()
}

fn params(relative: bool, key_run_l: u8, force: Option<(u64, u64)>) -> RunParams {
    RunParams {
        index: 1,
        relative,
        w0: 0,
        key_run_l,
        table_uid: 0x1111,
        schema_hash: 0x2222,
        key_spec_hash: 0x3333,
        force,
    }
}

// ---------------------------------------------------------------- a run's rows of a key

/// Every row of `key` in `run`, through `row_after` from the start; each rid above the one before.
fn rows_of(run: &Run<MemImage<'_>>, key: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut after = None;
    while let Some(r) = run.row_after(key, after).unwrap() {
        assert!(after.is_none_or(|a| r > a), "rids rise");
        out.push(r);
        after = Some(r);
    }
    out
}

/// `run`, built from `items` (stored rids plus `base`), against them: every key's rows, from the start and
/// from rids inside and outside them, and keys it does not hold.
fn check_run(run: &Run<MemImage<'_>>, items: &Items, base: u64, absent: &[Vec<u8>], r: &mut XorShift, what: &str) {
    let mut want: BTreeMap<&[u8], Vec<u64>> = BTreeMap::new();
    for (k, rid, _) in items {
        want.entry(k).or_default().push(base + rid);
    }
    for (k, rids) in &want {
        assert_eq!(rows_of(run, k), *rids, "{what}: rows of {}", nt_testhost::hex(k));
        for _ in 0..4 {
            let a = rids[r.below(rids.len() as u64) as usize] + r.below(3) - 1;
            let next = rids.iter().copied().find(|&x| x > a);
            assert_eq!(run.row_after(k, Some(a)).unwrap(), next, "{what}: after {a}");
        }
        assert_eq!(run.row_after(k, Some(u64::MAX)).unwrap(), None);
    }
    for k in absent {
        if !want.contains_key(k.as_slice()) {
            assert_eq!(run.row_after(k, None).unwrap(), None, "{what}: a key the run does not hold");
        }
    }
}

#[test]
fn run_rows_of_a_key() {
    let mut r = XorShift(0x0123_4567);
    // Text keys of one to hundreds of rows each, under rising rids; some longer than W.
    let keys = pool(&mut r, 60, 30);
    let mut items: Items = Vec::new();
    let mut rid = 5u64;
    for k in &keys {
        let n = match r.below(4) {
            0 => 1,
            1 => 1 + r.below(5),
            2 => 20 + r.below(60),
            _ => 200 + r.below(400),
        };
        for _ in 0..n {
            let gap = if r.below(5) == 0 { 5_000 } else { 3 };
            rid += 1 + r.below(gap);
            items.push((k.clone(), rid, false));
        }
    }
    let items = VecItems::sorted(items);
    let absent = pool(&mut XorShift(0xAB5E), 40, 30);
    for (relative, force) in [(false, None), (false, Some((8, 0))), (true, Some((16, 16))), (true, Some((24, 64)))] {
        let b = build(&items, &params(relative, 0, force), FILLS[0], false).unwrap();
        let base = if relative { 1 << 40 } else { 0 };
        let run = Run::open(b.image(1), base, 0, u64::MAX).unwrap();
        let what = format!("text keys, relative {relative}, (W, V) {force:?}");
        check_run(&run, &items.0, base, &absent, &mut r, &what);
    }
    // 8-byte keys after a 3-byte prefix: lone rows stepping by one in key and rid form key runs, among keys of
    // three rows.
    let key = |f: u64| [b"pfx".as_slice(), &f.to_be_bytes()].concat();
    let mut items: Items = Vec::new();
    let (mut f, mut rid) = (r.below(5), 100u64);
    while items.len() < 3_000 {
        match r.below(12) {
            0 => f += 1 + r.below(3),
            1 => rid += 1 + r.below(3),
            2 => {
                for j in 0..3 {
                    items.push((key(f), rid + j, false));
                }
                (rid, f) = (rid + 3, f + 1);
            }
            _ => {
                items.push((key(f), rid, false));
                (rid, f) = (rid + 1, f + 1);
            }
        }
    }
    let items = VecItems::sorted(items);
    let absent: Vec<Vec<u8>> = (0..200).map(|_| key(r.below(f + 10))).collect();
    for v in [8, 16, 32] {
        let b = build(&items, &params(false, 8, Some((16, v))), FILLS[1], false).unwrap();
        assert!(b.plan.header.flags & native_table::index::run::F_KEY_RUNS != 0, "key runs formed");
        check_run(&b.open(), &items.0, 0, &absent, &mut r, &format!("key runs, V {v}"));
    }
    // A segment holds rows only: a tombstone of the key walked is refused.
    let items = VecItems::sorted(vec![(text(b"a"), 7, false), (text(b"a"), 9, true), (text(b"b"), 3, false)]);
    let b = build(&items, &params(false, 0, Some((8, 0))), FILLS[0], false).unwrap();
    let run = b.open();
    assert_eq!(run.row_after(&text(b"a"), None).unwrap(), Some(7));
    assert!(matches!(
        run.row_after(&text(b"a"), Some(7)),
        Err(Fault::Corrupt(CorruptAt { kind: Corruption::Encoding, .. }))
    ));
    assert_eq!(run.row_after(&text(b"b"), None).unwrap(), Some(3));
}

// ---------------------------------------------------------------- the walk

/// The rows of the tree and the segments: key → rids.
type Model = BTreeMap<Vec<u8>, BTreeSet<u64>>;

/// A row as a walk hands it out.
type Row = (Vec<u8>, u64);

/// The model's rows in the walk's order from `start`: keys rising forward and falling backward, the rows of
/// one key in rid order either way.
fn model_walk(m: &Model, dir: Dir, start: Start<'_>) -> Vec<Row> {
    let mut keys: Vec<&Vec<u8>> = m.iter().filter(|(_, rids)| !rids.is_empty()).map(|(k, _)| k).collect();
    if dir == Dir::Rev {
        keys.reverse();
    }
    // Whether key k comes after key s in the walk's order.
    let later = |k: &[u8], s: &[u8]| if dir == Dir::Fwd { k > s } else { k < s };
    let mut out = Vec::new();
    for k in keys {
        for &r in &m[k] {
            let take = match start {
                Start::End => true,
                Start::Key(s) => k.as_slice() == s || later(k, s),
                Start::After(s, a) => later(k, s) || (k.as_slice() == s && r > a),
            };
            if take {
                out.push((k.clone(), r));
            }
        }
    }
    out
}

/// A walk of the index from `start`, at most `limit` rows.
fn walk(ix: &Secondary, h: &VecHost, dir: Dir, start: Start<'_>, limit: u64) -> Vec<Row> {
    let (mut key, mut scratch) = (vec![0u8; 4_096], vec![0u8; 8_192]);
    let mut out = Vec::new();
    let n = ix
        .walk(h, &SPEC, dir, start, limit, &mut key, &mut scratch, |k, r| {
            out.push((k.to_vec(), r));
            Ok(())
        })
        .unwrap();
    assert_eq!(n, out.len() as u64);
    assert!(n <= limit);
    out
}

/// The whole walk in pieces of `step` rows, each resumed after the last row of the one before.
fn walk_in_pieces(ix: &Secondary, h: &VecHost, dir: Dir, step: u64) -> Vec<Row> {
    let mut all = walk(ix, h, dir, Start::End, step);
    while let Some((k, r)) = all.last().cloned() {
        let more = walk(ix, h, dir, Start::After(&k, r), step);
        if more.is_empty() {
            break;
        }
        all.extend(more);
    }
    all
}

/// The index's smallest and largest keys.
fn ends(ix: &Secondary, h: &VecHost) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let (mut out, mut scratch) = (vec![0u8; 4_096], vec![0u8; 4_096]);
    let min = ix.min_key(h, &SPEC, &mut out, &mut scratch).unwrap().map(|n| out[..n].to_vec());
    let max = ix.max_key(h, &SPEC, &mut out, &mut scratch).unwrap().map(|n| out[..n].to_vec());
    (min, max)
}

/// Every walk of the index against the model; `probes` are keys to start at, held or not. Returns the rows
/// walked, for comparing fill patterns.
fn check_walks(ix: &Secondary, h: &VecHost, m: &Model, probes: &[Vec<u8>], r: &mut XorShift, what: &str) -> Vec<Row> {
    let mut seen = Vec::new();
    for dir in [Dir::Fwd, Dir::Rev] {
        let want = model_walk(m, dir, Start::End);
        let got = walk(ix, h, dir, Start::End, u64::MAX);
        assert!(
            got == want,
            "{what}, {dir:?}: the walk differs from the model ({} rows, {} expected)",
            got.len(),
            want.len()
        );
        for step in [1, 3, 17] {
            assert!(walk_in_pieces(ix, h, dir, step) == want, "{what}, {dir:?}: a walk in pieces of {step}");
        }
        assert!(walk(ix, h, dir, Start::End, 0).is_empty());
        for k in probes {
            let limit = 1 + r.below(40);
            let want = model_walk(m, dir, Start::Key(k));
            let got = walk(ix, h, dir, Start::Key(k), limit);
            assert!(got[..] == want[..want.len().min(limit as usize)], "{what}, {dir:?}: from {}", nt_testhost::hex(k));
        }
        for _ in 0..60 {
            let Some((k, rids)) = m.iter().nth(r.below(m.len() as u64) as usize) else { break };
            let a = match rids.iter().nth(r.below(rids.len() as u64 + 1) as usize) {
                Some(&x) => (x + r.below(3)).saturating_sub(1),
                None => r.next_u64() >> 20,
            };
            let want = model_walk(m, dir, Start::After(k, a));
            let got = walk(ix, h, dir, Start::After(k, a), 25);
            assert!(got[..] == want[..want.len().min(25)], "{what}, {dir:?}: after {a} of {}", nt_testhost::hex(k));
        }
        seen.extend(got);
    }
    let held: Vec<&Vec<u8>> = m.iter().filter(|(_, rids)| !rids.is_empty()).map(|(k, _)| k).collect();
    let want = (held.first().map(|k| k.to_vec()), held.last().map(|k| k.to_vec()));
    assert_eq!(ends(ix, h), want, "{what}: the smallest and largest keys");
    seen
}

/// The rows of the tree and of the segments together.
fn union(tree: &Model, segs: &Model) -> Model {
    let mut m = segs.clone();
    for (k, rids) in tree {
        m.entry(k.clone()).or_default().extend(rids);
    }
    m
}

/// One case under fill `fill`: two uploaded segments (when `segments`), a tree of `ops` inserts and deletes
/// (none when 0), and every walk against the model, before and after deleting half the tree's rows.
fn case(fill: u8, segments: bool, ops: u64, seed: u64) -> Vec<Row> {
    let mut r = XorShift(seed);
    let keys = pool(&mut r, 300, 30);
    let mut t = TTable::new(fill);
    // The segments' rows and the tree's.
    let (mut segs, mut m) = (Model::new(), Model::new());
    // The segments' rows: rids relative to each upload's base, from keys of the first two thirds of the pool.
    let bases = [4_096 * 1_000, 4_096 * 1_002];
    if segments {
        for (base, n) in bases.into_iter().zip([1_500u64, 700]) {
            let mut items: Items = (0..n).map(|_| (keys[r.below(200) as usize].clone(), 0, false)).collect();
            items.sort();
            let items: Items = items.into_iter().enumerate().map(|(i, (k, _, t))| (k, i as u64, t)).collect();
            for (k, rid, _) in &items {
                segs.entry(k.clone()).or_default().insert(base + rid);
            }
            t.add_loaded(&VecItems(items), base, 2);
        }
    }
    let mut ix = Secondary::create(t.ix, 16).unwrap();
    if segments {
        ix.run_list = t.run_list;
    }
    let (mut h, tbl) = (t.h, t.tbl);
    h.set_logging(false);
    let mut path = Path::new();
    // The tree's rows: rids below, among and above the segments', some equal to a segment's under its key.
    let mut rows: Vec<(Vec<u8>, u64)> = Vec::new();
    let mut taken = BTreeSet::new();
    for _ in 0..ops {
        if r.below(5) == 0 && !rows.is_empty() {
            let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
            assert!(ix.delete(&mut h, tbl, &k, rid, &mut path).unwrap());
            m.get_mut(&k).unwrap().remove(&rid);
            continue;
        }
        let k = keys[(100 + r.below(200)) as usize].clone();
        let rid = match r.below(4) {
            0 => base_rid(&mut r, bases[0]),
            1 => base_rid(&mut r, bases[1]),
            _ => r.below(bases[1] + 3 * 4_096),
        };
        if !taken.insert((k.clone(), rid)) {
            continue;
        }
        assert!(ix.insert(&mut h, tbl, &k, rid, &mut path).unwrap());
        m.entry(k.clone()).or_default().insert(rid);
        rows.push((k, rid));
    }
    let mut probes: Vec<Vec<u8>> = keys.iter().step_by(3).cloned().collect();
    probes.extend([text(b""), text(b"absent"), text(b"zzzz"), vec![0xFF; 3]]);
    let what = format!("segments {segments}, {ops} operations, fill {fill:#x}");
    let mut seen = check_walks(&ix, &h, &union(&m, &segs), &probes, &mut r, &what);
    // Half the tree's rows deleted: keys the tree no longer holds, and keys left in the segments alone.
    for _ in 0..rows.len() / 2 {
        let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
        assert!(ix.delete(&mut h, tbl, &k, rid, &mut path).unwrap());
        m.get_mut(&k).unwrap().remove(&rid);
    }
    seen.extend(check_walks(&ix, &h, &union(&m, &segs), &probes, &mut r, &format!("{what}, half deleted")));
    seen
}

/// A rid inside the segment based at `base` (the segments' rids start there).
fn base_rid(r: &mut XorShift, base: u64) -> u64 {
    base + r.below(1_600)
}

#[test]
fn walks_against_the_model() {
    for (segments, ops, seed) in [(true, 6_000, 0x5A11), (false, 6_000, 0x5A12), (true, 0, 0x5A13), (false, 0, 0x5A14)]
    {
        let a = case(FILLS[0], segments, ops, seed);
        let b = case(FILLS[1], segments, ops, seed);
        assert!(a == b, "the fill patterns give different walks");
        assert_eq!(a.is_empty(), !segments && ops == 0);
    }
}

#[test]
fn walk_of_an_empty_index() {
    let mut h = VecHost::new(FILLS[0]);
    let _ = new_root(&mut h).unwrap();
    let ix = Secondary::create(1, 8).unwrap();
    for dir in [Dir::Fwd, Dir::Rev] {
        assert!(walk(&ix, &h, dir, Start::End, u64::MAX).is_empty());
        assert!(walk(&ix, &h, dir, Start::Key(&text(b"a")), u64::MAX).is_empty());
        assert!(walk(&ix, &h, dir, Start::After(&text(b"a"), 3), u64::MAX).is_empty());
    }
    assert_eq!(ends(&ix, &h), (None, None));
}

// ---------------------------------------------------------------- ends with nulls, and many segments

/// The smallest and largest keys whose first part is not null, with the keys in the tree or in one uploaded
/// segment: equal either way, for each kind whose smallest non-null value encodes as the tag 01 followed by
/// zero bytes only (a segment's count pads keys with zero bytes, as its key areas do).
#[test]
fn ends_skip_nulls_in_the_tree_and_in_segments() {
    use native_table::table::cell::{enc_key, Val};
    use native_table::table::layout::{LeafInfo, Ty};
    fn enc(ty: Ty, v: Val<'_>) -> Vec<u8> {
        let l = LeafInfo { ordinal: 0, id: 0, ty, nullable: true, key: true, path: 0, added: 0, dropped: 0 };
        let mut s = VecSink::default();
        enc_key(&mut s, &l, &v).unwrap();
        s.0
    }
    let part = |kind| [KeyPart { leaf: 0, kind, nullable: true }];
    let cases: [([KeyPart; 1], Ty, [Val<'static>; 3]); 7] = [
        (part(KeyKind::Bool), Ty::Bool, [Val::Null, Val::Bool(false), Val::Bool(true)]),
        // A smallest value of 40 zero bytes, a key longer than the tree's W.
        (part(KeyKind::Fixed(40)), Ty::Fixed(40), [Val::Null, Val::Bytes(&[0; 40]), Val::Bytes(&[7; 40])]),
        (part(KeyKind::U32), Ty::U(4), [Val::Null, Val::U(0), Val::U(7)]),
        (part(KeyKind::U64), Ty::U(8), [Val::Null, Val::U(0), Val::U(1 << 40)]),
        (part(KeyKind::Nat), Ty::Nat, [Val::Null, Val::U(0), Val::U(5)]),
        (part(KeyKind::Text), Ty::Text, [Val::Null, Val::Bytes(b""), Val::Bytes(b"a")]),
        (part(KeyKind::Unit), Ty::Unit, [Val::Null, Val::Unit, Val::Unit]),
    ];
    for (parts, ty, values) in &cases {
        let spec = KeySpec { parts };
        let keys: BTreeSet<Vec<u8>> = values.iter().map(|&v| enc(*ty, v)).collect();
        let held: Vec<&Vec<u8>> = keys.iter().filter(|k| k[0] != 0).collect();
        let want = (held.first().map(|k| k.to_vec()), held.last().map(|k| k.to_vec()));
        for in_tree in [true, false] {
            let mut t = TTable::new(FILLS[0]);
            let items: Items = keys.iter().enumerate().map(|(i, k)| (k.clone(), i as u64, false)).collect();
            let mut ix = Secondary::create(t.ix, 8).unwrap();
            if !in_tree {
                t.add_loaded(&VecItems::sorted(items.clone()), 1 << 40, 1);
                ix.run_list = t.run_list;
            }
            let (mut h, tbl) = (t.h, t.tbl);
            if in_tree {
                let mut path = Path::new();
                for (k, rid, _) in &items {
                    assert!(ix.insert(&mut h, tbl, k, rid + 1, &mut path).unwrap());
                }
            }
            let (mut out, mut scratch) = (vec![0u8; 64], vec![0u8; 64]);
            let min = ix.min_key(&h, &spec, &mut out, &mut scratch).unwrap().map(|n| out[..n].to_vec());
            let max = ix.max_key(&h, &spec, &mut out, &mut scratch).unwrap().map(|n| out[..n].to_vec());
            assert_eq!((min, max), want, "{:?}, keys in the tree {in_tree}", parts[0].kind);
        }
    }
}

/// An index of `segments` uploaded segments, each holding 50 keys of its own and `per` rows of `hot`, the
/// first `holders` of them; with every page of the host watched.
fn segments_holding(segments: u64, holders: u64, per: u64, hot: &[u8]) -> (Secondary, VecHost) {
    let mut t = TTable::new(FILLS[0]);
    for j in 0..segments {
        let mut items: Items = (0..50u64).map(|i| (text(format!("k{j}-{i}").as_bytes()), i, false)).collect();
        if j < holders {
            items.extend((0..per).map(|i| (hot.to_vec(), 100 + i, false)));
        }
        t.add_loaded(&VecItems::sorted(items), (1 << 40) + j * 4_096, 1);
    }
    let mut ix = Secondary::create(t.ix, 16).unwrap();
    ix.run_list = t.run_list;
    let mut h = t.h;
    h.set_logging(false);
    let pages = h.memory().len() as u64 / 65_536;
    h.watched = (1..=pages).map(|p| p * 65_536).collect();
    (ix, h)
}

/// The host reads of a walk of `rows` rows from `Start::Key(key)`.
fn walk_reads(ix: &Secondary, h: &VecHost, key: &[u8], rows: u64) -> u64 {
    let (mut k, mut scratch) = (vec![0u8; 4_096], vec![0u8; 8_192]);
    h.watched_reads.set(0);
    let n = ix.walk(h, &SPEC, Dir::Fwd, Start::Key(key), rows, &mut k, &mut scratch, |_, _| Ok(())).unwrap();
    assert_eq!(n, rows);
    h.watched_reads.get()
}

/// A walk's reads per row of one key do not grow with the uploaded segments: each segment holding the key is
/// read again only for the rows it hands out. Reads per row are the reads of 400 rows past those of 200, for a
/// key in one segment and spread over all of them, against one segment's.
#[test]
fn walk_reads_per_row_stay_flat_over_segments() {
    let hot = text(b"hot");
    for spread in [false, true] {
        let per_row = |segments: u64| {
            let holders = if spread { segments } else { 1 };
            let per = 400 / holders;
            let (ix, h) = segments_holding(segments, holders, per, &hot);
            let rows = per * holders;
            let half = rows / 2;
            (walk_reads(&ix, &h, &hot, rows) - walk_reads(&ix, &h, &hot, half)) as f64 / (rows - half) as f64
        };
        let one = per_row(1);
        for segments in [4, 16, 64] {
            let got = per_row(segments);
            assert!(got <= one + 1.0, "spread {spread}: {segments} segments read {got:.1} per row, one {one:.1}");
        }
    }
}

/// One key held by more segments than a walk keeps the next rows of, with rows that interleave across them
/// and with the tree's: every walk against the model.
#[test]
fn walk_over_a_key_in_many_segments() {
    let hot = text(b"hot");
    let mut r = XorShift(0x0A11);
    let base = 1u64 << 40;
    let mut t = TTable::new(FILLS[1]);
    let mut m = Model::new();
    // Segment j's rows of the key: rids base + j + 128·i, so the segments' rows interleave in rid order.
    for j in 0..90u64 {
        let mut items: Items = Vec::new();
        for i in 0..1 + (j * 7) % 12 {
            items.push((hot.clone(), j + 128 * i, false));
            m.entry(hot.clone()).or_default().insert(base + j + 128 * i);
        }
        for i in 0..1 + r.below(4) {
            let k = text(format!("k{}", r.below(40)).as_bytes());
            items.push((k.clone(), 100_000 + j * 8 + i, false));
            m.entry(k).or_default().insert(base + 100_000 + j * 8 + i);
        }
        t.add_loaded(&VecItems::sorted(items), base, 30);
    }
    let mut ix = Secondary::create(t.ix, 16).unwrap();
    ix.run_list = t.run_list;
    let (mut h, tbl) = (t.h, t.tbl);
    h.set_logging(false);
    let mut path = Path::new();
    // The tree's rows of the key, between the segments'.
    for i in 0..40u64 {
        let rid = base + 100 + 128 * i;
        assert!(ix.insert(&mut h, tbl, &hot, rid, &mut path).unwrap());
        m.entry(hot.clone()).or_default().insert(rid);
    }
    let probes = vec![hot.clone(), text(b"k1"), text(b"k5"), text(b"a"), text(b"z")];
    check_walks(&ix, &h, &m, &probes, &mut r, "a key in 90 segments");
}
