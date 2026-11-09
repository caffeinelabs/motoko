//! Group counts of secondary indexes against a model: each index's distinct keys with their row counts, in key
//! order, read a few groups a call with inserts, updates that move rows between keys, removes and maintenance
//! between the calls. Indexes on a Text (short keys and keys longer than the tree's key area), an option Nat
//! (null keys first), a Nat of many distinct values, and two of two parts (a Bool and an option Nat; an Int
//! and a Text) group by their full keys. Each call hands out the model's groups above the key it starts after,
//! the first `limit` of them, and fewer only when none is left; every key handed out decodes to its group's
//! values. A buffer too short for a key is `Scratch` with the bytes it needs, and the walk resumes after the
//! last key it handed out.

use std::collections::BTreeMap;

use native_table::fault::{Fault, ImageId};
use native_table::key::enc::{dec_bool, dec_bytes, int_header};
use native_table::schema::IndexDecl;
use native_table::table::{Table, Upsert};
use native_table::value::ty;
use native_table::{Progress, Wake};
use nt_testhost::rows::{schema_indexed, t, Src, V};
use nt_testhost::{TestBudget, VecHost, VecScratch, VecSink, XorShift, FILLS};

fn id(col: u32) -> u32 {
    col << 16
}

const S: u32 = 1;
const O: u32 = 2;
const A: u32 = 3;
const F: u32 = 4;
const I: u32 = 5;

/// The indexes: id and parts as (column, nullable).
const INDEXES: [(u8, &[(u32, bool)]); 5] = [
    (1, &[(S, false)]),
    (2, &[(O, true)]),
    (3, &[(A, false)]),
    (4, &[(F, false), (O, true)]),
    (5, &[(I, false), (S, false)]),
];

/// {key Nat, s Text, o ?Nat, a Nat, f Bool, i Int}, with the indexes above, each of W 16.
fn schema() -> Vec<u8> {
    let cols = [
        t::scalar(ty::NAT),
        t::scalar(ty::TEXT),
        t::opt(t::scalar(ty::NAT)),
        t::scalar(ty::NAT),
        t::scalar(ty::BOOL),
        t::scalar(ty::INT),
    ];
    let decls: Vec<IndexDecl> = INDEXES
        .iter()
        .map(|&(ix, parts)| {
            let mut part = [(0u32, false); 7];
            for (p, &(c, n)) in part.iter_mut().zip(parts) {
                *p = (id(c), n);
            }
            IndexDecl { id: ix, flags: 0, w: 16, parts: parts.len() as u8, part }
        })
        .collect();
    schema_indexed(&cols, &decls).0
}

/// The canonical signed LEB128 of an i64.
fn sleb(mut x: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let b = (x & 0x7f) as u8;
        x >>= 7;
        if (x == 0 && b & 0x40 == 0) || (x == -1 && b & 0x40 != 0) {
            out.push(b);
            return out;
        }
        out.push(b | 0x80);
    }
}

/// Texts that repeat: short ones, ones longer than W, and a few of a few hundred bytes.
fn text_pool(r: &mut XorShift) -> Vec<Vec<u8>> {
    (0..30)
        .map(|i| {
            let len = match i % 10 {
                0..=5 => r.below(12) as usize,
                6..=8 => 17 + r.below(40) as usize,
                _ => 200 + r.below(200) as usize,
            };
            (0..len).map(|_| b'a' + r.below(3) as u8).collect()
        })
        .collect()
}

/// A row with key k: o null one time in five, a among `spread` values.
fn row(r: &mut XorShift, texts: &[Vec<u8>], k: u64, spread: u64) -> Vec<(u32, V)> {
    let s = if r.below(10) == 0 {
        (0..r.below(30)).map(|_| b'a' + r.below(26) as u8).collect()
    } else {
        texts[r.below(texts.len() as u64) as usize].clone()
    };
    vec![
        (id(0), V::U(k)),
        (id(S), V::Bytes(s)),
        (id(O), if r.below(5) == 0 { V::Null } else { V::U(r.below(40)) }),
        (id(A), V::U(r.below(spread) * 1_000_003)),
        (id(F), V::Bool(r.below(2) == 0)),
        (id(I), V::Int(sleb(r.below(9) as i64 - 4))),
    ]
}

/// key → row as written.
type Model = BTreeMap<u64, Vec<(u32, V)>>;

/// A part's value as decoded from a key: null, a Bool, a Nat or Int, or bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum P {
    Null,
    Bool(bool),
    Int(i128),
    Bytes(Vec<u8>),
}

fn part_of(v: &V) -> P {
    match v {
        V::Null => P::Null,
        V::Bool(b) => P::Bool(*b),
        V::U(x) => P::Int(i128::from(*x)),
        V::Bytes(b) => P::Bytes(b.clone()),
        V::Int(leb) => {
            let (mut x, mut shift) = (0i128, 0u32);
            for &b in leb {
                x |= i128::from(b & 0x7f) << shift;
                shift += 7;
            }
            if leb.last().is_some_and(|&b| b & 0x40 != 0) {
                x -= 1i128 << shift;
            }
            P::Int(x)
        }
        other => panic!("{other:?}"),
    }
}

/// Index `ix`'s key decoded part by part, with the crate's key decoders.
fn decode(ix: usize, key: &[u8]) -> Vec<P> {
    let im = ImageId::Runtime;
    let mut pos = 0u64;
    let mut out = Vec::new();
    for &(c, nullable) in INDEXES[ix].1 {
        if nullable {
            let (some, at) = dec_bool(key, pos, im).unwrap();
            pos = at;
            if !some {
                out.push(P::Null);
                continue;
            }
        }
        match c {
            S => {
                let mut v = VecSink::default();
                pos = dec_bytes(key, pos, &mut v, im).unwrap();
                out.push(P::Bytes(v.0));
            }
            F => {
                let (b, at) = dec_bool(key, pos, im).unwrap();
                pos = at;
                out.push(P::Bool(b));
            }
            _ => {
                let (neg, n, at) = int_header(key, pos, im).unwrap();
                let mag = &key[at as usize..(at + n) as usize];
                let x = mag.iter().fold(0i128, |acc, &b| (acc << 8) | i128::from(if neg { !b } else { b }));
                out.push(P::Int(if neg { -x } else { x }));
                pos = at + n;
            }
        }
    }
    assert_eq!(pos as usize, key.len(), "index {}: the key ends after its parts", INDEXES[ix].0);
    out
}

/// Index `ix`'s groups in the model: each key, as the table encodes the row's cells for the index, with its
/// parts' values and its rows.
fn model_groups(tb: &Table<'_, VecHost>, ix: usize, m: &Model) -> BTreeMap<Vec<u8>, (Vec<P>, i128)> {
    let mut out: BTreeMap<Vec<u8>, (Vec<P>, i128)> = BTreeMap::new();
    let mut buf = vec![0u8; 1 << 12];
    for x in m.values() {
        let n = tb.index_key(INDEXES[ix].0, &mut Src::new(x), &mut buf).unwrap();
        let parts =
            INDEXES[ix].1.iter().map(|&(c, _)| part_of(&x.iter().find(|(l, _)| *l == id(c)).unwrap().1)).collect();
        out.entry(buf[..n].to_vec()).or_insert((parts, 0)).1 += 1;
    }
    out
}

/// One call of index `ix`'s group walk after `after`, checked against the model: its groups, and how many.
fn call(tb: &Table<'_, VecHost>, m: &Model, ix: usize, after: Option<&[u8]>, limit: u64) -> Vec<(Vec<u8>, i128)> {
    let mut buf = vec![0u8; 3 * 1_024];
    let mut got = Vec::new();
    let n = tb
        .group_counts(INDEXES[ix].0, after, limit, &mut buf, |k, c| {
            got.push((k.to_vec(), c));
            Ok(())
        })
        .unwrap();
    assert_eq!(n, got.len() as u64);
    let model = model_groups(tb, ix, m);
    let above: Vec<_> = model.iter().filter(|(k, _)| after.is_none_or(|a| k.as_slice() > a)).collect();
    let want: Vec<(Vec<u8>, i128)> = above.iter().take(limit as usize).map(|(k, (_, c))| ((*k).clone(), *c)).collect();
    assert_eq!(got, want, "index {}: the groups after {after:?}, {limit} at most", INDEXES[ix].0);
    assert_eq!(n < limit, above.len() < limit as usize, "fewer than the limit only at the end");
    for (k, _) in &got {
        assert_eq!(decode(ix, k), model[k].0, "index {}: a key decodes to its group's values", INDEXES[ix].0);
    }
    got
}

fn upsert(tb: &mut Table<'_, VecHost>, m: &mut Model, x: Vec<(u32, V)>) {
    let k = match x[0].1 {
        V::U(k) => k,
        _ => unreachable!(),
    };
    match tb.upsert(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x)) {
        Ok((Upsert::Inserted(_) | Upsert::Replaced(_), _)) => {}
        Err(Fault::Busy) => {
            // A refused update of a sealed row: removed and inserted again.
            assert!(tb.remove(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x[..1])).unwrap().0.is_some());
            assert!(matches!(
                tb.upsert(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x)).unwrap().0,
                Upsert::Inserted(_)
            ));
        }
        Err(e) => panic!("upsert: {e:?}"),
    }
    m.insert(k, x);
}

fn remove(tb: &mut Table<'_, VecHost>, m: &mut Model, k: u64) {
    let gone = tb.remove(&mut VecScratch::new(FILLS[0]), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0;
    assert_eq!(gone.is_some(), m.remove(&k).is_some());
}

/// A random write: a new row, an update (a new row, or one indexed column moved to another key), or a remove.
fn write(tb: &mut Table<'_, VecHost>, m: &mut Model, r: &mut XorShift, texts: &[Vec<u8>], next: &mut u64) {
    match r.below(10) {
        0..=3 => {
            let x = row(r, texts, *next, 1_200);
            *next += 1;
            upsert(tb, m, x);
        }
        4..=7 if !m.is_empty() => {
            let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            let mut x = m[&k].clone();
            let y = row(r, texts, k, 1_200);
            match r.below(4) {
                0 => x = y,
                c => {
                    let col = [S, O, A, F, I][(c as usize + r.below(2) as usize) % 5] as usize;
                    x[col] = y[col].clone();
                }
            }
            upsert(tb, m, x);
        }
        _ if !m.is_empty() => {
            let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            remove(tb, m, k);
        }
        _ => {}
    }
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

#[test]
fn group_counts_model() {
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 43, &schema()).unwrap();
    let mut m: Model = BTreeMap::new();
    let mut r = XorShift(0x6E0A_C0DE);
    let texts = text_pool(&mut r);
    let mut next = 0u64;

    // Every index is empty: no group.
    for ix in 0..INDEXES.len() {
        assert!(call(&tb, &m, ix, None, 10).is_empty());
    }
    while next < 12_000 {
        let x = row(&mut r, &texts, next, 1_200);
        next += 1;
        upsert(&mut tb, &mut m, x);
    }
    maintain_all(&mut tb);

    // Each index walked whole, a few groups a call, with writes and now and then maintenance between calls.
    let (mut calls, mut groups, mut writes) = (0u64, 0u64, 0u64);
    for round in 0..3 {
        for ix in 0..INDEXES.len() {
            let mut after: Option<Vec<u8>> = None;
            loop {
                let limit = match r.below(8) {
                    0 => 1,
                    1 => 200,
                    _ => 1 + r.below(40),
                };
                let got = call(&tb, &m, ix, after.as_deref(), limit);
                calls += 1;
                groups += got.len() as u64;
                if (got.len() as u64) < limit {
                    break;
                }
                after = got.last().map(|(k, _)| k.clone());
                for _ in 0..r.below(12) {
                    write(&mut tb, &mut m, &mut r, &texts, &mut next);
                    writes += 1;
                }
                if r.below(20) == 0 {
                    maintain_all(&mut tb);
                }
            }
        }
        // A limit of 0 hands out nothing; an index the table does not have is `BadRef`.
        assert!(call(&tb, &m, round % INDEXES.len(), None, 0).is_empty());
        let mut buf = vec![0u8; 3 * 16];
        assert_eq!(tb.group_counts(9, None, 10, &mut buf, |_, _| Ok(())), Err(Fault::BadRef));
    }
    let distinct: Vec<usize> = (0..INDEXES.len()).map(|ix| model_groups(&tb, ix, &m).len()).collect();
    assert!(distinct[2] > 900, "the Nat index has many distinct keys: {distinct:?}");
    assert!(model_groups(&tb, 1, &m).keys().next() == Some(&vec![0u8]), "the null key comes first");

    // A buffer of 3·W meets Text keys longer than its third: `Scratch` with the bytes it needs, and the walk
    // resumes after the last key it handed out with a buffer of that size.
    let mut got: Vec<(Vec<u8>, i128)> = Vec::new();
    let (mut size, mut grew) = (48usize, 0u64);
    loop {
        let mut buf = vec![0u8; size];
        let after = got.last().map(|(k, _)| k.clone());
        let r = tb.group_counts(1, after.as_deref(), u64::MAX, &mut buf, |k, c| {
            got.push((k.to_vec(), c));
            Ok(())
        });
        match r {
            Ok(_) => break,
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size);
                size = need as usize;
                grew += 1;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    let want: Vec<(Vec<u8>, i128)> = model_groups(&tb, 0, &m).into_iter().map(|(k, (_, c))| (k, c)).collect();
    assert_eq!(got, want);
    assert!(grew > 0, "a long key needed a larger buffer");

    // Every row removed: no group is left.
    for k in m.keys().copied().collect::<Vec<_>>() {
        remove(&mut tb, &mut m, k);
    }
    for ix in 0..INDEXES.len() {
        assert!(call(&tb, &m, ix, None, 10).is_empty());
    }
    println!("calls {calls}, groups {groups}, writes {writes}, distinct keys {distinct:?}, buffer grew {grew} times");
}
