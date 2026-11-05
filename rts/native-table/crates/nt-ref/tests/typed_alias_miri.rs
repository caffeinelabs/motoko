//! Values handed out as `&mut V` by an `iter_mut` loop and a `range_mut` loop over the crate's cursor on
//! raw typed nodes stay valid across later cursor calls, allocations and hand-backs, and are written both
//! before and after those calls. Run under Miri with strict provenance, once under stacked borrows and once
//! under tree borrows (`scripts/miri.sh`): a cursor step that formed a reference wider than the field it
//! reads (a whole node, say) would invalidate the values held before it.
//!
//! The same script runs over the transcription of today's `PBTreeMap`, which holds values the same way.

use nt_ref::mem::{select, Region};
use nt_ref::walk::check;
use nt_ref::{CrateMap, MapShapeChecked, RefMap};

fn sizes() -> (u64, u64) {
    // Entries of the small map (three levels need more than 254 · 85 entries; two levels and many leaves
    // suffice to cross leaves and internal entries) and of the boxed map.
    if cfg!(miri) {
        (700, 24)
    } else {
        (20_000, 400)
    }
}

fn big(k: u64) -> [u8; 1024] {
    let mut v = [0u8; 1024];
    v[..8].copy_from_slice(&k.to_be_bytes());
    v
}

/// One map's script. `$m` is a map of `<u64, u64>`, `$b` of `<u64, [u8; 1024]>`, `$x` a scratch map whose
/// inserts allocate nodes while values are held.
macro_rules! script {
    ($m:expr, $b:expr, $x:expr) => {{
        let (n, nb) = sizes();
        let (m, b, x) = (&mut $m, &mut $b, &mut $x);
        for i in 0..n {
            m.insert(i, i);
        }
        for i in 0..nb {
            b.insert(i, big(i));
        }

        // Every value held from iter_mut: each written as it is handed out, before the next cursor call,
        // then all written again after the loop, last first.
        let mut all: Vec<(u64, &mut u64)> = Vec::new();
        for (k, v) in m.iter_mut() {
            *v = 3 * k;
            all.push((k, v));
        }
        assert_eq!(all.len() as u64, n);
        for (k, v) in all.iter_mut().rev() {
            assert_eq!(**v, 3 * *k);
            **v += 1;
        }
        drop(all);

        // Boxed values held through their boxes.
        let mut held: Vec<(u64, &mut [u8; 1024])> = Vec::new();
        for (k, v) in b.iter_mut() {
            v[1023] = k as u8;
            held.push((k, v));
        }
        for (k, v) in held.into_iter().rev() {
            assert_eq!(v[1023], k as u8);
            v[1022] = (k as u8).wrapping_add(1);
        }

        // A range's values held while another map allocates nodes, then hands every node back.
        let mut held: Vec<(u64, &mut u64)> = Vec::new();
        let mut scratch = Vec::new();
        for (k, v) in m.range_mut(5..n - 5) {
            *v += k;
            held.push((k, v));
            if k % 16 == 0 {
                for j in 0..64 {
                    x.insert(k * 64 + j, j);
                    scratch.push(k * 64 + j);
                }
            }
        }
        for j in scratch {
            x.remove(j);
        }
        assert_eq!(x.root(), 0);
        for (k, v) in held.iter_mut().rev() {
            assert_eq!(**v, 4 * *k + 1);
            **v += *k;
        }
        drop(held);

        // A bounded range with an excluded end: values held across the end-bound comparisons.
        let mut held: Vec<&mut u64> =
            m.range_mut((std::ops::Bound::Excluded(1), std::ops::Bound::Excluded(n / 2))).map(|(_, v)| v).collect();
        for v in held.iter_mut() {
            **v += 7;
        }
        for v in held.into_iter().rev() {
            *v -= 7;
        }

        for (k, v) in m.iter() {
            let want = if (5..n - 5).contains(&k) { 5 * k + 1 } else { 3 * k + 1 };
            assert_eq!(*v, want, "value of {k}");
        }
        for (k, v) in b.iter() {
            assert_eq!(v[..8], k.to_be_bytes());
            assert_eq!(v[1023], k as u8);
            assert_eq!(v[1022], (k as u8).wrapping_add(1));
        }
        assert_eq!(check::<u64, u64>(m.root()), n);
        assert_eq!(check::<u64, [u8; 1024]>(b.root()), nb);
    }};
}

#[test]
fn values_held_from_iter_mut_and_range_mut_are_written_apart() {
    let _region = Region::install(if cfg!(miri) { 1 << 20 } else { 8 << 20 });
    select(1);
    let (mut m, mut b, mut x) =
        (CrateMap::<u64, u64>::new(), CrateMap::<u64, [u8; 1024]>::new(), CrateMap::<u64, u64>::new());
    script!(m, b, x);
}

#[test]
fn checked_values_held_from_iter_mut_and_range_mut_are_written_apart() {
    let _region = Region::install(if cfg!(miri) { 1 << 20 } else { 8 << 20 });
    select(2);
    type C<V> = CrateMap<u64, V, MapShapeChecked<u64, V>>;
    let (mut m, mut b, mut x) = (C::<u64>::new(), C::<[u8; 1024]>::new(), C::<u64>::new());
    script!(m, b, x);
}

#[test]
fn reference_values_held_from_iter_mut_and_range_mut_are_written_apart() {
    let _region = Region::install(if cfg!(miri) { 1 << 20 } else { 8 << 20 });
    select(0);
    let (mut m, mut b, mut x) =
        (RefMap::<u64, u64>::new(), RefMap::<u64, [u8; 1024]>::new(), RefMap::<u64, u64>::new());
    script!(m, b, x);
}
