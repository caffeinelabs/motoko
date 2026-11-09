//! Secondary index keys of any length over the tree and the uploaded segments, against a model of the rows.
//!
//! A run's count of a key (`Run::count_key`) equals the items' count for every key it holds and keys it does
//! not: text keys of up to 30,000 bytes, short and long beside each other, sharing long starts (so the run's shared prefix is longer
//! than the key area), one a prefix of another, at several (W, V), with posting chunks; and key runs of keys
//! of 5,000 bytes, items stepping by one in their last 8 bytes.
//!
//! The walk, the ends and the counts of an index whose tree and two uploaded segments hold such keys: the
//! walks' buffers start small and grow to the bytes each `Scratch` names, a walk resumed after the last row it
//! handed out, and every row comes out once, in the model's order; every key's count through the tree and the
//! segments is the model's.

use std::collections::{BTreeMap, BTreeSet};

use native_table::index::secondary::{Secondary, Start};
use native_table::index::{Run, RunImage};
use native_table::key::{enc_bytes, KeyKind, KeyPart, KeySpec};
use native_table::tree::{Dir, Path};
use native_table::Fault;
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

/// Texts of 0 to 30 letters, and long ones: starts of one 30,000-byte text (one a prefix of another), with
/// their last byte changed, with a zero byte, and with short tails after a common 6,000-byte start.
fn texts(r: &mut XorShift) -> Vec<Vec<u8>> {
    let base = base();
    let mut out = BTreeSet::new();
    while out.len() < 60 {
        let len = r.below(31) as usize;
        out.insert((0..len).map(|_| [b'a', b'b', 0][r.below(3) as usize]).collect::<Vec<u8>>());
    }
    for len in [4_090usize, 4_200, 5_000, 9_000, 30_000] {
        let (mut last, mut zero) = (base[..len].to_vec(), base[..len].to_vec());
        *last.last_mut().unwrap() ^= 0x20;
        zero[len / 3] = 0;
        out.extend([base[..len].to_vec(), last, zero]);
    }
    for tail in ["", "a", "b", "ab", "abcdefghijklmnopqrstuvwxyz"] {
        out.insert([&base[..6_000], tail.as_bytes()].concat());
    }
    out.into_iter().map(|t| text(&t)).collect()
}

/// The long text the long keys start from (no zero byte, so a start of it encodes as itself).
fn base() -> Vec<u8> {
    (0..30_000u32).map(|i| b'a' + (i * 7 % 23) as u8).collect()
}

/// Whether a key starts with the first 6,000 bytes of `base`.
fn shares_6000(k: &[u8]) -> bool {
    k.starts_with(&base()[..6_000])
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

/// `run`'s count of each key against the items' count.
fn check_counts<I: RunImage>(run: &Run<I>, items: &Items, absent: &[Vec<u8>], what: &str) {
    let mut want: BTreeMap<&[u8], i64> = BTreeMap::new();
    for (k, _, _) in items {
        *want.entry(k).or_default() += 1;
    }
    let keys = want.keys().copied().chain(absent.iter().map(|k| k.as_slice()));
    for k in keys {
        let n = want.get(k).copied().unwrap_or(0);
        assert_eq!(run.count_key(k).unwrap(), n, "{what}: {} bytes", k.len());
    }
}

#[test]
fn run_counts_of_long_keys() {
    let mut r = XorShift(0x1_0C6E);
    let keys = texts(&mut r);
    // Every key, of one to 40 rows (posting chunks); and the long keys alone, which share a start longer
    // than W.
    let mut items: Items = Vec::new();
    let mut rid = 1u64;
    for k in &keys {
        for _ in 0..1 + r.below(40) {
            rid += 1 + r.below(3);
            items.push((k.clone(), rid, false));
        }
    }
    let long: Items = items.iter().filter(|x| x.0.len() > 6_000).cloned().collect();
    // Keys sharing their first 6,000 bytes: the run stores that prefix, and the rest of each fits W.
    let tails: Items = items.iter().filter(|x| shares_6000(&x.0)).cloned().collect();
    let mut absent: Vec<Vec<u8>> = [&b"zz"[..], b"", &keys[keys.len() - 1][1..]].iter().map(|t| text(t)).collect();
    absent.extend([[&tails[0].0[..6_000], b"b\0\0"].concat(), tails[0].0[..5_990].to_vec()]);
    let mut prefixes = Vec::new();
    for (what, set) in [("every key", &items), ("the long keys", &long), ("keys sharing 6,000 bytes", &tails)] {
        let set = VecItems::sorted(set.clone());
        for force in [None, Some((8, 0)), Some((16, 16)), Some((64, 64))] {
            let b = build(&set, &params(false, 0, force), FILLS[0], false).unwrap();
            prefixes.push(b.plan.header.prefix);
            let what = format!("{what}, (W, V) {force:?}, prefix {}", b.plan.header.prefix);
            check_counts(&b.open(), &set.0, &absent, &what);
        }
    }
    assert!(prefixes.iter().any(|&p| p >= 6_000), "a run stores a prefix of 6,000 bytes: {prefixes:?}");
    // Keys of 5,000 bytes ending in an 8-byte field: lone rows stepping by one in key and rid form key runs,
    // among keys of three rows.
    let start = vec![b'k'; 4_992];
    let key = |f: u64| [start.as_slice(), &f.to_be_bytes()].concat();
    let mut items: Items = Vec::new();
    let (mut f, mut rid) = (r.below(5), 100u64);
    while items.len() < 600 {
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
    let absent: Vec<Vec<u8>> = (0..60).map(|_| key(r.below(f + 10))).chain([vec![b'k'; 5_000]]).collect();
    for v in [8, 16, 32] {
        let b = build(&items, &params(false, 8, Some((16, v))), FILLS[1], false).unwrap();
        assert!(b.plan.header.flags & native_table::index::run::F_KEY_RUNS != 0, "key runs formed");
        check_counts(&b.open(), &items.0, &absent, &format!("key runs, V {v}"));
    }
}

// ---------------------------------------------------------------- the walk over the tree and the segments

/// The rows of the tree and the segments: key → rids.
type Model = BTreeMap<Vec<u8>, BTreeSet<u64>>;

/// A row as a walk hands it out.
type Row = (Vec<u8>, u64);

/// Each key's count.
type Counts = BTreeMap<Vec<u8>, i128>;

/// The model's rows in the walk's order from `start`.
fn model_walk(m: &Model, dir: Dir, start: Start<'_>) -> Vec<Row> {
    let mut keys: Vec<&Vec<u8>> = m.iter().filter(|(_, rids)| !rids.is_empty()).map(|(k, _)| k).collect();
    if dir == Dir::Rev {
        keys.reverse();
    }
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

/// A walk from `start`, at most `limit` rows, through buffers of `size` bytes each that grow to the bytes each
/// `Scratch` names, resumed after the last row handed out; the rows and the times the buffers grew.
fn walk(ix: &Secondary, h: &VecHost, dir: Dir, start: Start<'_>, limit: u64, size: usize) -> (Vec<Row>, u64) {
    let (mut out, mut size, mut grew): (Vec<Row>, usize, u64) = (Vec::new(), size, 0);
    loop {
        let (mut key, mut scratch) = (vec![0u8; size], vec![0u8; 2 * size]);
        let last = out.last().cloned();
        let from = match &last {
            Some((k, rid)) => Start::After(k, *rid),
            None => start,
        };
        let mut got = Vec::new();
        let r = ix.walk(h, &SPEC, dir, from, limit - out.len() as u64, &mut key, &mut scratch, |k, rid| {
            got.push((k.to_vec(), rid));
            Ok(())
        });
        out.extend(got);
        match r {
            Ok(_) => return (out, grew),
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size, "a need of {need} bytes for buffers of {size}");
                (size, grew) = (need as usize, grew + 1);
            }
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// The smallest and largest keys, through buffers grown as the walk's.
fn ends(ix: &Secondary, h: &VecHost) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let end = |largest: bool| {
        let mut size = 8usize;
        loop {
            let (mut out, mut scratch) = (vec![0u8; size], vec![0u8; size]);
            let r = if largest {
                ix.max_key(h, &SPEC, &mut out, &mut scratch)
            } else {
                ix.min_key(h, &SPEC, &mut out, &mut scratch)
            };
            match r {
                Ok(x) => return x.map(|n| out[..n].to_vec()),
                Err(Fault::Scratch { need }) => {
                    assert!(need as usize > size);
                    size = need as usize;
                }
                Err(e) => panic!("{e:?}"),
            }
        }
    };
    (end(false), end(true))
}

/// Every walk and the ends against the model `m`, and every key's count against `counts` (a row the tree and a
/// segment both hold counts in each, and is walked once).
fn check(ix: &Secondary, h: &VecHost, m: &Model, counts: &Counts, probes: &[Vec<u8>], r: &mut XorShift, what: &str) {
    for dir in [Dir::Fwd, Dir::Rev] {
        let want = model_walk(m, dir, Start::End);
        let (got, grew) = walk(ix, h, dir, Start::End, u64::MAX, 8);
        assert!(got == want, "{what}, {dir:?}: the walk ({} rows, {} expected)", got.len(), want.len());
        assert!(grew > 0 || want.is_empty(), "{what}: the buffers grew");
        // Large enough from the start: no growth.
        assert_eq!(walk(ix, h, dir, Start::End, u64::MAX, 64_000), (want.clone(), 0), "{what}, {dir:?}");
        // In pieces of a few rows, each resumed after the last row of the one before.
        let mut all = walk(ix, h, dir, Start::End, 3, 8).0;
        while let Some((k, rid)) = all.last().cloned() {
            let more = walk(ix, h, dir, Start::After(&k, rid), 3, 8).0;
            if more.is_empty() {
                break;
            }
            all.extend(more);
        }
        assert!(all == want, "{what}, {dir:?}: the walk in pieces");
        for k in probes {
            let limit = 1 + r.below(30);
            let want = model_walk(m, dir, Start::Key(k));
            let got = walk(ix, h, dir, Start::Key(k), limit, 8).0;
            assert!(got[..] == want[..want.len().min(limit as usize)], "{what}, {dir:?}: from {} bytes", k.len());
        }
    }
    let held: Vec<&Vec<u8>> = m.iter().filter(|(_, rids)| !rids.is_empty()).map(|(k, _)| k).collect();
    assert_eq!(ends(ix, h), (held.first().map(|k| k.to_vec()), held.last().map(|k| k.to_vec())), "{what}: ends");
    for k in m.keys().chain(probes) {
        let n = counts.get(k).copied().unwrap_or(0);
        assert_eq!(ix.count(h, k).unwrap(), n, "{what}: the count of {} bytes", k.len());
    }
}

/// One case under fill `fill`: three uploaded segments (when `segments`), the first of every key, the second of
/// the long keys only, the third of keys sharing their first 6,000 bytes; a tree of random inserts and deletes; every check before and after deleting half the
/// tree's rows, and with the tree emptied.
fn case(fill: u8, segments: bool, seed: u64) {
    let mut r = XorShift(seed);
    let keys = texts(&mut r);
    let long: Vec<Vec<u8>> = keys.iter().filter(|k| k.len() > 6_000).cloned().collect();
    let tails: Vec<Vec<u8>> = long.iter().filter(|k| shares_6000(k)).cloned().collect();
    let mut t = TTable::new(fill);
    let (mut segs, mut m) = (Model::new(), Model::new());
    let bases = [4_096 * 1_000, 4_096 * 1_002];
    if segments {
        for (base, from) in bases.into_iter().zip([&keys, &long]).chain([(4_096 * 1_004, &tails)]) {
            let mut items: Items =
                (0..300).map(|_| (from[r.below(from.len() as u64) as usize].clone(), 0, false)).collect();
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
    let mut rows: Vec<(Vec<u8>, u64)> = Vec::new();
    for _ in 0..1_500 {
        if r.below(5) == 0 && !rows.is_empty() {
            let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
            assert!(ix.delete(&mut h, tbl, &k, rid, &mut path).unwrap());
            m.get_mut(&k).unwrap().remove(&rid);
            continue;
        }
        let k = keys[r.below(keys.len() as u64) as usize].clone();
        let rid = if r.below(3) == 0 { bases[0] + r.below(600) } else { r.below(bases[1] + 3 * 4_096) };
        if m.get(&k).is_some_and(|x| x.contains(&rid)) {
            continue;
        }
        assert!(ix.insert(&mut h, tbl, &k, rid, &mut path).unwrap());
        m.entry(k.clone()).or_default().insert(rid);
        rows.push((k, rid));
    }
    let union = |m: &Model| {
        let mut u = segs.clone();
        for (k, rids) in m {
            u.entry(k.clone()).or_default().extend(rids);
        }
        u
    };
    let counts = |m: &Model| {
        let mut c = Counts::new();
        for (k, rids) in m.iter().chain(&segs) {
            *c.entry(k.clone()).or_default() += rids.len() as i128;
        }
        c
    };
    let mut probes: Vec<Vec<u8>> = keys.iter().step_by(4).cloned().collect();
    probes.extend([text(b""), text(b"zzzz"), long[0][..long[0].len() - 2].to_vec(), [&long[1][..], b"\x01"].concat()]);
    let what = format!("segments {segments}, fill {fill:#x}");
    check(&ix, &h, &union(&m), &counts(&m), &probes, &mut r, &what);
    for _ in 0..rows.len() / 2 {
        let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
        assert!(ix.delete(&mut h, tbl, &k, rid, &mut path).unwrap());
        m.get_mut(&k).unwrap().remove(&rid);
    }
    check(&ix, &h, &union(&m), &counts(&m), &probes, &mut r, &format!("{what}, half deleted"));
    for (k, rid) in rows.drain(..) {
        assert!(ix.delete(&mut h, tbl, &k, rid, &mut path).unwrap());
        m.get_mut(&k).unwrap().remove(&rid);
    }
    assert_eq!(ix.root, 0);
    check(&ix, &h, &union(&m), &counts(&m), &probes, &mut r, &format!("{what}, the tree emptied"));
}

#[test]
fn walks_and_counts_of_long_keys_over_the_tree_and_segments() {
    for (fill, segments, seed) in [(FILLS[0], true, 0x10_0A), (FILLS[1], true, 0x10_0B), (FILLS[0], false, 0x10_0C)] {
        case(fill, segments, seed);
    }
}

// ---------------------------------------------------------------- huge numbers read in windows

/// A canonical (signed) LEB128 form of `n` bytes, its 7-bit groups drawn from `r` (the low ones zero when
/// `zeros`); a negative one when `neg`.
fn leb(r: &mut XorShift, n: usize, signed: bool, neg: bool, zeros: usize) -> Vec<u8> {
    let mut out: Vec<u8> = (0..n).map(|i| if i < zeros { 0x80 } else { 0x80 | r.below(128) as u8 }).collect();
    out[n - 1] = match (signed, neg) {
        (false, _) => 1 + r.below(127) as u8,
        (true, false) => 1 + r.below(0x3F) as u8,
        (true, true) => 0x40 + r.below(0x3F) as u8,
    };
    out
}

/// `enc_int_leb_read` gives `enc_int_leb`'s bytes, its form read through windows: Nats and Ints, positive and
/// negative, short and of 20,000 bytes, with their low groups zero (where a negative's negation starts) or not.
#[test]
fn leb_read_equals_enc_int_leb() {
    use native_table::key::{enc_int_leb, enc_int_leb_read};
    use native_table::ImageId;
    let mut r = XorShift(0x1EB);
    for n in [1usize, 2, 9, 10, 300, 5_000, 20_000] {
        for (signed, neg) in [(false, false), (true, false), (true, true)] {
            for zeros in [0, n / 2, n - 1] {
                let form = leb(&mut r, n, signed, neg, zeros);
                let mut want = VecSink::default();
                enc_int_leb(&mut want, &form, signed, ImageId::Runtime).unwrap();
                let mut got = VecSink::default();
                let read = |at: u64, out: &mut [u8]| {
                    let at = at as usize;
                    out.copy_from_slice(&form[at..at + out.len()]);
                    Ok(())
                };
                enc_int_leb_read(&mut got, n as u64, signed, ImageId::Runtime, read).unwrap();
                assert!(got.0 == want.0, "{n} bytes, signed {signed}, negative {neg}, {zeros} low groups zero");
            }
        }
    }
}

/// `LebFacts` gives `enc_int_leb`'s bytes from windows of the form alone: a negative's lowest non-zero byte
/// found in windows of any size, the facts from the form's last bytes, and the magnitude's key bytes written
/// in runs of any size, each from the form's bytes its span names. Nats and Ints, positive and negative,
/// short and of 20,000 bytes, with their low groups zero (where a negative's negation starts) or not.
#[test]
fn leb_facts_equal_enc_int_leb() {
    use native_table::key::{enc_int_leb, leb_low, LebFacts, LEB_TOP};
    use native_table::ImageId;
    let mut r = XorShift(0x1EF);
    for n in [1usize, 2, 9, 10, 17, 18, 24, 25, 300, 5_000, 20_000] {
        for (signed, neg) in [(false, false), (true, false), (true, true)] {
            for zeros in [0, n / 2, n - 1] {
                let form = leb(&mut r, n, signed, neg, zeros);
                let mut want = VecSink::default();
                enc_int_leb(&mut want, &form, signed, ImageId::Runtime).unwrap();
                let (mut low, mut at) = (None, 0usize);
                while neg && low.is_none() {
                    let w = (1 + r.below(800) as usize).min(n - at);
                    low = leb_low(&form[at..at + w], at as u64);
                    at += w;
                }
                let top = &form[n - n.min(LEB_TOP)..];
                let f = LebFacts::new(n as u64, signed, low.unwrap_or(0), top, ImageId::Runtime).unwrap();
                let mut got = VecSink::default();
                f.head(&mut got).unwrap();
                let mut hi = f.mag_len;
                while hi > 0 {
                    let lo = hi.saturating_sub(1 + r.below(700));
                    let (a, b) = f.span(lo, hi);
                    let mut out = vec![0u8; (hi - lo) as usize];
                    f.key_bytes(&form[a as usize..b as usize], a, lo, &mut out).unwrap();
                    got.0.extend_from_slice(&out);
                    hi = lo;
                }
                assert!(got.0 == want.0, "{n} bytes, signed {signed}, negative {neg}, {zeros} low groups zero");
            }
        }
    }
}
