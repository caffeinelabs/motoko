//! The tree's index form: the entry field, the order of kinds, model tests with
//! (W, V) = (8, 0), (32, 0), (1,016, 0) and (8, 8) and keys longer than W, key-level prefix ranges, reverse
//! scans, and shared-prefix descents giving the same results with the same number of comparisons.

use std::cell::Cell;
use std::cmp::Ordering;
use std::ops::Bound::{self, *};

use native_table::key::enc_bytes;
use native_table::key::probe::{
    cmp_entry, count_field, payload_to_count, row_field, tomb_field, KeyProbe, Kind, ResolveKey,
};
use native_table::tree::{find, Fwd, IndexShape, Probe, Rev};
use native_table::Fault;
use nt_testhost::index::{Ent, ITree, IndexNodes};
use nt_testhost::{VecSink, XorShift, FILLS};

/// succ(p): p with its trailing `FF` bytes dropped and 1 added to the last remaining byte, written into `out`;
/// its length, or `None` when p is empty or all `FF`.
fn succ(p: &[u8], out: &mut [u8]) -> Option<usize> {
    let n = p.iter().rposition(|&b| b != 0xFF)? + 1;
    out[..n].copy_from_slice(&p[..n]);
    out[n - 1] = out[n - 1].wrapping_add(1);
    Some(n)
}

#[test]
fn i62_round_trip_and_kinds() {
    for c in [0i64, 1, -1, 7, (1 << 61) - 1, -(1 << 61), 123_456_789, -987_654_321] {
        assert_eq!(payload_to_count(count_field(c)), c);
    }
    struct NoLog;
    impl ResolveKey for NoLog {
        fn width(&self) -> u64 {
            8
        }
        fn record(&self, _: u64) -> Result<(u64, u64), Fault> {
            Err(Fault::BadRef)
        }
        fn key_piece(&self, _: u64, _: u64, _: &mut [u8]) -> Result<usize, Fault> {
            Err(Fault::BadRef)
        }
        fn rid_base(&self) -> u64 {
            0
        }
    }
    let area = *b"abc\0\0\0\0\0";
    let c = |field: u64, kind: Kind| cmp_entry(&NoLog, 8, &area, field, &KeyProbe { key: b"abc", kind }, 0).unwrap().0;
    // Low < rows by rid < a tombstone at the same rid < the count entry.
    assert_eq!(c(row_field(5), Kind::Low), Ordering::Greater);
    assert_eq!(c(row_field(5), Kind::Row(5)), Ordering::Equal);
    assert_eq!(c(row_field(5), Kind::Row(6)), Ordering::Less);
    assert_eq!(c(tomb_field(5), Kind::Row(5)), Ordering::Greater);
    assert_eq!(c(tomb_field(5), Kind::Row(6)), Ordering::Less);
    assert_eq!(c(row_field(5), Kind::Count), Ordering::Less);
    assert_eq!(c(count_field(9), Kind::Count), Ordering::Equal);
    assert_eq!(c(count_field(9), Kind::Row(u64::MAX >> 3)), Ordering::Greater);
    // Rids near 2^61 compare in 128 bits with a rid base.
    struct Base;
    impl ResolveKey for Base {
        fn width(&self) -> u64 {
            8
        }
        fn record(&self, _: u64) -> Result<(u64, u64), Fault> {
            Err(Fault::BadRef)
        }
        fn key_piece(&self, _: u64, _: u64, _: &mut [u8]) -> Result<usize, Fault> {
            Err(Fault::BadRef)
        }
        fn rid_base(&self) -> u64 {
            u64::MAX
        }
    }
    let o =
        cmp_entry(&Base, 8, &area, row_field((1 << 61) - 1), &KeyProbe { key: b"abc", kind: Kind::Row(u64::MAX) }, 0);
    assert_eq!(o.unwrap().0, Ordering::Greater);
}

/// A prefix-free key: an escaped text over a small alphabet, sometimes with a long shared prefix.
fn gen_key(r: &mut XorShift, w: u64) -> Vec<u8> {
    let mut t: Vec<u8> = Vec::new();
    if r.below(3) == 0 {
        t.extend(std::iter::repeat(b'p').take((w as usize) + r.below(200) as usize));
    }
    for _ in 0..r.below(2 * w + 4) {
        t.push([0u8, b'a', b'b', 0xFF][r.below(4) as usize]);
    }
    let mut s = VecSink::default();
    enc_bytes(&mut s, &t).unwrap();
    s.0
}

fn model_shape(w: u64, v: u64, seed: u64) {
    let shape = IndexShape::new(w, v).unwrap();
    for fill in FILLS {
        let mut t = ITree::new(shape, fill);
        let mut model: Vec<Ent> = Vec::new();
        let mut r = XorShift(seed);
        let mut keys: Vec<Vec<u8>> = (0..300).map(|_| gen_key(&mut r, w)).collect();
        keys.sort();
        keys.dedup();
        for step in 0..12_000 {
            let key = keys[r.below(keys.len() as u64) as usize].clone();
            let rank = if r.below(8) == 0 { 2 } else { 1 };
            let e = Ent {
                key,
                rank,
                rid: if rank == 2 { r.below(1_000) } else { r.below(50) * 4_096 + r.below(4_096) },
                tomb: rank == 1 && r.below(4) == 0,
                value: (0..v).map(|_| r.next_u64() as u8).collect(),
            };
            let pos = model.binary_search_by(|m| m.order(&e));
            if r.below(3) < 2 {
                // A count entry or row is unique per (key, rank, rid): skip if one with that position exists.
                let exists =
                    model.iter().any(|m| m.key == e.key && m.rank == e.rank && (e.rank == 2 || m.rid == e.rid));
                if !exists {
                    assert!(t.insert(&e), "insert at {step}");
                    let i = pos.unwrap_err();
                    model.insert(i, e);
                }
            } else if let Some(m) = (!model.is_empty()).then(|| model[r.below(model.len() as u64) as usize].clone()) {
                if !m.tomb {
                    assert!(t.remove(&m), "remove at {step}");
                    let i = model.binary_search_by(|x| x.order(&m)).unwrap();
                    model.remove(i);
                }
            }
            if step % 1_000 == 999 {
                let stored: Vec<Ent> = model.iter().map(|e| e.stored(w)).collect();
                assert_eq!(t.scan::<Fwd>(Unbounded, Unbounded), stored, "forward scan, W {w} V {v}");
                let mut rev = stored.clone();
                rev.reverse();
                assert_eq!(t.scan::<Rev>(Unbounded, Unbounded), rev, "reverse scan");
                // Prefix ranges through normalized key-level bounds: [(p, Low), (succ(p), Low)).
                for _ in 0..20 {
                    let k = &keys[r.below(keys.len() as u64) as usize];
                    let p = &k[..r.below(k.len() as u64 + 1) as usize];
                    let mut buf = vec![0u8; p.len()];
                    let lo = KeyProbe { key: p, kind: Kind::Low };
                    let hi_len = succ(p, &mut buf);
                    let hi = hi_len.map(|n| KeyProbe { key: &buf[..n], kind: Kind::Low });
                    let hb: Bound<&KeyProbe<'_>> = match &hi {
                        Some(h) => Excluded(h),
                        None => Unbounded,
                    };
                    let want: Vec<Ent> = model.iter().filter(|e| e.key.starts_with(p)).map(|e| e.stored(w)).collect();
                    assert_eq!(t.scan::<Fwd>(Included(&lo), hb), want, "prefix {:x?}", p);
                    let mut rw = want.clone();
                    rw.reverse();
                    assert_eq!(t.scan::<Rev>(hb, Included(&lo)), rw, "reverse prefix");
                }
                // Every entry is found by its own probe.
                for e in model.iter().filter(|e| !e.tomb).take(50) {
                    assert!(find(shape, &t.nodes, t.root, &e.probe()).unwrap().is_some());
                }
            }
        }
        // The run must have built a real tree: hundreds of entries over several levels.
        assert!(model.len() > 300, "model has {} entries", model.len());
        assert!(t.nodes.live.len() > 2, "{} nodes", t.nodes.live.len());
        eprintln!("W {w} V {v}: {} entries in {} nodes", model.len(), t.nodes.live.len());
        // Empty the tree.
        for e in model.clone() {
            if !e.tomb {
                assert!(t.remove(&e));
            }
        }
        // Only the tombstones, which this front end never removes, are left.
        assert!(t.scan::<Fwd>(Unbounded, Unbounded).iter().all(|e| e.tomb));
    }
}

#[test]
fn model_w8_v0() {
    model_shape(8, 0, 11);
}

#[test]
fn model_w32_v0() {
    model_shape(32, 0, 12);
}

#[test]
fn model_w1016_v0() {
    model_shape(1_016, 0, 13);
}

#[test]
fn model_w8_v8() {
    model_shape(8, 8, 14);
}

/// Counts comparisons; with `P` false it hides the shared-prefix path.
struct Counting<'a, const P: bool> {
    inner: KeyProbe<'a>,
    calls: Cell<u64>,
}

impl<const P: bool> Probe<IndexNodes> for Counting<'_, P> {
    const PREFIX: bool = P;
    fn cmp_at(&self, m: &IndexNodes, n: u64, koff: u64) -> Result<Ordering, Fault> {
        self.calls.set(self.calls.get() + 1);
        self.inner.cmp_at(m, n, koff)
    }
    fn cmp_from(&self, m: &IndexNodes, n: u64, koff: u64, skip: u64) -> Result<(Ordering, u64), Fault> {
        self.calls.set(self.calls.get() + 1);
        self.inner.cmp_from(m, n, koff, skip)
    }
}

#[test]
fn shared_prefix_descents_equal_plain_ones() {
    let shape = IndexShape::new(8, 0).unwrap();
    let mut t = ITree::new(shape, FILLS[0]);
    let mut r = XorShift(77);
    // Keys sharing a 500-byte prefix: every comparison past W resolves overflow pieces.
    let prefix: Vec<u8> = (0..500).map(|i| b'a' + (i % 7) as u8).collect();
    let mut keys = Vec::new();
    for i in 0..2_000u64 {
        let mut k = prefix.clone();
        k.extend(i.to_be_bytes());
        k.extend((0..r.below(40)).map(|_| r.next_u64() as u8));
        let mut s = VecSink::default();
        enc_bytes(&mut s, &k).unwrap();
        keys.push(s.0);
        let e = Ent { key: keys.last().unwrap().clone(), rank: 1, rid: i, tomb: false, value: vec![] };
        assert!(t.insert(&e));
    }
    for (i, k) in keys.iter().enumerate().step_by(7) {
        for kind in [Kind::Low, Kind::Row(i as u64), Kind::Count] {
            let a = Counting::<true> { inner: KeyProbe { key: k, kind }, calls: Cell::new(0) };
            let b = Counting::<false> { inner: KeyProbe { key: k, kind }, calls: Cell::new(0) };
            assert_eq!(find(shape, &t.nodes, t.root, &a).unwrap(), find(shape, &t.nodes, t.root, &b).unwrap());
            assert_eq!(a.calls.get(), b.calls.get(), "the same number of comparisons");
        }
    }
}
