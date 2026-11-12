//! The tree's index form: the entry field and the order of kinds, and a model test with (W, V) = (1,016, 0),
//! whose few entries a node make the tree deep, with keys longer than W, key-level prefix ranges and reverse
//! scans.

use std::cmp::Ordering;
use std::ops::Bound::{self, *};

use native_table::key::enc_bytes;
use native_table::key::probe::{cmp_entry, count_field, payload_to_count, row_field, KeyProbe, Kind, ResolveKey};
use native_table::tree::{find, Fwd, IndexShape, Rev};
use native_table::Fault;
use nt_testhost::index::{Ent, ITree};
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
        fn key_bytes(&self, _: u64, _: u64, _: u64) -> Result<&[u8], Fault> {
            Err(Fault::BadRef)
        }
    }
    let area = *b"abc\0\0\0\0\0";
    let c = |field: u64, kind: Kind| cmp_entry(&NoLog, 8, &area, field, &KeyProbe { key: b"abc", kind }, 0).unwrap().0;
    // Low < rows by rid < the count entry.
    assert_eq!(c(row_field(5), Kind::Low), Ordering::Greater);
    assert_eq!(c(row_field(5), Kind::Row(5)), Ordering::Equal);
    assert_eq!(c(row_field(5), Kind::Row(6)), Ordering::Less);
    assert_eq!(c(row_field(5), Kind::Count), Ordering::Less);
    assert_eq!(c(count_field(9), Kind::Count), Ordering::Equal);
    assert_eq!(c(count_field(9), Kind::Row(u64::MAX >> 3)), Ordering::Greater);
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
                assert!(t.remove(&m), "remove at {step}");
                let i = model.binary_search_by(|x| x.order(&m)).unwrap();
                model.remove(i);
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
                for e in model.iter().take(50) {
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
            assert!(t.remove(&e));
        }
        assert!(t.scan::<Fwd>(Unbounded, Unbounded).is_empty());
    }
}

#[test]
fn model_w1016_v0() {
    model_shape(1_016, 0, 13);
}
