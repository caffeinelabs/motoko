//! A table's secondary indexes: a schema's index declarations are checked when a table is made with it, and a
//! count, a walk and a top name an index the table declares. Rows whose primary key and indexed text are longer
//! than their trees' key areas hand their key-log records back as they are updated and removed. Texts of up to
//! 100,000 bytes under three indexes, and updates of rows in every state (the open group, sealed groups, their
//! patches included), against a model of the rows: every index's counts and walks and every integer total equal
//! the model's. The table root holds the shape its schema image gives, which a reopened table takes.
//!
//! Each run is made under both fill patterns and must give the same answers and the same host calls.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use native_table::consts::COMPACT_DEAD;
use native_table::fault::Fault;
use native_table::index::area_nodes::KeyLog;
use native_table::index::secondary::{Secondary, Start};
use native_table::key::{KeyKind, KeyPart, KeySpec};
use native_table::schema::IndexDecl;
use native_table::table::jobs::Ran;
use native_table::table::layout::{Shape, STORED_BYTES};
use native_table::table::primary::Primary;
use native_table::table::{buffer, read, root, Table, Upsert, KEY_STACK};
use native_table::tree::Dir;
use native_table::value::ty;
use native_table::{PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{schema_indexed, schema_versioned, t, Src, VCol, V};
use nt_testhost::{Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

fn id(col: u32) -> u32 {
    col << 16
}

/// Scratch for keys longer than a point operation's stack.
fn key_scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

/// Columns: 0 the key (Nat), 1 Text, 2 i64, 3 ?Bool, 4 a u8 of three values, 5 Int, 6 the unit, 7 i64 (no
/// index).
fn columns() -> Vec<Vec<u8>> {
    vec![
        t::scalar(ty::NAT),
        t::scalar(ty::TEXT),
        t::scalar(ty::I64),
        t::opt(t::scalar(ty::BOOL)),
        t::scalar(ty::U8),
        t::scalar(ty::INT),
        t::scalar(ty::UNIT),
        t::scalar(ty::I64),
    ]
}

fn decl(index: u8, w: u16, parts: &[(u32, bool)]) -> IndexDecl {
    let mut part = [(0u32, false); 7];
    for (p, &x) in part.iter_mut().zip(parts) {
        *p = x;
    }
    IndexDecl { id: index, flags: 0, w, parts: parts.len() as u8, part }
}

/// An index: id, W, and parts as (column, nullable, key kind).
type Ix = (u8, u16, &'static [(u32, bool, KeyKind)]);

/// Every index a test declares: the five a table is made with (`INDEXES`), and one on column 8, which a schema
/// change adds.
const ALL: [Ix; 6] = [
    (1, 8, &[(4, false, KeyKind::U8)]),
    (2, 16, &[(3, true, KeyKind::Bool), (2, false, KeyKind::I64)]),
    (3, 16, &[(1, false, KeyKind::Text)]),
    (4, 8, &[(5, false, KeyKind::Int)]),
    (5, 8, &[(6, false, KeyKind::Unit)]),
    (6, 8, &[(8, false, KeyKind::Nat)]),
];

/// The indexes a table is made with.
const INDEXES: &[Ix] = ALL.split_at(5).0;

/// The declarations of the indexes of `ALL` whose ids `ids` names.
fn decls_of(ids: &[u8]) -> Vec<IndexDecl> {
    ALL.iter()
        .filter(|x| ids.contains(&x.0))
        .map(|&(ix, w, parts)| {
            let p: Vec<(u32, bool)> = parts.iter().map(|&(c, n, _)| (id(c), n)).collect();
            decl(ix, w, &p)
        })
        .collect()
}

fn decls() -> Vec<IndexDecl> {
    decls_of(&[1, 2, 3, 4, 5])
}

/// The canonical signed LEB128 of an i128.
fn sleb(mut x: i128) -> Vec<u8> {
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

/// The texts rows draw from most of the time, so that keys repeat: short ones, ones longer than W = 16, and
/// a few longer than a row holds inline.
fn text_pool(r: &mut XorShift) -> Vec<Vec<u8>> {
    (0..40)
        .map(|i| {
            let len = match i % 10 {
                0..=5 => r.below(12) as usize,
                6..=8 => 17 + r.below(80) as usize,
                _ => 2_100 + r.below(900) as usize,
            };
            (0..len).map(|_| b'a' + r.below(3) as u8).collect()
        })
        .collect()
}

/// A row with key k.
fn row(r: &mut XorShift, texts: &[Vec<u8>], k: u64) -> Vec<(u32, V)> {
    let text = if r.below(8) == 0 {
        let n = r.below(40) as usize;
        (0..n).map(|_| b'a' + r.below(26) as u8).collect()
    } else {
        texts[r.below(texts.len() as u64) as usize].clone()
    };
    let int = match r.below(10) {
        0 => sleb(i128::from(r.next_u64() % 4) << 70),
        _ => sleb(i128::from(r.below(7)) - 3),
    };
    vec![
        (id(0), V::U(k)),
        (id(1), V::Bytes(text)),
        (id(2), V::I(r.below(12) as i64 - 6)),
        (id(3), if r.below(4) == 0 { V::Null } else { V::Bool(r.below(2) == 0) }),
        (id(4), V::U(r.below(3))),
        (id(5), V::Int(int)),
        (id(6), V::Unit),
        (id(7), V::I(r.next_u64() as i64)),
    ]
}

// ---------------------------------------------------------------- harness

/// A digest of host calls, folded as they are taken so long runs keep no log.
#[derive(Default)]
struct LogHash(std::collections::hash_map::DefaultHasher, u64);

impl LogHash {
    fn take(&mut self, h: &mut VecHost) {
        for c in h.take_log() {
            self.1 += 1;
            let r = |r: Region| match r {
                Region::Block(b) => (1u64, b),
                Region::Page(p) => (2u64, p),
            };
            match c {
                Call::Get(a) => (0u8, r(a), (0, 0)).hash(&mut self.0),
                Call::GetMut(a) => (1u8, r(a), (0, 0)).hash(&mut self.0),
                Call::PairMut(a, b) => (2u8, r(a), r(b)).hash(&mut self.0),
                Call::ReadWrite(a, b) => (3u8, r(a), r(b)).hash(&mut self.0),
                Call::PageAlloc(p) => (4u8, (p, 0), (0, 0)).hash(&mut self.0),
                Call::PageFree(p) => (5u8, (p, 0), (0, 0)).hash(&mut self.0),
            }
        }
    }
}

/// Runs `run` once per fill pattern on a fresh host; the results and the host-call logs must be equal.
fn per_fill<T: PartialEq + std::fmt::Debug>(run: impl Fn(&mut VecHost, &mut LogHash) -> T) -> T {
    let mut out: Vec<(T, (u64, u64))> = FILLS
        .iter()
        .map(|&f| {
            let mut h = VecHost::new(f);
            let mut lh = LogHash::default();
            let r = run(&mut h, &mut lh);
            lh.take(&mut h);
            (r, (lh.0.finish(), lh.1))
        })
        .collect();
    let b = out.pop().unwrap();
    let a = out.pop().unwrap();
    assert_eq!(a.0, b.0, "results differ between fill patterns");
    assert_eq!(a.1, b.1, "host-call logs differ between fill patterns");
    a.0
}

fn everything() -> TestBudget {
    TestBudget::new(u64::MAX)
}

/// Runs maintenance until nothing is pending.
fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&everything()).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

// ---------------------------------------------------------------- the model

/// key → (rid, row as written).
type Model = BTreeMap<u64, (u64, Vec<(u32, V)>)>;

/// The cells of index i's parts in `row`, as a count's key source takes them.
fn key_cells(i: usize, row: &[(u32, V)]) -> Vec<(u32, V)> {
    ALL[i].2.iter().map(|&(c, _, _)| row.iter().find(|(l, _)| *l == id(c)).unwrap().clone()).collect()
}

/// Index i's keys in the model: each key's cells and its rows.
fn model_counts(i: usize, m: &Model) -> HashMap<String, (Vec<(u32, V)>, i128)> {
    let mut out: HashMap<String, (Vec<(u32, V)>, i128)> = HashMap::new();
    for (_, row) in m.values() {
        let cells = key_cells(i, row);
        out.entry(format!("{cells:?}")).or_insert((cells, 0)).1 += 1;
    }
    out
}

/// A key of index i no row holds, when the index has one.
fn absent(i: usize) -> Option<Vec<(u32, V)>> {
    match ALL[i].0 {
        2 => Some(vec![(id(3), V::Bool(true)), (id(2), V::I(1_000))]),
        3 => Some(vec![(id(1), V::text("zzz"))]),
        4 => Some(vec![(id(5), V::Int(sleb(12_345)))]),
        _ => None,
    }
}

/// Index i's key spec, which gives the end of each key its group walk hands out.
fn spec_parts(i: usize) -> Vec<KeyPart> {
    ALL[i].2.iter().map(|&(c, nullable, kind)| KeyPart { leaf: id(c), kind, nullable }).collect()
}

/// Index i's group walk: each key's count, in key order.
fn walk(tb: &Table<'_, VecHost>, i: usize) -> Vec<i128> {
    let ix = Secondary::read(tb.host(), tb.root(), ALL[i].0).unwrap();
    let parts = spec_parts(i);
    let spec = KeySpec { parts: &parts };
    let mut out = vec![0u8; 1 << 14];
    let mut after: Option<Vec<u8>> = None;
    let mut counts = Vec::new();
    while let Some((len, c)) = ix.group_next(tb.host(), &spec, after.as_deref(), &mut out).unwrap() {
        if let Some(a) = &after {
            assert!(out[..len] > a[..], "the walk's keys rise");
        }
        counts.push(c);
        after = Some(out[..len].to_vec());
    }
    counts
}

/// A row as a walk hands it out: its key under the index and its rid.
type Row = (Vec<u8>, u64);

/// Index i's sorted walk from `start` in direction `dir`, at most `limit` rows.
fn sorted(tb: &Table<'_, VecHost>, i: usize, dir: Dir, start: Start<'_>, limit: u64) -> Vec<Row> {
    walk_grown(tb, ALL[i].0, dir, start, limit).0
}

/// Index `ix`'s sorted walk from `start` in direction `dir`, at most `limit` rows, through a buffer that starts
/// at 48 bytes and grows to the bytes each `Scratch` names, the walk resumed after the last row it handed out;
/// with the times it grew.
fn walk_grown(tb: &Table<'_, VecHost>, ix: u8, dir: Dir, start: Start<'_>, limit: u64) -> (Vec<Row>, u64) {
    let (mut out, mut size, mut grew): (Vec<Row>, usize, u64) = (Vec::new(), 48, 0);
    loop {
        let mut buf = vec![0u8; size];
        let last = out.last().cloned();
        let from = match &last {
            Some((k, rid)) => Start::After(k, *rid),
            None => start,
        };
        let mut got = Vec::new();
        let r = tb.walk(ix, dir, from, limit - out.len() as u64, &mut buf, |k, rid| {
            got.push((k.to_vec(), rid));
            Ok(())
        });
        out.extend(got);
        match r {
            Ok(_) => return (out, grew),
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size, "index {ix}: a need of {need} bytes for a buffer of {size}");
                (size, grew) = (need as usize, grew + 1);
            }
            Err(e) => panic!("index {ix}: {e:?}"),
        }
    }
}

/// `f` with a buffer that starts at 48 bytes and grows to the bytes each `Scratch` names.
fn grown<T>(mut f: impl FnMut(&mut [u8]) -> Result<T, Fault>) -> (T, Vec<u8>) {
    let mut size = 48usize;
    loop {
        let mut buf = vec![0u8; size];
        match f(&mut buf) {
            Ok(x) => return (x, buf),
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size, "a need of {need} bytes for a buffer of {size}");
                size = need as usize;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// Index i's rows in the model, in the walk's order in direction `dir`: keys rising forward and falling
/// backward, the rows of one key in rid order.
fn model_rows(tb: &Table<'_, VecHost>, i: usize, m: &Model, dir: Dir) -> Vec<Row> {
    let mut buf = vec![0u8; 8_192];
    let mut rows: Vec<Row> = m
        .values()
        .map(|(rid, row)| {
            let n = tb.index_key(ALL[i].0, &mut Src::new(&key_cells(i, row)), &mut buf).unwrap();
            (buf[..n].to_vec(), *rid)
        })
        .collect();
    rows.sort();
    if dir == Dir::Rev {
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    }
    rows
}

/// Index i's sorted walks and ends against the model.
fn check_sorted(tb: &Table<'_, VecHost>, i: usize, m: &Model) {
    let ix = ALL[i].0;
    for dir in [Dir::Fwd, Dir::Rev] {
        let want = model_rows(tb, i, m, dir);
        assert!(sorted(tb, i, dir, Start::End, u64::MAX) == want, "index {ix}, {dir:?}: the sorted walk");
        // In pieces, each resumed after the last row of the one before.
        let mut got = sorted(tb, i, dir, Start::End, 97);
        while let Some((k, rid)) = got.last().cloned() {
            let more = sorted(tb, i, dir, Start::After(&k, rid), 97);
            if more.is_empty() {
                break;
            }
            got.extend(more);
        }
        assert!(got == want, "index {ix}, {dir:?}: the walk in pieces");
        // From the key of a row in the middle: every row of that key, then the keys after it.
        if let Some((k, _)) = want.get(want.len() / 2) {
            let from = want.iter().position(|(x, _)| x == k).unwrap();
            let got = sorted(tb, i, dir, Start::Key(k), 30);
            assert!(got[..] == want[from..want.len().min(from + 30)], "index {ix}, {dir:?}: from a key");
        }
    }
    // The smallest and largest keys whose first part is not null (a nullable part's tag 00).
    let nullable = ALL[i].2[0].1;
    let keys: Vec<Vec<u8>> =
        model_rows(tb, i, m, Dir::Fwd).into_iter().map(|x| x.0).filter(|k| !(nullable && k[0] == 0)).collect();
    let (min, out) = grown(|out| tb.min_key(ix, out));
    let min = min.map(|n| out[..n].to_vec());
    let (max, out) = grown(|out| tb.max_key(ix, out));
    let max = max.map(|n| out[..n].to_vec());
    assert_eq!((min, max), (keys.first().cloned(), keys.last().cloned()), "index {ix}: the ends");
}

/// Every index against the model: every key's count, a key no row holds, the group walk, and the sorted walks.
fn check(tb: &Table<'_, VecHost>, m: &Model) -> u64 {
    check_live(tb, m, &[1, 2, 3, 4, 5])
}

/// `check` of the indexes `live` names; every other one is absent (`BadRef`).
fn check_live(tb: &Table<'_, VecHost>, m: &Model, live: &[u8]) -> u64 {
    let mut keys = 0u64;
    for i in 0..ALL.len() {
        let ix = ALL[i].0;
        if !live.contains(&ix) {
            assert_eq!(
                tb.count(&mut key_scratch(), ix, &mut Src::new(&[])),
                Err(Fault::BadRef),
                "index {ix} is removed"
            );
            continue;
        }
        let counts = model_counts(i, m);
        for (k, (cells, c)) in &counts {
            assert_eq!(tb.count(&mut key_scratch(), ix, &mut Src::new(cells)).unwrap(), *c, "index {ix}, key {k}");
        }
        if let Some(cells) = absent(i) {
            assert_eq!(
                tb.count(&mut key_scratch(), ix, &mut Src::new(&cells)).unwrap(),
                0,
                "index {ix}: a key no row holds"
            );
        }
        let mut want: Vec<i128> = counts.values().map(|x| x.1).collect();
        let mut got = walk(tb, i);
        want.sort();
        got.sort();
        assert_eq!(got, want, "index {ix}: the group walk");
        check_sorted(tb, i, m);
        keys += counts.len() as u64;
    }
    keys
}

// ---------------------------------------------------------------- tests

/// Sets leaf `leaf` of the row with key k to `v`: an upsert of the model's row with that cell changed, keeping
/// the model.
fn set_cell(tb: &mut Table<'_, VecHost>, m: &mut Model, k: u64, leaf: u32, v: V) {
    let row = &mut m.get_mut(&k).expect("key present").1;
    row.iter_mut().find(|(l, _)| *l == leaf).unwrap().1 = v;
    let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(row)).unwrap();
    assert!(matches!(u, Upsert::Replaced(_)), "key {k} present");
}

/// How `upsert` wrote a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wrote {
    Inserted,
    Updated,
    /// An update of a sealed row.
    UpdatedSealed,
}

/// Writes a row through `upsert`, as the model expects.
fn upsert(tb: &mut Table<'_, VecHost>, m: &mut Model, k: u64, row: Vec<(u32, V)>) -> Wrote {
    let sealed = m
        .get(&k)
        .is_some_and(|x| matches!(read::find(tb.host(), tb.root(), x.0).unwrap(), Some(read::Row::Sealed { .. })));
    match tb.upsert(&mut key_scratch(), &mut Src::new(&row)) {
        Ok((Upsert::Inserted(rid), _)) => {
            assert!(!m.contains_key(&k));
            m.insert(k, (rid, row));
            Wrote::Inserted
        }
        Ok((Upsert::Replaced(rid), _)) => {
            assert_eq!(m[&k].0, rid, "an update keeps the rid");
            m.insert(k, (rid, row));
            if sealed {
                Wrote::UpdatedSealed
            } else {
                Wrote::Updated
            }
        }
        Err(e) => panic!("upsert: {e:?}"),
    }
}

#[test]
fn declarations() {
    let cols = columns();
    let make = |d: &[IndexDecl]| {
        let (img, _) = schema_indexed(&cols, d);
        let mut h = VecHost::new(FILLS[0]);
        Table::create(&mut h, 1, &img).map(|_| ())
    };
    assert!(make(&decls()).is_ok());
    let ok = decl(7, 8, &[(id(2), false)]);
    // An id outside 1..=63, or taken twice.
    assert!(make(&[decl(0, 8, &[(id(2), false)])]).is_err());
    assert!(make(&[decl(64, 8, &[(id(2), false)])]).is_err());
    assert!(make(&[ok, decl(7, 8, &[(id(1), false)])]).is_err());
    // A W its tree cannot take.
    assert!(make(&[decl(7, 12, &[(id(2), false)])]).is_err());
    assert!(make(&[decl(7, 0, &[(id(2), false)])]).is_err());
    // A part naming no leaf, or with a nullability its leaf does not have.
    assert!(make(&[decl(7, 8, &[(id(9), false)])]).is_err());
    assert!(make(&[decl(7, 8, &[(id(3), false)])]).is_err());
    assert!(make(&[decl(7, 8, &[(id(2), true)])]).is_err());
    // No part, or more than seven.
    let mut none = ok;
    none.parts = 0;
    assert!(make(&[none]).is_err());
    // A primary declaration is not a secondary index.
    let mut primary = ok;
    primary.flags = 1;
    let (img, _) = schema_indexed(&cols, &[primary]);
    let mut h = VecHost::new(FILLS[0]);
    let (tb, _) = Table::create(&mut h, 1, &img).unwrap();
    assert!(matches!(tb.count(&mut key_scratch(), 7, &mut Src::new(&[(id(2), V::I(0))])), Err(Fault::BadRef)));
    // A count names an index the table declares, and takes its parts' cells.
    let (img, _) = schema_indexed(&cols, &[ok]);
    let mut h = VecHost::new(FILLS[0]);
    let (tb, _) = Table::create(&mut h, 1, &img).unwrap();
    assert_eq!(tb.count(&mut key_scratch(), 7, &mut Src::new(&[(id(2), V::I(0))])).unwrap(), 0);
    assert!(matches!(tb.count(&mut key_scratch(), 8, &mut Src::new(&[(id(2), V::I(0))])), Err(Fault::BadRef)));
    assert!(tb.count(&mut key_scratch(), 7, &mut Src::new(&[(id(1), V::text("a"))])).is_err());
    // A walk and an end name an index the table declares.
    let buf = &mut [0u8; 192];
    assert!(matches!(tb.walk(8, Dir::Fwd, Start::End, 1, buf, |_, _| Ok(())), Err(Fault::BadRef)));
    assert!(matches!(tb.min_key(8, buf), Err(Fault::BadRef)));
    assert_eq!(tb.walk(7, Dir::Rev, Start::End, 1, buf, |_, _| Ok(())).unwrap(), 0);
    assert_eq!(tb.max_key(7, buf).unwrap(), None);
}

/// The key logs of the table's primary index and of its secondary index 1.
fn key_logs(tb: &Table<'_, VecHost>) -> (KeyLog, KeyLog) {
    let (h, tbl) = (tb.host(), tb.root());
    (Primary::read(h, tbl).unwrap().map_or(KeyLog::default(), |p| p.log), Secondary::read(h, tbl, 1).unwrap().log)
}

/// Rows whose primary key (a text of 48 bytes) and indexed text (60 to 62 bytes) are longer than their trees'
/// key areas: updates that move the indexed text among three values, and removes taken again, leave both key
/// logs at the blocks they had and the index trees' area at its pages; removing every row empties both logs.
#[test]
fn long_keys_hand_back_their_records() {
    let cols = vec![t::scalar(ty::TEXT), t::scalar(ty::TEXT)];
    let (img, _) = schema_indexed(&cols, &[decl(1, 16, &[(id(1), false)])]);
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 3, &img).unwrap();
    let row = |i: u64, v: u64| {
        vec![
            (id(0), V::Bytes(format!("{i:0>48}").into_bytes())),
            (id(1), V::Bytes(format!("{v:x<60}{i}").into_bytes())),
        ]
    };
    let key = |i: u64| vec![(id(0), V::Bytes(format!("{i:0>48}").into_bytes()))];
    for i in 0..20 {
        tb.upsert(&mut key_scratch(), &mut Src::new(&row(i, 0))).unwrap();
    }
    let (p0, s0) = key_logs(&tb);
    assert!(p0.blocks > 0 && s0.blocks > 0, "both trees use their key logs");
    // The pages of area 1, which holds every index tree and key log.
    let area1 = |tb: &Table<'_, VecHost>| native_table::area::AreaRecord::read(tb.host(), tb.root(), 1).unwrap().pages;
    let pages = area1(&tb);
    for round in 0..30_000u64 {
        let i = round % 20;
        if round % 7 == 0 {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&key(i))).unwrap().0.is_some());
        }
        tb.upsert(&mut key_scratch(), &mut Src::new(&row(i, round % 3))).unwrap();
    }
    let (p, s) = key_logs(&tb);
    assert!(p.blocks <= p0.blocks + 1 && s.blocks <= s0.blocks + 1, "key logs grew: {p:?}, {s:?}");
    assert_eq!(area1(&tb), pages, "the index trees' pages");
    for i in 0..20 {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&key(i))).unwrap().0.is_some());
    }
    assert_eq!(key_logs(&tb), (KeyLog::default(), KeyLog::default()), "every row removed");
}

/// Texts of 5,000, 20,000 and 100,000 bytes beside short ones (some sharing all but their last byte, some with
/// a zero byte, which the key encoding escapes, one a prefix of another), under three indexes: the text alone;
/// a nullable text then an i64; an i64 then the text. Inserts, updates, removes and seals against a model, with
/// every key's count through each index; removing every row empties every tree and key log.
#[test]
fn long_keys_model() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::TEXT), t::opt(t::scalar(ty::TEXT)), t::scalar(ty::I64)];
    let decls = [
        decl(1, 16, &[(id(1), false)]),
        decl(2, 64, &[(id(2), true), (id(3), false)]),
        decl(3, 8, &[(id(3), false), (id(1), false)]),
    ];
    let parts: [&[u32]; 3] = [&[1], &[2, 3], &[3, 1]];
    let (img, _) = schema_indexed(&cols, &decls);
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 3, &img).unwrap();
    let mut r = XorShift(0x10_6c3e);
    let base: Vec<u8> = (0..100_000u32).map(|i| (i % 90) as u8 + 32).collect();
    let mut pool: Vec<Vec<u8>> = vec![base[..4_999].to_vec(), b"short".to_vec(), vec![]];
    for len in [5_000usize, 20_000, 100_000] {
        let (mut last, mut zero) = (base[..len].to_vec(), base[..len].to_vec());
        *last.last_mut().unwrap() ^= 0x40;
        zero[len / 2] = 0;
        pool.extend([base[..len].to_vec(), last, zero]);
    }
    let cells = |i: usize, row: &[(u32, V)]| -> Vec<(u32, V)> {
        parts[i].iter().map(|&c| row.iter().find(|(l, _)| *l == id(c)).unwrap().clone()).collect()
    };
    let check = |tb: &Table<'_, VecHost>, m: &Model, fill: &Model| {
        for (i, (ix, _)) in [(1u8, 0), (2, 0), (3, 0)].iter().enumerate() {
            let mut keys: Vec<Vec<(u32, V)>> = m.values().map(|(_, row)| cells(i, row)).collect();
            // Keys no row holds: the long texts with another i64, or alone.
            keys.extend(
                pool.iter().map(|t| cells(i, &[(id(1), V::Bytes(t.clone())), (id(2), V::Null), (id(3), V::I(9))])),
            );
            for k in keys {
                let want = m.values().filter(|(_, row)| cells(i, row) == k).count() as i128;
                assert_eq!(tb.count(&mut key_scratch(), *ix, &mut Src::new(&k)).unwrap(), want, "index {ix}");
            }
            // The sorted walks, whole and in pieces, and the ends, over keys of any length.
            let mut buf = vec![0u8; 200_000];
            let mut rows: Vec<Row> = m
                .values()
                .chain(fill.values())
                .map(|(rid, row)| {
                    let n = tb.index_key(*ix, &mut Src::new(&cells(i, row)), &mut buf).unwrap();
                    (buf[..n].to_vec(), *rid)
                })
                .collect();
            rows.sort();
            for dir in [Dir::Fwd, Dir::Rev] {
                if dir == Dir::Rev {
                    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                }
                let (got, grew) = walk_grown(tb, *ix, dir, Start::End, u64::MAX);
                if got != rows {
                    let d = got.iter().zip(&rows).position(|(a, b)| a != b);
                    let show = |r: Option<&Row>| r.map(|r| (r.0.len(), r.0[..r.0.len().min(12)].to_vec(), r.1));
                    let d0 = d.unwrap_or(got.len().min(rows.len()));
                    panic!("index {ix}, {dir:?}: the sorted walk: {} rows, {} expected, first difference at {d:?}: {:?} vs {:?}", got.len(), rows.len(), show(got.get(d0)), show(rows.get(d0)));
                }
                assert!(grew > 0 || rows.iter().all(|r| r.0.len() <= 16), "index {ix}: the buffer grew");
                let mut pieces = walk_grown(tb, *ix, dir, Start::End, 7).0;
                while let Some((k, rid)) = pieces.last().cloned() {
                    let more = walk_grown(tb, *ix, dir, Start::After(&k, rid), 7).0;
                    if more.is_empty() {
                        break;
                    }
                    pieces.extend(more);
                }
                assert!(pieces == rows, "index {ix}, {dir:?}: the walk in pieces");
                if let Some((k, _)) = rows.get(rows.len() / 2) {
                    let from = rows.iter().position(|(x, _)| x == k).unwrap();
                    let got = walk_grown(tb, *ix, dir, Start::Key(k), 9).0;
                    assert!(got[..] == rows[from..rows.len().min(from + 9)], "index {ix}, {dir:?}: from a key");
                }
            }
            rows.sort();
            let keys: Vec<&Vec<u8>> = rows.iter().map(|x| &x.0).filter(|k| !(i == 1 && k[0] == 0)).collect();
            let (min, out) = grown(|out| tb.min_key(*ix, out));
            assert_eq!(min.map(|n| out[..n].to_vec()).as_ref(), keys.first().copied(), "index {ix}: min");
            let (max, out) = grown(|out| tb.max_key(*ix, out));
            assert_eq!(max.map(|n| out[..n].to_vec()).as_ref(), keys.last().copied(), "index {ix}: max");
        }
    };
    // The rows outside the model: the counts leave them out, the walks hand them out too.
    let (mut m, mut fill) = (Model::new(), Model::new());
    let mut sealed = 0u64;
    for step in 0..500u64 {
        let op = r.below(10);
        if op < 5 || m.is_empty() {
            let k = r.below(48);
            let pick = |r: &mut XorShift| pool[r.below(pool.len() as u64) as usize].clone();
            let opt = if r.below(4) == 0 { V::Null } else { V::Bytes(pick(&mut r)) };
            let row =
                vec![(id(0), V::U(k)), (id(1), V::Bytes(pick(&mut r))), (id(2), opt), (id(3), V::I(r.below(3) as i64))];
            upsert(&mut tb, &mut m, k, row);
        } else if op < 8 {
            let k = r.below(48);
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        } else if op < 9 {
            maintain_all(&mut tb);
            let on = |rid| matches!(read::find(tb.host(), tb.root(), rid).unwrap(), Some(read::Row::Sealed { .. }));
            sealed = sealed.max(m.values().filter(|(rid, _)| on(*rid)).count() as u64);
        }
        if step % 50 == 49 {
            check(&tb, &m, &fill);
        }
        if step == 100 {
            // A group's worth of short rows outside the model, keys and values of their own, so the rows above
            // are sealed with their group.
            for j in 0..4_100u64 {
                let t = V::Bytes(format!("filler {j}").into_bytes());
                let row = vec![(id(0), V::U(1_000 + j)), (id(1), t.clone()), (id(2), t), (id(3), V::I(-1))];
                assert_eq!(upsert(&mut tb, &mut fill, 1_000 + j, row), Wrote::Inserted, "a new row");
            }
            maintain_all(&mut tb);
        }
    }
    check(&tb, &m, &fill);
    assert!(sealed >= 1, "rows were sealed: {sealed}");
    for k in 0..48 {
        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        assert_eq!(gone, m.remove(&k).map(|x| x.0));
    }
    for k in 1_000..5_100 {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
    }
    for ix in 1..=3 {
        let s = Secondary::read(tb.host(), tb.root(), ix).unwrap();
        assert_eq!((s.root, s.nodes, s.log), (0, 0, KeyLog::default()), "index {ix} is empty");
    }
}

// ---------------------------------------------------------------- updates of buffered and sealed rows

/// Where a stored row lies as an update finds it: 0 the open group, 1 a sealed group.
fn place(tb: &Table<'_, VecHost>, rid: u64) -> usize {
    match read::find(tb.host(), tb.root(), rid).unwrap() {
        Some(read::Row::Buffered(_)) => 0,
        Some(read::Row::Sealed { .. }) => 1,
        None => panic!("rid {rid} is stored"),
    }
}

/// Each integer leaf's total is the model's sum and count, and the buffered live count counts the buffered
/// rows.
fn check_running(tb: &Table<'_, VecHost>, m: &Model) {
    let (h, tbl, shape) = (tb.host(), tb.root(), *tb.shape());
    let tail = root::u64_at(h, tbl, root::TAIL).unwrap();
    let buffered = |rid: u64| matches!(read::find(h, tbl, rid).unwrap(), Some(read::Row::Buffered(_)));
    let n_buffered = m.values().filter(|(rid, _)| buffered(*rid)).count() as u64;
    assert_eq!(buffer::u64_of(h, tail, buffer::H_BUFFERED).unwrap(), n_buffered, "buffered live rows");
    for o in 0..shape.leaves {
        let l = shape.leaf(h, o).unwrap();
        if l.key {
            continue;
        }
        let value = |row: &Vec<(u32, V)>| row.iter().find(|(x, _)| *x == l.id).unwrap().1.clone();
        if l.ty.running().is_some() {
            let (mut want, mut n) = (Big::default(), 0u64);
            for (_, row) in m.values() {
                let v = match value(row) {
                    V::Null => continue,
                    V::I(x) => Big::from_i128(i128::from(x)),
                    V::U(x) => Big::from_i128(i128::from(x)),
                    V::Int(b) => Big::from_leb(&b, true),
                    other => panic!("leaf {o}: {other:?}"),
                };
                want = want.add(&v);
                n += 1;
            }
            let mut buf = vec![0u8; 1 << 12];
            let (got, gn) = tb.sum(l.id, &mut buf).unwrap();
            assert_eq!((Big::of_total(got, &buf), gn), (want, n), "total of leaf {o}");
        }
    }
}

// ---------------------------------------------------------------- updates in every row state

/// The schema image: `columns`, with the indexes.
fn image() -> Vec<u8> {
    let vc: Vec<VCol> = columns().into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    schema_versioned(1, &vc, &decls()).0
}

/// The leaf ids no index holds.
const UNINDEXED: &[u32] = &[7 << 16];

/// Runs up to n steps; counts each kind of step that ran.
fn units(tb: &mut Table<'_, VecHost>, n: u64, ran: &mut BTreeMap<String, u64>) {
    for _ in 0..n {
        let k = tb.step().unwrap();
        if k == Ran::Nothing {
            return;
        }
        *ran.entry(format!("{k:?}")).or_insert(0) += 1;
    }
}

/// Updates of rows in every state (the open group, sealed groups, their patches included), through `upsert` and
/// through `set` of one leaf, among random inserts, removes, seals and maintenance steps (compactions): every
/// index's counts and walks, every integer total and every Bool, tag and buffered count equal the model's.
#[test]
fn updates_in_every_state() {
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 9, &image()).unwrap();
        let mut r = XorShift(0x5eed_57a7);
        let mut texts = text_pool(&mut r);
        texts.extend((0..3u8).map(|i| vec![b'x' + i; KEY_STACK + 100 + 700 * usize::from(i)]));
        let mut m = Model::new();
        let mut by = [[0u64; 2]; 2];
        let mut ran = BTreeMap::new();
        for step in 0..12_000u64 {
            if step == 6_500 {
                // A burst of removes of sealed rows: their group passes the dead positions that queue a
                // compaction, which runs among the updates that follow.
                let mut sealed: Vec<u64> =
                    m.iter().filter(|(_, (rid, _))| place(&tb, *rid) == 1).map(|(k, _)| *k).collect();
                for j in (1..sealed.len()).rev() {
                    sealed.swap(j, r.below(j as u64 + 1) as usize);
                }
                for k in sealed.into_iter().take(COMPACT_DEAD as usize + 50) {
                    let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                    assert_eq!(gone, m.remove(&k).map(|x| x.0));
                }
            }
            let op = r.below(100);
            let some_key = |r: &mut XorShift, m: &Model| *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            if op < 40 || m.is_empty() {
                let k = r.below(1 << 40);
                let new = row(&mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                match u {
                    Upsert::Inserted(rid) => assert!(m.insert(k, (rid, new)).is_none()),
                    Upsert::Replaced(rid) => assert_eq!(m.insert(k, (rid, new)).unwrap().0, rid),
                }
            } else if op < 65 {
                let k = some_key(&mut r, &m);
                let (rid, old) = m[&k].clone();
                let fresh = row(&mut r, &texts, k);
                let pick = |ls: &[u32], r: &mut XorShift| ls[r.below(ls.len() as u64) as usize];
                let new = match r.below(3) {
                    0 => fresh,
                    w => {
                        let c = if w == 1 {
                            fresh[1 + r.below(fresh.len() as u64 - 1) as usize].0
                        } else {
                            pick(UNINDEXED, &mut r)
                        };
                        let cell = fresh.iter().find(|(l, _)| *l == c).unwrap().clone();
                        old.into_iter().map(|x| if x.0 == c { cell.clone() } else { x }).collect()
                    }
                };
                by[0][place(&tb, rid)] += 1;
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                assert_eq!(u, Upsert::Replaced(rid), "an update keeps the rid");
                m.insert(k, (rid, new));
            } else if op < 85 {
                let k = some_key(&mut r, &m);
                let rid = m[&k].0;
                let fresh = row(&mut r, &texts, k);
                let cell = [fresh[1 + r.below(fresh.len() as u64 - 1) as usize].clone()];
                if cell[0].0 == id(6) {
                    continue;
                }
                by[1][place(&tb, rid)] += 1;
                set_cell(&mut tb, &mut m, k, cell[0].0, cell[0].1.clone());
            } else if op < 96 {
                let k = if r.below(6) == 0 { r.below(1 << 40) } else { some_key(&mut r, &m) };
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, m.remove(&k).map(|x| x.0));
            } else {
                let n = 1 + r.below(40);
                units(&mut tb, n, &mut ran);
            }
            if step % 1_000 == 500 {
                units(&mut tb, u64::MAX, &mut ran);
            }
            assert_eq!(tb.len().unwrap(), m.len() as u64, "step {step}");
            if step % 250 == 249 {
                check_running(&tb, &m);
            }
            if step % 1_000 == 999 {
                check(&tb, &m);
                lh.take(tb.host_mut());
            }
        }
        for (way, n) in by.iter().enumerate() {
            for (at, &c) in n.iter().enumerate() {
                assert!(c >= 1, "updates by way {way} of rows at place {at}: {by:?}");
            }
        }
        assert!(ran.get("Compaction").copied().unwrap_or(0) >= 1, "compaction steps ran: {ran:?}");
        assert!(root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap() >= 1, "groups were sealed");
        maintain_all(&mut tb);
        check(&tb, &m);
        check_running(&tb, &m);
        let mut left: Vec<u64> = m.keys().copied().collect();
        for j in (1..left.len()).rev() {
            left.swap(j, r.below(j as u64 + 1) as usize);
        }
        for k in left {
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        }
        check_running(&tb, &m);
        for &(ix, _, _) in INDEXES {
            let s = Secondary::read(tb.host(), tb.root(), ix).unwrap();
            assert_eq!((s.root, s.nodes), (0, 0), "index {ix} is empty");
        }
        (by, ran)
    });
    eprintln!("updates_in_every_state: {digest:?}");
}

/// A table of `n` rows of `columns`, and its model.
fn filled(h: &mut VecHost, n: u64) -> (Table<'_, VecHost>, Model) {
    let (mut tb, _) = Table::create(h, 9, &image()).unwrap();
    let mut r = XorShift(3);
    let mut texts = text_pool(&mut r);
    texts.push(vec![b'q'; KEY_STACK + 300]);
    let mut m = Model::new();
    for k in 0..n {
        let x = row(&mut r, &texts, k);
        let (Upsert::Inserted(rid), _) = tb.upsert(&mut key_scratch(), &mut Src::new(&x)).unwrap() else { panic!() };
        m.insert(k, (rid, x));
    }
    (tb, m)
}

/// The table root holds the shape its schema image gives, from the table's making on; a reopened table has
/// it, reads its rows and keeps its indexes. An open never uses a stored shape of another table format or of
/// another schema page, schema hash or version, nor one with any byte of those four words changed: it derives
/// the shape from the schema image again and stores it. A stored shape that passes every check is the one an
/// open uses, as it is: its hash is not checked.
#[test]
fn the_stored_shape() {
    let derived = |h: &VecHost, root: u64| Shape::open(h, root::u64_at(h, root, root::SCHEMA_PAGE).unwrap()).unwrap();
    let at = root::SHAPE as usize..(root::SHAPE + STORED_BYTES) as usize;
    let bytes = |h: &VecHost, root: u64| h.get(Region::Block(root)).unwrap()[at.clone()].to_vec();
    let set_bytes =
        |h: &mut VecHost, root: u64, b: &[u8]| h.get_mut(Region::Block(root)).unwrap()[at.clone()].copy_from_slice(b);
    // The stored shape's words 0 (`FORMAT_TABLE`), 1 (schema page), 2 (schema hash), 3 (schema version), 15
    // (the hot state's first row byte) and 24 (the hash) changed.
    let with_word = |b: &[u8], k: usize, f: &dyn Fn(u64) -> u64| {
        let mut b = b.to_vec();
        let w = u64::from_le_bytes(b[8 * k..8 * k + 8].try_into().unwrap());
        b[8 * k..8 * k + 8].copy_from_slice(&f(w).to_le_bytes());
        b
    };
    let reopen = |h: &mut VecHost, root: u64| *Table::open(h, root).unwrap().shape();
    let mut h = VecHost::new(FILLS[0]);
    let (tb, m) = filled(&mut h, 300);
    let root = tb.root();
    drop(tb);
    assert_eq!(Shape::stored(&h, root).unwrap(), Some(derived(&h, root)), "made");
    {
        let tb = Table::open(&mut h, root).unwrap();
        assert_eq!((*tb.shape(), tb.len().unwrap()), (derived(tb.host(), root), 300), "reopened");
        check(&tb, &m);
    }
    let s = derived(&h, root);
    let good = bytes(&h, root);
    let refused: [(&str, Vec<u8>); 5] = [
        ("an earlier table format", with_word(&good, 0, &|w| w - 1)),
        ("a later table format", with_word(&good, 0, &|w| w + 1)),
        ("another schema page", with_word(&good, 1, &|w| w + 1)),
        ("another schema hash", with_word(&good, 2, &|w| w ^ 1)),
        ("another schema version", with_word(&good, 3, &|w| w - 1)),
    ];
    for (what, b) in refused {
        set_bytes(&mut h, root, &b);
        assert_eq!(Shape::stored(&h, root).unwrap(), None, "{what}");
        assert_eq!(reopen(&mut h, root), s, "{what}: not used");
        assert_eq!(bytes(&h, root), good, "{what}: stored again");
    }
    // Any byte of the four words checked.
    for k in 0..32 {
        let mut b = good.clone();
        b[k] ^= 0x10;
        set_bytes(&mut h, root, &b);
        assert_eq!(Shape::stored(&h, root).unwrap(), None, "byte {k} changed");
        assert_eq!(reopen(&mut h, root), s, "byte {k} changed: not used");
        assert_eq!(bytes(&h, root), good, "byte {k} changed: stored again");
    }
    // A stored shape that passes the checks, here with another hot state layout, is what an open takes; so is
    // one whose hash changed, which an open does not check.
    let other = with_word(&good, 15, &|w| w + 8);
    set_bytes(&mut h, root, &other);
    let taken = reopen(&mut h, root);
    let mut want = s;
    want.hot_h += 8;
    assert_eq!(taken, want);
    let hash = with_word(&good, 24, &|w| w ^ 1);
    set_bytes(&mut h, root, &hash);
    assert_eq!(reopen(&mut h, root), s, "another hash: taken");
    assert_eq!(bytes(&h, root), hash, "another hash: not stored again");
    set_bytes(&mut h, root, &good);
    let tb = Table::open(&mut h, root).unwrap();
    assert_eq!(*tb.shape(), s);
    check(&tb, &m);
}

// ---------------------------------------------------------------- cases the model's draws do not reach

/// A table built with 63 indexes: the descriptors of indexes 31 to 63 lie in index directory blocks 1 and 2,
/// made when the indexes are declared; every index holds the row.
#[test]
fn sixty_three_indexes() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::I64)];
    let d: Vec<IndexDecl> = (1..=63).map(|i| decl(i, 8, &[(id(1), false)])).collect();
    let (img, _) = schema_indexed(&cols, &d);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1)), (id(1), V::I(5))])).unwrap();
    for i in 1..=63 {
        assert_eq!(tb.count(&mut key_scratch(), i, &mut Src::new(&[(id(1), V::I(5))])).unwrap(), 1, "index {i}");
    }
}

/// A row whose indexed Int is −2^35000 (5,001 LEB128 bytes, the low 5,000 groups zero): its key, longer than a
/// point operation's stack, is read from the stored row a piece at a time when the row is removed.
#[test]
fn huge_negative_int_key_removed() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::INT)];
    let (img, _) = schema_indexed(&cols, &[decl(1, 8, &[(id(1), false)])]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    let mut leb = vec![0x80u8; 5_000];
    leb.push(0x7F);
    let key = |leb: &Vec<u8>| vec![(id(1), V::Int(leb.clone()))];
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1)), (id(1), V::Int(leb.clone()))])).unwrap();
    assert_eq!(tb.count(&mut key_scratch(), 1, &mut Src::new(&key(&leb))).unwrap(), 1);
    assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1))])).unwrap().0.is_some());
    assert_eq!(tb.count(&mut key_scratch(), 1, &mut Src::new(&key(&leb))).unwrap(), 0);
}

/// A Text key of 4,100 bytes in an index of W = 32: longer than a point operation's stack, its key-log record
/// is shorter than a block.
#[test]
fn long_text_key_record_shorter_than_a_block() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::TEXT)];
    let (img, _) = schema_indexed(&cols, &[decl(1, 32, &[(id(1), false)])]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    let text = V::Bytes(vec![b'a'; 4_100]);
    tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1)), (id(1), text.clone())])).unwrap();
    assert_eq!(tb.count(&mut key_scratch(), 1, &mut Src::new(&[(id(1), text.clone())])).unwrap(), 1);
    assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(1))])).unwrap().0.is_some());
    assert_eq!(tb.count(&mut key_scratch(), 1, &mut Src::new(&[(id(1), text)])).unwrap(), 0);
}

/// A forward walk resumed after a row whose key has lost every row since goes on at the next key, all its rows.
#[test]
fn walk_resumed_after_its_key_is_gone() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::I64)];
    let (img, _) = schema_indexed(&cols, &[decl(1, 8, &[(id(1), false)])]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 1, &img).unwrap();
    for (k, v) in [(1u64, 10i64), (2, 10), (3, 20), (4, 20)] {
        tb.upsert(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k)), (id(1), V::I(v))])).unwrap();
    }
    let buf = &mut [0u8; 64];
    let mut first = None;
    tb.walk(1, Dir::Fwd, Start::End, 1, buf, |k, rid| {
        first = Some((k.to_vec(), rid));
        Ok(())
    })
    .unwrap();
    let (k, rid) = first.unwrap();
    for key in [1u64, 2] {
        tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(key))])).unwrap();
    }
    let rid_of = |k: u64| nt_testhost::rows::rid_of(&tb, &[(id(0), V::U(k))]).unwrap();
    let mut rest = Vec::new();
    tb.walk(1, Dir::Fwd, Start::After(&k, rid), 10, buf, |_, rid| {
        rest.push(rid);
        Ok(())
    })
    .unwrap();
    assert_eq!(rest, vec![rid_of(3), rid_of(4)]);
}
