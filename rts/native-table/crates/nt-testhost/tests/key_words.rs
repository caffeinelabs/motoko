//! The comparator of key bytes eight at a time (`key::words`): long byte strings in memcmp order, and node
//! searches by words giving what comparisons entry by entry give, for every kind of probe, equal key areas,
//! tombstones, count entries, overflowed keys and rid bases.

use std::cmp::Ordering;
use std::ops::Bound;

use native_table::key::probe::{KeyProbe, Kind};
use native_table::key::words::{bytes_cmp_any, bytes_cmp_long};
use native_table::tree::{descend, find, Cursor, Fwd, IndexShape, Path, Probe};
use native_table::Fault;
use nt_testhost::index::{Ent, ITree, IndexNodes};
use nt_testhost::{XorShift, FILLS};

/// Long byte strings compare as memcmp over their common length, with the bytes they share: equal for
/// thousands of bytes then differing at every position of a 32-byte stretch and of a word, by one bit, with
/// lengths that end inside a stretch and a word.
#[test]
fn long_byte_strings_compare_as_memcmp() {
    let mut r = XorShift(0x32B);
    for len in [65usize, 71, 96, 127, 128, 129, 1_000, 4_096, 4_103] {
        let base: Vec<u8> = (0..len).map(|_| r.below(256) as u8).collect();
        let mut cases = vec![base.clone()];
        for at in (0..len).step_by(1 + len / 97).chain([len - 1, len / 2]) {
            for bit in [1u8, 0x80] {
                let mut v = base.clone();
                v[at] ^= bit;
                cases.push(v);
            }
        }
        cases.push(base[..len - 3].to_vec());
        for a in &cases {
            for b in [&base, &cases[cases.len() / 2], &cases[1]] {
                let n = a.len().min(b.len());
                let shared = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count();
                let want = (a[..n].cmp(&b[..n]), shared);
                assert_eq!(bytes_cmp_long(a, b), want, "{len}");
                assert_eq!(bytes_cmp_any(a, b), want, "{len}");
            }
        }
    }
}

/// A probe that hides the search by words: every comparison entry by entry, as before it existed.
struct Plain<'a> {
    inner: KeyProbe<'a>,
}

impl Probe<IndexNodes> for Plain<'_> {
    const PREFIX: bool = true;
    fn cmp_at(&self, m: &IndexNodes, n: u64, koff: u64) -> Result<Ordering, Fault> {
        self.inner.cmp_at(m, n, koff)
    }
    fn cmp_from(&self, m: &IndexNodes, n: u64, koff: u64, skip: u64) -> Result<(Ordering, u64), Fault> {
        self.inner.cmp_from(m, n, koff, skip)
    }
}

fn first_entries(t: &ITree, c: &mut Cursor<Fwd>) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    while let Some((n, i)) = c.peek(t.nodes.shape, &t.nodes).unwrap() {
        out.push((n, i));
        if out.len() == 4 {
            break;
        }
        c.advance(t.nodes.shape, &t.nodes, n, i).unwrap();
    }
    out
}

/// A tree of (W, V) holding keys with many equal key areas (a small alphabet, trailing zeros, keys longer
/// than W and than 64 bytes), rows and count entries, searched with every kind of probe through both ways.
fn searches_agree(w: u64, v: u64, seed: u64) {
    let shape = IndexShape::new(w, v).unwrap();
    let mut t = ITree::new(shape, FILLS[(seed % FILLS.len() as u64) as usize]);
    let mut r = XorShift(seed);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for _ in 0..150 {
        let n = match r.below(4) {
            0 => r.below(9),
            1 => w.saturating_sub(r.below(9)),
            2 => w + 1 + r.below(70),
            _ => r.below(w + 1),
        } as usize;
        // Often a longer or shorter form of a key already made: equal key areas, keys over W sharing them.
        let mut k: Vec<u8> = match keys.len() {
            0 => Vec::new(),
            l if r.below(2) == 0 => keys[r.below(l as u64) as usize].clone(),
            _ => Vec::new(),
        };
        k.truncate(n);
        while k.len() < n {
            k.push([0u8, 1, b'a', 0xFF][r.below(4) as usize]);
        }
        keys.push(k);
    }
    keys.sort();
    keys.dedup();
    let value = vec![0u8; v as usize];
    for _ in 0..3_000 {
        let key = keys[r.below(keys.len() as u64) as usize].clone();
        let rank = if r.below(8) == 0 { 2 } else { 1 };
        let rid = if rank == 2 { r.below(1_000) } else { r.below(400) };
        let e = Ent { key, rank, rid, value: value.clone() };
        t.insert(&e);
    }
    let mut probes = keys.clone();
    // Each key's first W bytes, without and with trailing zeros: the key area of a longer key exactly.
    for k in &keys {
        let mut p = k[..k.len().min(w as usize)].to_vec();
        probes.push(p.clone());
        while p.last() == Some(&0) {
            p.pop();
        }
        probes.push(p.clone());
        p.push(0);
        probes.push(p);
    }
    probes.extend(
        (0..40).map(|_| (0..r.below(w + 2)).map(|_| [0u8, 1, b'a', b'b', 0xFF][r.below(5) as usize]).collect()),
    );
    let mut by_words = 0u64;
    for k in &probes {
        // A rid the tree may hold.
        let rid = r.below(400);
        for kind in [Kind::Low, Kind::Row(rid), Kind::Row(u64::MAX), Kind::Count] {
            let p = KeyProbe { key: k, kind };
            let q = Plain { inner: p };
            assert_eq!(find(shape, &t.nodes, t.root, &p).unwrap(), find(shape, &t.nodes, t.root, &q).unwrap());
            let (mut a, mut b) = (Path::new(), Path::new());
            let sa = descend(shape, &t.nodes, t.root, &p, &mut a).unwrap();
            let sb = descend(shape, &t.nodes, t.root, &q, &mut b).unwrap();
            assert_eq!((sa, a.frames()), (sb, b.frames()), "W {w} key {k:?} {kind:?}");
            let mut ca: Cursor<Fwd> = Cursor::seek(shape, &t.nodes, t.root, Bound::Included(&p)).unwrap();
            let mut cb: Cursor<Fwd> = Cursor::seek(shape, &t.nodes, t.root, Bound::Excluded(&p)).unwrap();
            let (xa, xb) = (first_entries(&t, &mut ca), first_entries(&t, &mut cb));
            let mut ca2: Cursor<Fwd> = Cursor::seek(shape, &t.nodes, t.root, Bound::Included(&q)).unwrap();
            let mut cb2: Cursor<Fwd> = Cursor::seek(shape, &t.nodes, t.root, Bound::Excluded(&q)).unwrap();
            assert_eq!((xa, xb), (first_entries(&t, &mut ca2), first_entries(&t, &mut cb2)));
            if k.len() as u64 <= w && w <= 32 {
                by_words += 1;
            }
        }
    }
    // Key areas of up to four words are searched by words; wider ones (and longer keys) entry by entry.
    assert!(w > 32 || by_words > 100, "W {w}: {by_words} probes searched by words");
}

#[test]
fn node_searches_by_words_equal_comparisons_entry_by_entry() {
    for (w, v) in [(8, 0), (16, 0), (24, 0), (32, 0), (8, 8), (16, 8), (40, 0), (64, 0), (200, 0)] {
        for seed in [1, 2, 3] {
            searches_agree(w, v, seed + w);
        }
    }
}
