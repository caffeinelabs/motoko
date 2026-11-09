//! The comparator of key bytes eight at a time (`key::words`): byte strings in memcmp order, key areas
//! against zero-padded keys, and node searches by words giving what comparisons entry by entry give, for
//! every kind of probe, equal key areas, tombstones, count entries, overflowed keys and rid bases.

use std::cmp::Ordering;
use std::ops::Bound;

use native_table::key::probe::{KeyProbe, Kind};
use native_table::key::words::{
    area_cmp, area_key_cmp, bytes_cmp, bytes_cmp_any, bytes_cmp_long, search_fits, KeyWords,
};
use native_table::tree::{descend, find, Cursor, Fwd, IndexShape, Path, Probe};
use native_table::Fault;
use nt_testhost::index::{Ent, ITree, IndexNodes};
use nt_testhost::{XorShift, FILLS};

/// Byte strings that differ at every position of a word, at word boundaries, by length alone, and by
/// trailing zeros.
fn strings(r: &mut XorShift) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        b"a".to_vec(),
        b"a\0".to_vec(),
        b"ab".to_vec(),
        b"abcdefg".to_vec(),
        b"abcdefgh".to_vec(),
        b"abcdefgi".to_vec(),
        b"abcdefgh\0".to_vec(),
        b"abcdefgh\x01".to_vec(),
        vec![0xFF; 8],
        vec![0xFF; 9],
        (1..=17).collect(),
        (1..=16).collect(),
        (1..=16).chain([0x12]).collect(),
    ];
    for _ in 0..60 {
        let n = r.below(40) as usize;
        v.push((0..n).map(|_| [0u8, 1, 0x7F, 0x80, 0xFF][r.below(5) as usize]).collect());
    }
    v
}

fn padded(a: &[u8], b: &[u8]) -> Ordering {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0).cmp(&b.get(i).copied().unwrap_or(0)))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

#[test]
fn byte_strings_compare_as_memcmp() {
    let s = strings(&mut XorShift(3));
    for a in &s {
        for b in &s {
            let n = a.len().min(b.len());
            let shared = a.iter().zip(b).take_while(|(x, y)| x == y).count();
            assert_eq!(bytes_cmp(a, b), (a[..n].cmp(&b[..n]), shared), "{a:?} {b:?}");
        }
    }
}

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

#[test]
fn key_areas_compare_as_the_zero_padded_key() {
    let s = strings(&mut XorShift(5));
    for w in [8usize, 16, 24, 40, 64] {
        for a in s.iter().filter(|a| a.len() <= w) {
            let mut area = a.clone();
            area.resize(w, 0);
            for b in &s {
                // The key zero-padded to W, or its first W bytes.
                let bw = &b[..b.len().min(w)];
                let want = padded(&area, bw);
                let first = (0..w).find(|&i| area[i] != bw.get(i).copied().unwrap_or(0));
                for from in (0..=first.unwrap_or(w)).step_by(8) {
                    match area_key_cmp(&area, b, from, w) {
                        None => assert!(want.is_eq() && first.is_none(), "{area:?} {b:?}"),
                        Some((o, at)) => assert_eq!((o, Some(at)), (want, first), "{area:?} {b:?} from {from}"),
                    }
                }
                if b.len() <= 64 {
                    let mut node = vec![0u8; 8];
                    node.extend(&area);
                    let k = KeyWords::of(b);
                    assert_eq!(area_cmp(&node, 8, 0, w / 8, &k), Some(want), "{area:?} {b:?}");
                }
            }
        }
    }
}

#[test]
fn a_node_search_by_words_finds_what_a_linear_scan_finds() {
    // Nine sorted 16-byte key areas, each followed by a field.
    let words = [1u64, 3, 3, 7, 0x0100, 0x0101, 0xFF00, u64::MAX - 1, u64::MAX];
    let mut node = vec![0u8; 8 + 9 * 24];
    for (i, w) in words.iter().enumerate() {
        node[8 + 24 * i..16 + 24 * i].copy_from_slice(&w.to_be_bytes());
    }
    for probe in [0u64, 1, 2, 3, 4, 0xFF, 0x0100, 0x0102, 0xFF00, u64::MAX] {
        let k = KeyWords::of(&probe.to_be_bytes());
        let below = words.iter().filter(|&&w| w < probe).count();
        let not_above = words.iter().filter(|&&w| w <= probe).count();
        for (rank, want) in [(Ordering::Greater, below), (Ordering::Less, not_above)] {
            assert_eq!(search_fits::<2, 24, _>(&node, 9, &k, |_| rank), Some(Err(want)), "{probe:#x}");
        }
        match search_fits::<2, 24, _>(&node, 9, &k, |_| Ordering::Equal) {
            Some(Ok(i)) => assert_eq!(words[i], probe),
            r => assert!(!words.contains(&probe) && r == Some(Err(below)), "{probe:#x}"),
        }
    }
    // An entry outside the node is no answer.
    let k = KeyWords::of(&[1]);
    assert_eq!(search_fits::<2, 24, _>(&node, 12, &k, |_| Ordering::Less), None);
}

#[test]
fn key_words_are_the_zero_padded_key() {
    for len in 0..=80usize {
        let key: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(37).wrapping_add(1)).collect();
        let want = |i: usize| {
            let mut b = [0u8; 8];
            for (j, x) in b.iter_mut().enumerate() {
                *x = key.get(8 * i + j).copied().unwrap_or(0);
            }
            u64::from_le_bytes(b)
        };
        for n in 0..=8usize {
            let k = KeyWords::first(&key, n);
            for i in 0..8 {
                assert_eq!(k.w[i], if i < n { want(i) } else { 0 }, "length {len}, {n} words, word {i}");
            }
        }
        assert_eq!(KeyWords::of(&key).w, KeyWords::first(&key, 8).w, "length {len}");
    }
}

/// Sorted key areas of N words sharing first words in runs, each followed by a field, searched for keys
/// between and on them: what a linear scan finds.
fn search_at<const N: usize, const KS: usize>(r: &mut XorShift) {
    let mut areas: Vec<Vec<u8>> = (0..60)
        .map(|_| {
            let mut a: Vec<u8> = (0..8 * N).map(|_| [0u8, 1, 0x7F, 0x80, 0xFF][r.below(5) as usize]).collect();
            // Most areas share their first bytes, so that later words decide.
            if r.below(4) != 0 {
                a[..8 * N - 1].fill(0x41);
            }
            a
        })
        .collect();
    areas.sort();
    areas.dedup();
    let mut node = vec![0u8; 8];
    for a in &areas {
        node.extend(a);
        node.extend([0u8; 8]);
    }
    let mut probes = areas.clone();
    for a in &areas {
        for d in [-1i16, 1] {
            let mut p = a.clone();
            let last = p.len() - 1;
            p[last] = (p[last] as i16 + d).clamp(0, 255) as u8;
            probes.push(p);
        }
    }
    for p in &probes {
        let below = areas.iter().filter(|a| a.as_slice() < p.as_slice()).count();
        let not_above = areas.iter().filter(|a| a.as_slice() <= p.as_slice()).count();
        // The key as W bytes, and less its trailing zeros (a shorter key standing for its zero-padded area).
        let trimmed = &p[..p.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1)];
        for key in [p.as_slice(), trimmed] {
            let k = KeyWords::of(key);
            for (rank, want) in [(Ordering::Greater, below), (Ordering::Less, not_above)] {
                let got = search_fits::<N, KS, _>(&node, areas.len(), &k, |_| rank);
                assert_eq!(got, Some(Err(want)), "{N} words, {key:?}");
            }
            match search_fits::<N, KS, _>(&node, areas.len(), &k, |_| Ordering::Equal) {
                Some(Ok(i)) => assert_eq!(&areas[i], p, "{N} words"),
                got => assert!(!areas.contains(p) && got == Some(Err(below)), "{N} words, {key:?}"),
            }
        }
    }
}

#[test]
fn node_searches_of_every_width_find_what_a_linear_scan_finds() {
    let mut r = XorShift(0x5EA7);
    search_at::<1, 16>(&mut r);
    search_at::<2, 24>(&mut r);
    search_at::<3, 32>(&mut r);
    search_at::<4, 40>(&mut r);
    search_at::<5, 48>(&mut r);
    search_at::<6, 56>(&mut r);
    search_at::<7, 64>(&mut r);
    search_at::<8, 72>(&mut r);
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
    while let Some(x) = c.next(t.nodes.shape, &t.nodes, Bound::<&KeyProbe<'_>>::Unbounded).unwrap() {
        out.push(x);
        if out.len() == 4 {
            break;
        }
    }
    out
}

/// A tree of (W, V) holding keys with many equal key areas (a small alphabet, trailing zeros, keys longer
/// than W and than 64 bytes), rows, tombstones and count entries, searched with every kind of probe through
/// both ways, at rid base `base`.
fn searches_agree(w: u64, v: u64, seed: u64, base: u64) {
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
        let e = Ent { key, rank, rid, tomb: rank == 1 && r.below(4) == 0, value: value.clone() };
        t.insert(&e);
    }
    t.nodes.rid_base = base;
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
        // A rid the tree holds, as an absolute rid (the stored one plus the base), or one it may not.
        let x = r.below(400);
        let rid = if r.below(4) == 0 { x } else { base.checked_add(x).unwrap_or(x) };
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
        for (seed, base) in [(1, 0), (2, 1 << 60), (3, u64::MAX - 7)] {
            searches_agree(w, v, seed + w, base);
        }
    }
}
