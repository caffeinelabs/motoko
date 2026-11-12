//! The secondary index (`index::secondary`) against a model of its rows: random inserts and deletes of rows
//! under text keys, from a few keys to nearly all distinct, shorter and longer than the tree's key area W,
//! with rising rids, older rids and rows taken or dropped twice. Each case runs under both fill patterns and
//! must leave the same entries and give the same answers. Every few operations and at the end the tree is
//! checked whole: each key's row entries in rid order, then its one count entry holding their number, no
//! count entry for a key without rows, every reachable node of the tree's form and counted, and each internal
//! node's rows under each child those of the child's subtree; every key's count, the rows by every first part of
//! a key's bytes and the group walk equal the model's; deleting every row empties the tree and its key log.
//!
//! The key log against the tree: its free chunks listed by size class and linked both ways, its blocks runs of
//! chunks holding exactly the records the tree's entries name and the free ones, under churn of one row's key
//! and under random writes of keys up to beyond a log block, cut back to a few rows.

use std::collections::{BTreeMap, BTreeSet};

use native_table::area::new_root;
use native_table::index::area_nodes::{AreaView, KeyLog};
use native_table::index::secondary::Secondary;
use native_table::key::probe::{is_count, is_ovf, payload, payload_to_count, KeyProbe, ResolveKey};
use native_table::key::{enc_bytes, KeyKind, KeyPart, KeySpec};
use native_table::tree::node::{child, count, is_leaf, ko, rows};
use native_table::tree::{Cursor, Fwd, Path, Shape};
use native_table::{ImageId, NodeRead, PageHost};
use nt_testhost::{VecHost, VecSink, XorShift, FILLS};

/// The index's key spec: one Text part.
const PARTS: [KeyPart; 1] = [KeyPart { leaf: 0, kind: KeyKind::Text, nullable: false }];
const SPEC: KeySpec<'static> = KeySpec { parts: &PARTS };

/// The rows the tree holds: key → rids.
type Model = BTreeMap<Vec<u8>, BTreeSet<u64>>;

/// A text key's encoding.
fn text(t: &[u8]) -> Vec<u8> {
    let mut s = VecSink::default();
    enc_bytes(&mut s, t).unwrap();
    s.0
}

/// The keys a case draws from: `n` texts of 0 to `max` letters of a small alphabet (with zero bytes, which
/// the encoding escapes).
fn pool(r: &mut XorShift, n: usize, max: u64) -> Vec<Vec<u8>> {
    let mut keys = BTreeSet::new();
    while keys.len() < n {
        let len = r.below(max + 1) as usize;
        let t: Vec<u8> = (0..len).map(|_| [b'a', b'b', b'c', 0][r.below(4) as usize]).collect();
        keys.insert(text(&t));
    }
    keys.into_iter().collect()
}

/// One test table: its memory and the index.
struct Side {
    h: VecHost,
    tbl: u64,
    ix: Secondary,
    path: Path,
}

impl Side {
    fn new(fill: u8, w: u64) -> Side {
        let mut h = VecHost::new(fill);
        h.set_logging(false);
        let tbl = new_root(&mut h).unwrap();
        Side { h, tbl, ix: Secondary::create(1, w).unwrap(), path: Path::new() }
    }

    fn insert(&mut self, key: &[u8], rid: u64) -> bool {
        self.ix.insert(&mut self.h, self.tbl, key, rid, &mut self.path).unwrap()
    }

    fn delete(&mut self, key: &[u8], rid: u64) -> bool {
        self.ix.delete(&mut self.h, self.tbl, key, rid, &mut self.path).unwrap()
    }
}

/// Every entry of the tree in order: its key (whole, as the spec ends it), whether it is a count entry, and
/// its rid or count.
fn entries<H: PageHost>(h: &H, ix: &Secondary) -> Vec<(Vec<u8>, bool, i64)> {
    let s = ix.shape;
    let v = AreaView::new(h, s, ix.log);
    let mut out = Vec::new();
    let mut c: Cursor<Fwd> = Cursor::seek(s, &v, ix.root, std::ops::Bound::<&KeyProbe<'_>>::Unbounded).unwrap();
    while let Some((n, i)) = c.peek(s, &v).unwrap() {
        c.advance(s, &v, n, i).unwrap();
        let at = ko(s, &s.shape(is_leaf(&v, n).unwrap()), i);
        let part = v.bytes(n, at, s.w + 8).unwrap().to_vec();
        let field = u64::from_be_bytes(part[s.w as usize..].try_into().unwrap());
        let mut key = part[..s.w as usize].to_vec();
        let mut value = payload(field);
        if is_ovf(field) {
            let (p, len) = v.record(field).unwrap();
            value = p;
            let mut rest = vec![0u8; (len - s.w) as usize];
            let mut done = 0;
            while done < rest.len() {
                done += v.key_piece(field, s.w + done as u64, &mut rest[done..]).unwrap();
            }
            key.extend_from_slice(&rest);
        }
        let end = SPEC.end(&key, ImageId::Runtime).unwrap() as usize;
        key.truncate(end);
        let n = if is_count(field) { payload_to_count(value) } else { value as i64 };
        out.push((key, is_count(field), n));
    }
    out
}

/// The nodes reachable from the root, each checked for the tree's layout, and the tree's levels.
fn reachable<H: PageHost>(h: &H, ix: &Secondary) -> (u64, u64) {
    let s = ix.shape;
    let v = AreaView::new(h, s, ix.log);
    let mut stack = if ix.root == 0 { vec![] } else { vec![(ix.root, 1u64)] };
    let (mut n, mut levels) = (0, 0);
    while let Some((x, depth)) = stack.pop() {
        let b = h.get(native_table::Region::Block(x)).unwrap();
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), s.layout(), "node {x}: layout");
        n += 1;
        levels = levels.max(depth);
        if !is_leaf(&v, x).unwrap() {
            let c = count(s, &v, x).unwrap();
            assert!(c >= 1);
            for j in 0..=c {
                stack.push((child(s, &v, x, j).unwrap(), depth + 1));
            }
        }
    }
    (n, levels)
}

/// The row entries of node x's subtree, each internal node's rows under each child checked against them.
fn subtree_rows<H: PageHost>(v: &AreaView<'_, H>, x: u64) -> u64 {
    let s = v.shape;
    let (c, leaf) = (count(s, v, x).unwrap(), is_leaf(v, x).unwrap());
    let field = |i| u64::from_be_bytes(v.bytes(x, ko(s, &s.shape(leaf), i) + s.w, 8).unwrap().try_into().unwrap());
    let mut r = (0..c).filter(|&i| !is_count(field(i))).count() as u64;
    if !leaf {
        for j in 0..=c {
            let under = subtree_rows(v, child(s, v, x, j).unwrap());
            assert_eq!(rows(s, v, x, j).unwrap(), under, "node {x}: the rows under child {j}");
            r += under;
        }
    }
    r
}

/// The rows the tree holds, its internal nodes' rows checked.
fn tree_rows(side: &Side) -> u64 {
    let v = AreaView::new(&side.h, side.ix.shape, side.ix.log);
    if side.ix.root == 0 {
        0
    } else {
        subtree_rows(&v, side.ix.root)
    }
}

/// The tree whole against the model: each key's rows in rid order, then its count entry; the node count; the
/// rows under each child of each internal node. Returns the tree's levels.
fn check_tree(side: &Side, m: &Model, what: &str) -> u64 {
    let got = entries(&side.h, &side.ix);
    let mut want = Vec::new();
    for (k, rids) in m {
        if rids.is_empty() {
            continue;
        }
        for &r in rids {
            want.push((k.clone(), false, r as i64));
        }
        want.push((k.clone(), true, rids.len() as i64));
    }
    assert_eq!(got.len(), want.len(), "{what}: entries");
    assert!(got == want, "{what}: the tree's entries differ from the model's");
    let (nodes, levels) = reachable(&side.h, &side.ix);
    assert_eq!(nodes, side.ix.nodes, "{what}: node count");
    assert_eq!(tree_rows(side), m.values().map(|r| r.len() as u64).sum::<u64>(), "{what}: rows");
    levels
}

/// Every key's count and the group walk, against the model's counts.
fn check_counts(side: &Side, m: &Model, probes: &[Vec<u8>], what: &str) {
    let mut want: BTreeMap<Vec<u8>, i128> = BTreeMap::new();
    for (k, rids) in m {
        if !rids.is_empty() {
            *want.entry(k.clone()).or_default() += rids.len() as i128;
        }
    }
    for k in probes {
        let got = side.ix.count(&side.h, k).unwrap();
        assert_eq!(got, want.get(k).copied().unwrap_or(0), "{what}: count of {}", nt_testhost::hex(k));
    }
    // Rows by the first bytes of their keys: some keys cut at every length, and bytes FF, which no key starts with.
    let cuts = probes.iter().step_by(probes.len() / 40 + 1).flat_map(|k| (0..=k.len()).map(|n| k[..n].to_vec()));
    for p in cuts.chain([vec![0xFF], vec![0xFF, 0xFF]]) {
        let rows: usize = m.range(p.clone()..).take_while(|(k, _)| k.starts_with(&p)).map(|(_, r)| r.len()).sum();
        let got = side.ix.count_prefix(&side.h, &mut p.clone()).unwrap();
        assert_eq!(got, rows as i128, "{what}: rows by {}", nt_testhost::hex(&p));
    }
    let mut groups = Vec::new();
    let mut out = vec![0u8; 4_096];
    let mut after: Option<Vec<u8>> = None;
    while let Some((len, c)) = side.ix.group_next(&side.h, &SPEC, after.as_deref(), &mut out).unwrap() {
        let k = out[..len].to_vec();
        if let Some(a) = &after {
            assert!(k > *a, "{what}: the walk went back");
        }
        groups.push((k.clone(), c));
        after = Some(k);
    }
    let want: Vec<(Vec<u8>, i128)> = want.into_iter().filter(|(_, c)| *c != 0).collect();
    assert!(
        groups == want,
        "{what}: the group walk differs from the model ({} groups, {} expected)",
        groups.len(),
        want.len()
    );
}

/// What a case left: the entries before the final deletes, the result of every operation, the key log's
/// blocks before the final deletes and the most levels the tree had.
type Outcome = (Vec<(Vec<u8>, bool, i64)>, Vec<bool>, u64, u64);

/// One case: `ops` random operations on keys from `keys`, rids rising from 1, then every row dropped.
fn case(fill: u8, w: u64, keys: &[Vec<u8>], ops: u64, seed: u64) -> Outcome {
    let mut r = XorShift(seed);
    let mut side = Side::new(fill, w);
    let mut m = Model::new();
    let mut rows: Vec<(Vec<u8>, u64)> = Vec::new();
    let mut results = Vec::new();
    let mut next = 1u64;
    let mut levels = 0;
    let what = |step: u64| format!("W {w}, {} keys, seed {seed}, fill {fill:#x}, step {step}", keys.len());
    for step in 0..ops {
        let x = r.below(100);
        let (key, rid, insert, expect) = if x < 60 || rows.is_empty() {
            // A new row, its rid above every other.
            next += 1 + r.below(3);
            (keys[r.below(keys.len() as u64) as usize].clone(), next, true, true)
        } else if x < 66 {
            // An older rid, under any key: new unless the model holds it.
            let k = keys[r.below(keys.len() as u64) as usize].clone();
            let rid = 1 + r.below(next);
            let held = m.get(&k).is_some_and(|s| s.contains(&rid));
            (k, rid, true, !held)
        } else if x < 70 {
            // A row the tree holds, taken again.
            let (k, rid) = rows[r.below(rows.len() as u64) as usize].clone();
            (k, rid, true, false)
        } else if x < 96 {
            // A row the tree holds, dropped.
            let i = r.below(rows.len() as u64) as usize;
            let (k, rid) = rows[i].clone();
            (k, rid, false, true)
        } else {
            // A row the tree does not hold, dropped.
            let k = keys[r.below(keys.len() as u64) as usize].clone();
            let rid = next + 1 + r.below(10);
            (k, rid, false, false)
        };
        let got = if insert { side.insert(&key, rid) } else { side.delete(&key, rid) };
        assert_eq!(got, expect, "{}: {} of {rid}", what(step), if insert { "insert" } else { "delete" });
        results.push(got);
        if got {
            let set = m.entry(key.clone()).or_default();
            if insert {
                set.insert(rid);
                rows.push((key.clone(), rid));
            } else {
                set.remove(&rid);
                let at = rows.iter().position(|(k, r)| *k == key && *r == rid).unwrap();
                rows.swap_remove(at);
            }
        }
        // The key's count after every operation.
        let want = m.get(&key).map_or(0, |s| s.len() as i128);
        assert_eq!(side.ix.count(&side.h, &key).unwrap(), want, "{}: count", what(step));
        if step % 251 == 0 {
            levels = levels.max(check_tree(&side, &m, &what(step)));
        }
    }
    levels = levels.max(check_tree(&side, &m, &what(ops)));
    check_counts(&side, &m, keys, &what(ops));
    let left = entries(&side.h, &side.ix);
    let blocks = side.ix.log.blocks;
    // Every row dropped, in random order: the tree ends empty, and so does its key log.
    while !rows.is_empty() {
        let i = r.below(rows.len() as u64) as usize;
        let (k, rid) = rows.swap_remove(i);
        assert!(side.delete(&k, rid));
        m.get_mut(&k).unwrap().remove(&rid);
        assert_eq!(side.ix.count(&side.h, &k).unwrap(), m[&k].len() as i128);
        if rows.len() % 101 == 0 {
            assert_eq!(tree_rows(&side), rows.len() as u64, "{}: rows while emptying", what(ops));
        }
    }
    assert_eq!((side.ix.root, side.ix.nodes), (0, 0), "{}: the emptied tree", what(ops));
    assert_eq!(side.ix.log, KeyLog::default(), "{}: the emptied tree's key log", what(ops));
    (left, results, blocks, levels)
}

#[test]
fn inserts_and_deletes_against_the_model() {
    let mut r = XorShift(0x5EC0_17DA);
    // (W, keys, longest text, operations): a few keys of many rows each, hundreds, and nearly all distinct;
    // keys up to 40 letters overflow W 8 and 16 into the key log; W 64 makes a tree of three levels.
    let cases: [(u64, usize, u64, u64); 7] = [
        (8, 3, 6, 3_000),
        (8, 300, 12, 4_000),
        (16, 40, 40, 4_000),
        (24, 2_000, 20, 4_000),
        (32, 5, 60, 3_000),
        (64, 3_000, 50, 6_000),
        (8, 20_000, 30, 4_000),
    ];
    for (i, &(w, n, max, ops)) in cases.iter().enumerate() {
        let keys = pool(&mut r, n, max);
        let seed = 0x1000 + i as u64;
        let a = case(FILLS[0], w, &keys, ops, seed);
        let b = case(FILLS[1], w, &keys, ops, seed);
        assert!(a == b, "W {w}, {n} keys: the fill patterns give different trees or answers");
        let longer = keys.iter().any(|k| k.len() as u64 > w);
        assert_eq!(a.2 > 0, longer, "W {w}, {n} keys: keys longer than W go through the key log");
        assert!(a.3 >= if w == 64 { 3 } else { 2 }, "W {w}, {n} keys: {} levels", a.3);
    }
}

// ---------------------------------------------------------------- the key log

/// The bytes of a record of a key of `len` bytes, longer than W.
fn record_bytes(w: u64, len: usize) -> u64 {
    16 + len as u64 - w
}

/// The records the tree's entries name: locator → the record's bytes.
fn records<H: PageHost>(h: &H, ix: &Secondary) -> BTreeMap<u64, u64> {
    let s = ix.shape;
    let v = AreaView::new(h, s, ix.log);
    let mut out = BTreeMap::new();
    let mut c: Cursor<Fwd> = Cursor::seek(s, &v, ix.root, std::ops::Bound::<&KeyProbe<'_>>::Unbounded).unwrap();
    while let Some((n, i)) = c.peek(s, &v).unwrap() {
        c.advance(s, &v, n, i).unwrap();
        let at = ko(s, &s.shape(is_leaf(&v, n).unwrap()), i);
        let part = v.bytes(n, at, s.w + 8).unwrap();
        let field = u64::from_be_bytes(part[s.w as usize..].try_into().unwrap());
        if is_ovf(field) {
            let (_, len) = v.record(field).unwrap();
            assert!(out.insert(payload(field), record_bytes(s.w, len as usize)).is_none(), "a record named twice");
        }
    }
    out
}

/// The size class of a free chunk of `s` bytes: eight per power of two from 32.
fn class(s: u64) -> u64 {
    let fl = 63 - u64::from(s.leading_zeros());
    (fl - 5) * 8 + ((s >> (fl - 3)) & 7)
}

/// The key log against the tree. Each free chunk is on the list of its class, linked both ways, its bytes at
/// its end, and the class bits name the lists that are not empty. Each log block of chunks is a run of chunks
/// from offset 0 to its end: the free ones listed and never two side by side, the others the records the
/// tree's entries name, each of its bytes (32 at least) or less than 32 more, flagged when the chunk before is
/// free. A record longer than a block has blocks of its own. The log holds exactly these blocks and its head.
/// Returns the blocks the records alone would fill.
fn check_log<H: PageHost>(h: &H, ix: &Secondary, what: &str) -> u64 {
    let mut recs = records(h, ix);
    let log = ix.log;
    if log.blocks == 0 {
        assert!(recs.is_empty() && log == KeyLog::default(), "{what}: an empty key log");
        return 0;
    }
    let word = |loc: u64, off: u64| {
        let b = log.radix.get(h, loc >> 12).unwrap();
        let at = ((loc & 0xFFF) + off) as usize;
        u64::from_le_bytes(h.get(native_table::Region::Block(b)).unwrap()[at..at + 8].try_into().unwrap())
    };
    assert_eq!(word(0, 0) as u32, u32::from_le_bytes(*b"NTKL"), "{what}: the head block");
    let (next, bits) = (word(0, 8), word(0, 16));
    let mut free = BTreeMap::new();
    for c in 0..64u64 {
        let (mut loc, mut prev) = (word(0, 24 + 8 * c), 0);
        assert_eq!(bits >> c & 1 == 1, loc != 0, "{what}: the bit of class {c}");
        while loc != 0 {
            let w0 = word(loc, 0);
            let s = w0 & 0x1FFF;
            assert_eq!((w0 >> 63, class(s)), (1, c), "{what}: free chunk {loc:#x}");
            assert_eq!((word(loc, 16), word(loc, s - 8)), (prev, s), "{what}: free chunk {loc:#x} linked back, sized");
            free.insert(loc, s);
            (prev, loc) = (loc, word(loc, 8));
        }
    }
    // The records longer than a block, and their blocks.
    let mut own = BTreeSet::new();
    recs.retain(|&loc, &mut bytes| {
        if bytes <= 4_096 {
            return true;
        }
        assert_eq!((loc & 0xFFF, word(loc, 8) >> 48), (0, 0), "{what}: record {loc:#x} of its own blocks");
        own.extend((loc >> 12)..(loc >> 12) + bytes.div_ceil(4_096));
        false
    });
    let (mut held, mut used) = (1u64, 0u64);
    for k in 1..next {
        if log.radix.get(h, k).unwrap() == 0 {
            continue;
        }
        held += 1;
        if own.contains(&k) {
            continue;
        }
        let (mut at, mut prev_free) = (0u64, false);
        while at < 4_096 {
            let loc = k << 12 | at;
            let s = if word(loc, 0) >> 63 == 1 {
                assert!(!prev_free, "{what}: two free chunks side by side at {loc:#x}");
                prev_free = true;
                free.remove(&loc).unwrap_or_else(|| panic!("{what}: free chunk {loc:#x} on no list"))
            } else {
                let w1 = word(loc, 8);
                let s = w1 >> 48 & 0x1FFF;
                let bytes = recs.remove(&loc).unwrap_or_else(|| panic!("{what}: chunk {loc:#x} no entry names"));
                let least = bytes.max(32);
                assert!(s >= least && s < least + 32, "{what}: record {loc:#x} of {bytes} bytes in {s}");
                assert_eq!(w1 >> 63 == 1, prev_free, "{what}: record {loc:#x}: the chunk before free");
                prev_free = false;
                used += s;
                s
            };
            assert!(s >= 32, "{what}: a chunk of {s} bytes at {loc:#x}");
            at += s;
        }
        assert_eq!(at, 4_096, "{what}: block {k} ends with a chunk");
    }
    assert!(free.is_empty(), "{what}: listed chunks in no block: {free:?}");
    assert!(recs.is_empty(), "{what}: records in no block: {recs:?}");
    assert_eq!(held, log.blocks, "{what}: the key log's blocks");
    used.div_ceil(4_096) + own.len() as u64
}

/// One row's indexed value moved back and forth between two keys longer than W, beside another row: the key
/// log keeps its head and one block, and the host its pages.
#[test]
fn key_log_under_churn_of_one_row() {
    let mut side = Side::new(FILLS[0], 16);
    let (a, b, c) = (text(&[b'a'; 40]), text(&[b'b'; 40]), text(&[b'c'; 40]));
    assert!(side.insert(&a, 1));
    assert!(side.insert(&b, 2));
    let pages = side.h.live_pages();
    for round in 0..=200_000u64 {
        let (from, to) = if round % 2 == 0 { (&b, &c) } else { (&c, &b) };
        assert!(side.delete(from, 2));
        assert!(side.insert(to, 2));
        if round % 50_000 == 0 {
            assert_eq!(side.ix.log.blocks, 2, "after {round} updates: the key log's blocks");
            assert_eq!(side.h.live_pages(), pages, "after {round} updates: the host's pages");
            check_log(&side.h, &side.ix, &format!("after {round} updates"));
        }
    }
    let counts: Vec<i128> = [&a, &b, &c].iter().map(|k| side.ix.count(&side.h, k).unwrap()).collect();
    assert_eq!(counts, [1, 0, 1]);
}

/// Random writes of keys from 1 to 9,000 bytes at W 16, some records longer than a log block: the key log
/// against the tree as it grows and shrinks; cut back to 5 rows, it needs at most a block per record; with
/// every row deleted it is empty. Both fill patterns give the same log.
#[test]
fn key_log_follows_the_rows_held() {
    let run = |fill: u8| {
        let mut r = XorShift(0x106);
        let mut keys = BTreeSet::new();
        while keys.len() < 600 {
            let len = match r.below(20) {
                0 => 4_000 + r.below(5_000),
                1..=5 => 60 + r.below(600),
                _ => r.below(60),
            } as usize;
            let t: Vec<u8> = (0..len).map(|_| [b'a', b'b', 0][r.below(3) as usize]).collect();
            keys.insert(text(&t));
        }
        let keys: Vec<Vec<u8>> = keys.into_iter().collect();
        let mut side = Side::new(fill, 16);
        let mut rows: Vec<(Vec<u8>, u64)> = Vec::new();
        let mut peak = 0u64;
        let mut seen = Vec::new();
        for step in 0..30_000u64 {
            // Growing for the first half, then shrinking.
            let grow = if step < 15_000 { 70 } else { 30 };
            if r.below(100) < grow || rows.is_empty() {
                let k = keys[r.below(keys.len() as u64) as usize].clone();
                assert!(side.insert(&k, step + 1));
                rows.push((k, step + 1));
            } else {
                let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
                assert!(side.delete(&k, rid));
            }
            peak = peak.max(side.ix.log.blocks);
            if step % 500 == 0 {
                let need = check_log(&side.h, &side.ix, &format!("fill {fill:#x}, step {step}"));
                seen.push((side.ix.log.blocks, need));
            }
        }
        while rows.len() > 5 {
            let (k, rid) = rows.swap_remove(r.below(rows.len() as u64) as usize);
            assert!(side.delete(&k, rid));
        }
        check_log(&side.h, &side.ix, &format!("fill {fill:#x}, 5 rows"));
        // Each record pins at most one block, or its own blocks.
        let w = side.ix.shape.w;
        let most: u64 = entries(&side.h, &side.ix)
            .iter()
            .filter(|(k, _, _)| k.len() as u64 > w)
            .map(|(k, _, _)| record_bytes(w, k.len()).div_ceil(4_096))
            .sum();
        let left = side.ix.log.blocks;
        assert!(left <= 1 + most, "fill {fill:#x}: {left} blocks for 5 rows (at most {most} and the head)");
        assert!(peak > 100, "fill {fill:#x}: the log grew to {peak} blocks");
        for (k, rid) in std::mem::take(&mut rows) {
            assert!(side.delete(&k, rid));
        }
        assert_eq!((side.ix.root, side.ix.log), (0, KeyLog::default()), "fill {fill:#x}: emptied");
        (seen, left, peak)
    };
    let a = run(FILLS[0]);
    assert_eq!(a, run(FILLS[1]), "the fill patterns give different key logs");
}
