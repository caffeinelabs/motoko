//! The one B-tree in its typed form: node shapes, the model test against `BTreeMap` for
//! three shapes (capacities 254/169, 169/127 and 3/3), split geometry, allocation and hand-back order, the
//! stale bytes a removal leaves, cursors in both directions with every bound, and identical results and host-call
//! logs under both fill patterns.

use std::collections::BTreeMap;
use std::ops::Bound::{self, *};

use native_table::consts::NODE_SIZE;
use native_table::tree::shape::{node_shape, IndexShape, NodeShape};
use native_table::tree::{Fwd, Rev, Shape, TypedShape};
use nt_testhost::typed::{NodeCall, Part, Shp, TMap};
use nt_testhost::{XorShift, FILLS};

#[test]
fn node_shape_vectors() {
    let ns = |ks, va, vs, internal| node_shape(ks, va, vs, internal);
    assert_eq!(ns(8, 8, 8, false), NodeShape { cap: 254, voff: 2_040, coff: 4_072 });
    assert_eq!(ns(8, 8, 8, true), NodeShape { cap: 169, voff: 1_360, coff: 2_712 });
    assert_eq!(ns(16, 8, 8, false).cap, 169);
    assert_eq!(ns(16, 8, 8, true).cap, 127);
    assert_eq!(ns(1_012, 8, 8, false).cap, 3);
    assert_eq!(ns(1_012, 8, 8, true).cap, 3);
    assert_eq!(ns(1_012, 4, 1_012, true).cap, 2);
    // A boxed key of 1,013 bytes is an 8-byte part.
    assert_eq!(ns(8, 8, 8, false).cap, 254);
    assert_eq!(<Shp<u64, u64> as TypedShape>::LEAF.cap, 254);
    assert_eq!(<Shp<u64, u64> as TypedShape>::INTERNAL.cap, 169);
    // Index shapes: nL = ⌊4,072 / S⌋ and nI = ⌊4,064 / (S + 8)⌋ for V = 0, S = W + 8.
    for w in (8..=1_016).step_by(8) {
        let sh = IndexShape::new(w, 0).unwrap();
        assert_eq!(sh.leaf.cap, 4_072 / (w + 8), "W {w}");
        assert_eq!(sh.internal.cap, 4_064 / (w + 16), "W {w}");
        assert!(sh.internal.cap >= 3);
    }
    let table = [
        (8, [254, 169, 127, 84, 50, 28, 14], [169, 127, 101, 72, 46, 26, 14]),
        (16, [169, 127, 101, 72, 46, 26, 14], [127, 101, 84, 63, 42, 25, 14]),
        (32, [101, 84, 72, 56, 39, 24, 13], [84, 72, 63, 50, 36, 23, 13]),
    ];
    for (w, leaf, internal) in table {
        for (k, v) in [0u64, 8, 16, 32, 64, 128, 256].into_iter().enumerate() {
            let sh = IndexShape::new(w, v).unwrap();
            assert_eq!((sh.leaf.cap, sh.internal.cap), (leaf[k], internal[k]), "W {w} V {v}");
        }
    }
    assert_eq!(IndexShape::new(40, 0).map(|s| (s.leaf.cap, s.internal.cap)), Some((84, 72)));
    assert_eq!(IndexShape::new(1_016, 0).map(|s| (s.leaf.cap, s.internal.cap)), Some((3, 3)));
    assert!(IndexShape::new(1_016, 8).is_none());
    assert!(IndexShape::new(12, 0).is_none());
    assert!(IndexShape::new(8, 24).is_none());
}

fn model_run<K: Part, V: Part>(
    fill: u8,
    seed: u64,
    ops: usize,
    key: impl Fn(u64) -> K,
    val: impl Fn(u64) -> V,
    range: u64,
) -> (Vec<NodeCall>, Vec<(K, V)>) {
    let mut t: TMap<K, V> = TMap::new(fill);
    let mut model: BTreeMap<K, V> = BTreeMap::new();
    let mut r = XorShift(seed);
    for step in 0..ops {
        let x = r.below(range);
        let k = key(x);
        match r.below(10) {
            0..=5 => {
                let v = val(r.next_u64());
                assert_eq!(t.insert(k, v), model.insert(k, v), "insert at {step}");
            }
            6..=8 => assert_eq!(t.remove(k), model.remove(&k), "remove at {step}"),
            _ => assert_eq!(t.get(k), model.get(&k).copied(), "get at {step}"),
        }
        if step % 97 == 0 {
            t.check();
            assert_eq!(t.len, model.len() as u64);
        }
        if step % 211 == 0 {
            let a = key(r.below(range));
            let b = key(r.below(range));
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let bounds: [(Bound<K>, Bound<K>); 5] = [
                (Unbounded, Unbounded),
                (Included(lo), Included(hi)),
                (Excluded(lo), Excluded(hi)),
                (Included(lo), Unbounded),
                (Unbounded, Excluded(hi)),
            ];
            for (l, h) in bounds {
                if lo == hi && matches!((l, h), (Excluded(_), Excluded(_))) {
                    // std's BTreeMap refuses this range; the tree yields nothing (checked below).
                    continue;
                }
                let want: Vec<(K, V)> = model.range((l, h)).map(|(k, v)| (*k, *v)).collect();
                assert_eq!(t.range::<Fwd>(l, h), want, "forward range");
                let mut rev = want.clone();
                rev.reverse();
                assert_eq!(t.range::<Rev>(l, h), rev, "reverse range");
            }
            // A start above the end, or Excluded(x)..Excluded(x), yields nothing.
            assert!(t.range::<Fwd>(Included(hi), Excluded(lo)).is_empty() || lo == hi);
            assert!(t.range::<Fwd>(Excluded(lo), Excluded(lo)).is_empty());
        }
    }
    t.check();
    let all = t.range::<Fwd>(Unbounded, Unbounded);
    let want: Vec<(K, V)> = model.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(all, want);
    // Remove everything: every node goes back and the root is 0.
    for (k, _) in want {
        assert!(t.remove(k).is_some());
    }
    assert_eq!((t.root, t.len), (0, 0));
    assert!(t.nodes.live.is_empty(), "nodes left: {}", t.nodes.live.len());
    (std::mem::take(&mut t.nodes.log), all)
}

fn k16(x: u64) -> [u8; 16] {
    let mut b = [0x11u8; 16];
    b[..8].copy_from_slice(&x.to_be_bytes());
    b
}

fn k1012(x: u64) -> [u8; 1012] {
    let mut b = [0x22u8; 1012];
    b[..8].copy_from_slice(&x.to_be_bytes());
    b
}

#[test]
fn model_u64_u64() {
    let a = model_run::<u64, u64>(FILLS[0], 1, 60_000, |x| x, |v| v, 20_000);
    let b = model_run::<u64, u64>(FILLS[1], 1, 60_000, |x| x, |v| v, 20_000);
    assert_eq!(a, b, "the two fill patterns must give identical results and host-call logs");
}

#[test]
fn model_16_byte_keys() {
    let a = model_run::<[u8; 16], u64>(FILLS[0], 2, 30_000, k16, |v| v, 8_000);
    let b = model_run::<[u8; 16], u64>(FILLS[1], 2, 30_000, k16, |v| v, 8_000);
    assert_eq!(a, b);
}

#[test]
fn model_capacity_three() {
    // Capacity 3/3: deep trees, every split, merge and rotation path, many times.
    let a = model_run::<[u8; 1012], u64>(FILLS[0], 3, 20_000, k1012, |v| v, 600);
    let b = model_run::<[u8; 1012], u64>(FILLS[1], 3, 20_000, k1012, |v| v, 600);
    assert_eq!(a, b);
}

#[test]
fn split_geometry_and_allocation_order() {
    // <u64, u64>: a leaf split at c = 254 gives 127 / up / 127.
    let mut t: TMap<u64, u64> = TMap::new(FILLS[0]);
    for k in 0..255u64 {
        t.insert(k, k);
    }
    let s = TMap::<u64, u64>::S;
    let root = t.root;
    assert!(!native_table::tree::node::is_leaf(&t.nodes, root).unwrap());
    let l = native_table::tree::node::child(s, &t.nodes, root, 0).unwrap();
    let r = native_table::tree::node::child(s, &t.nodes, root, 1).unwrap();
    assert_eq!(native_table::tree::node::count(s, &t.nodes, l).unwrap(), 127);
    assert_eq!(native_table::tree::node::count(s, &t.nodes, r).unwrap(), 127);
    assert_eq!(native_table::tree::node::count(s, &t.nodes, root).unwrap(), 1);
    // The right node is allocated before the new root.
    let allocs: Vec<(u64, bool)> = t
        .nodes
        .log
        .iter()
        .filter_map(|c| if let NodeCall::Alloc { node, leaf } = c { Some((*node, *leaf)) } else { None })
        .collect();
    assert_eq!(allocs.len(), 3);
    assert_eq!(allocs[1], (r, true));
    assert_eq!(allocs[2], (root, false));

    // Capacity 3: a split gives 2 / up / 1; a root split allocates right nodes bottom-up, then the root.
    let mut t: TMap<[u8; 1012], u64> = TMap::new(FILLS[1]);
    let s3 = TMap::<[u8; 1012], u64>::S;
    for k in 0..4u64 {
        t.insert(k1012(k), k);
    }
    let l = native_table::tree::node::child(s3, &t.nodes, t.root, 0).unwrap();
    let r = native_table::tree::node::child(s3, &t.nodes, t.root, 1).unwrap();
    assert_eq!(native_table::tree::node::count(s3, &t.nodes, l).unwrap(), 2);
    assert_eq!(native_table::tree::node::count(s3, &t.nodes, r).unwrap(), 1);
    let before = t.nodes.log.len();
    // Fill until a two-level split happens in one insert.
    let mut k = 4u64;
    loop {
        let height = t.check();
        t.insert(k1012(k), k);
        k += 1;
        if t.check() > height {
            break;
        }
    }
    let tail: Vec<NodeCall> = t.nodes.log[before..].to_vec();
    let last_allocs: Vec<bool> = tail
        .iter()
        .rev()
        .take_while(|c| matches!(c, NodeCall::Alloc { .. }))
        .filter_map(|c| if let NodeCall::Alloc { leaf, .. } = c { Some(*leaf) } else { None })
        .collect();
    // Reversed: the root (internal) last, after a right internal node and a right leaf.
    assert_eq!(last_allocs.first(), Some(&false));
    assert_eq!(last_allocs.last(), Some(&true));
}

#[test]
fn removal_hands_back_bottom_up_then_the_root() {
    let mut t: TMap<[u8; 1012], u64> = TMap::new(FILLS[0]);
    for k in 0..40u64 {
        t.insert(k1012(k), k);
    }
    t.nodes.log.clear();
    for k in 0..40u64 {
        let before = t.nodes.log.len();
        t.remove(k1012(k));
        // No removal allocates.
        assert!(t.nodes.log[before..].iter().all(|c| matches!(c, NodeCall::HandBack(_))));
        t.check();
    }
    assert_eq!(t.root, 0);
    assert!(matches!(t.nodes.log.last(), Some(NodeCall::HandBack(_))));
}

#[test]
fn stale_bytes_of_a_leaf_removal() {
    // After a leaf removal, slot count − 1 keeps a copy of the old last entry.
    let mut t: TMap<u64, u64> = TMap::new(FILLS[0]);
    for k in 1..=5u64 {
        t.insert(k * 10, k * 100);
    }
    let n = t.root;
    t.remove(20);
    let node = t.nodes.node(n).to_vec();
    let sh = <Shp<u64, u64> as TypedShape>::LEAF;
    // Live: 10, 30, 40, 50 in slots 0..4; slot 4 still holds 50 / 500.
    let key = |i: usize| u64::from_le_bytes(node[8 + 8 * i..16 + 8 * i].try_into().unwrap());
    let val =
        |i: usize| u64::from_le_bytes(node[sh.voff as usize + 8 * i..sh.voff as usize + 8 + 8 * i].try_into().unwrap());
    assert_eq!((0..4).map(key).collect::<Vec<_>>(), vec![10, 30, 40, 50]);
    assert_eq!((key(4), val(4)), (50, 500));
    // Slot 5 was never written: it still carries the fill pattern.
    assert!(node[8 + 8 * 5..16 + 8 * 5].iter().all(|&b| b == FILLS[0]));
    assert_eq!(node.len() as u64, NODE_SIZE);
}

#[test]
fn checked_typed_instance_agrees() {
    // The CHECKED = true instance computes the same tree on valid data.
    use native_table::tree::{descend, insert_at, insert_first, Path, Search, Typed};
    use nt_testhost::typed::{ShpChecked, TPending, TProbe};
    let s: Typed<ShpChecked<u64, u64>> = Typed::NEW;
    assert!(<Typed<ShpChecked<u64, u64>> as Shape>::CHECKED);
    let mut m: TMap<u64, u64> = TMap::new(FILLS[0]);
    let mut nodes = nt_testhost::typed::TypedNodes::<u64, u64>::new(FILLS[0]);
    let mut root = 0u64;
    let mut r = XorShift(9);
    for _ in 0..5_000 {
        let k = r.below(3_000);
        m.insert(k, k);
        let mut p = TPending { key: [0; 1024], val: [0; 1024], child: 0 };
        p.key[..8].copy_from_slice(&k.to_le_bytes());
        p.val[..8].copy_from_slice(&k.to_le_bytes());
        if root == 0 {
            root = insert_first(s, &mut nodes, &p).unwrap();
            continue;
        }
        let mut path = Path::new();
        if let Search::Vacant(n, j) = descend(s, &nodes, root, &TProbe(k), &mut path).unwrap() {
            insert_at(s, &mut nodes, &mut root, n, j, &mut p, &mut path).unwrap();
        }
    }
    assert_eq!(nodes.mem, m.nodes.mem, "checked and unchecked instances write identical bytes");
}
