//! A table's secondary indexes against a model of its rows: a table declaring five indexes (a three-case tag;
//! an optional Bool with an i64; a text, short or longer than the tree's key area, out of line now and then; an
//! Int, small or wider than 64 bits; the unit, whose key is empty) takes random inserts, updates that change an
//! indexed cell or only another one, removes, and maintenance that seals rows. Every few writes, the
//! count of every key the model holds, through the table, equals the model's, a key no row holds counts 0, and
//! each index's group walk finds as many keys and rows as the model. Each index's sorted walk, forward and
//! backward, whole, in pieces resumed after their last row, and from a row's key, hands out the model's rows
//! under their keys in key order (the rows of one key in rid order); its smallest and largest keys whose first
//! part is not null are the model's.
//! Removing every row empties every tree. Each run is made under both fill patterns and must give the same
//! answers and the same host calls.
//!
//! Also: a schema's index declarations are checked when a table is made with it, and a count, a walk and a top
//! name an index the table declares. Rows whose primary key and indexed text are longer than their trees' key
//! areas hand their key-log records back as they are updated and removed. Keys longer than `KEY_STACK` bytes,
//! which no stack buffer holds, are taken, counted, updated and removed like any other.
//! Updates of rows in the open group and in sealed groups (their patches included), through `upsert` and
//! through `set` of one leaf, keep every index and every integer total equal to the model's, among seals,
//! compactions, the column release and schema changes that add and drop columns. Schema changes remove indexes
//! (those holding a dropped column, those no longer declared) and add them in any combination: an added index
//! is built over the rows in maintenance steps, a group's rows a step, `Busy` until then, and kept by the
//! writes made meanwhile.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use native_table::consts::{COMPACT_DEAD, GROUP_SLOTS};
use native_table::fault::{Fault, SchemaError};
use native_table::index::area_nodes::KeyLog;
use native_table::index::secondary::{Secondary, Start};
use native_table::key::{KeyKind, KeyPart, KeySpec};
use native_table::mem::rd_u64;
use native_table::schema::IndexDecl;
use native_table::table::jobs::Ran;
use native_table::table::layout::{Shape, STORED_BYTES};
use native_table::table::primary::Primary;
use native_table::table::{buffer, indexes, read, root, Table, Upsert, KEY_STACK};
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
    let (mut out, mut scratch) = (vec![0u8; 1 << 14], vec![0u8; 1 << 14]);
    let mut after: Option<Vec<u8>> = None;
    let mut counts = Vec::new();
    while let Some((len, c)) = ix.group_next(tb.host(), &spec, after.as_deref(), &mut out, &mut scratch).unwrap() {
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

/// `check` of the indexes `live` names; every other one is gone (`BadRef`).
fn check_live(tb: &Table<'_, VecHost>, m: &Model, live: &[u8]) -> u64 {
    check_built(tb, m, live, &[])
}

/// `check_live`, the indexes `busy` names being built: their counts, walks and ends are `Busy`.
fn check_built(tb: &Table<'_, VecHost>, m: &Model, live: &[u8], busy: &[u8]) -> u64 {
    let mut keys = 0u64;
    for i in 0..ALL.len() {
        let ix = ALL[i].0;
        if busy.contains(&ix) {
            let cells = m.values().next().map(|(_, row)| key_cells(i, row)).unwrap_or_default();
            assert_eq!(
                tb.count(&mut key_scratch(), ix, &mut Src::new(&cells)),
                Err(Fault::Busy),
                "index {ix} is being built"
            );
            let mut buf = vec![0u8; 1 << 14];
            let r = tb.walk(ix, Dir::Fwd, Start::End, 10, &mut buf, |_, _| Ok(()));
            assert_eq!(r, Err(Fault::Busy), "index {ix}: a walk while it is being built");
            assert_eq!(tb.max_key(ix, &mut buf), Err(Fault::Busy), "index {ix}: an end while it is being built");
            continue;
        }
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
    /// Refused (`Busy`), then removed and written again.
    Refused,
}

/// Writes a row through `upsert`, as the model expects; a refused update (`Busy`) is removed and written again.
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
        Err(Fault::Busy) => {
            assert!(sealed, "only a sealed row's update is refused");
            let rid = m[&k].0;
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, Some(rid));
            let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
            let Upsert::Inserted(rid2) = u else { panic!("re-insert") };
            m.insert(k, (rid2, row));
            Wrote::Refused
        }
        Err(e) => panic!("upsert: {e:?}"),
    }
}

#[test]
fn indexes_model() {
    let cols = columns();
    let (img, _) = schema_indexed(&cols, &decls());
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 9, &img).unwrap();
        let mut r = XorShift(0x5eed_1dc5);
        let texts = text_pool(&mut r);
        let mut m = Model::new();
        let (mut refused, mut sealed_updates, mut keys) = (0u64, 0u64, 0u64);
        for step in 0..14_000u64 {
            let op = r.below(100);
            let some_key = |r: &mut XorShift, m: &Model| *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            if op < 50 || m.is_empty() {
                let k = r.below(1 << 40);
                let row = row(&mut r, &texts, k);
                let w = upsert(&mut tb, &mut m, k, row);
                refused += u64::from(w == Wrote::Refused);
                sealed_updates += u64::from(w == Wrote::UpdatedSealed);
            } else if op < 68 {
                // An update: every cell new, or only the cell no index holds.
                let k = some_key(&mut r, &m);
                let new = if r.below(3) == 0 {
                    let mut row = m[&k].1.clone();
                    row[7].1 = V::I(r.next_u64() as i64);
                    row
                } else {
                    row(&mut r, &texts, k)
                };
                let w = upsert(&mut tb, &mut m, k, new);
                refused += u64::from(w == Wrote::Refused);
                sealed_updates += u64::from(w == Wrote::UpdatedSealed);
            } else if op < 88 {
                let k = if r.below(6) == 0 { r.below(1 << 40) } else { some_key(&mut r, &m) };
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, m.remove(&k).map(|x| x.0));
            } else if op < 90 {
                if r.below(2) == 0 {
                    tb.write_step(&everything()).unwrap();
                } else {
                    maintain_all(&mut tb);
                }
            } else {
                // One count, of a row's key under one index.
                let i = r.below(INDEXES.len() as u64) as usize;
                let k = some_key(&mut r, &m);
                let cells = key_cells(i, &m[&k].1);
                let want = m.values().filter(|(_, row)| key_cells(i, row) == cells).count() as i128;
                assert_eq!(
                    tb.count(&mut key_scratch(), INDEXES[i].0, &mut Src::new(&cells)).unwrap(),
                    want,
                    "step {step}"
                );
            }
            assert_eq!(tb.len().unwrap(), m.len() as u64, "step {step}");
            if step % 1_000 == 999 {
                keys += check(&tb, &m);
                lh.take(tb.host_mut());
            }
        }
        maintain_all(&mut tb);
        keys += check(&tb, &m);
        let seals = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
        assert!(seals >= 1, "rows were sealed: {seals}");
        assert!(sealed_updates >= 1, "sealed rows were updated: {sealed_updates}");
        // Removing every row, in an order of its own, leaves every tree empty.
        let mut left: Vec<u64> = m.keys().copied().collect();
        for j in (1..left.len()).rev() {
            left.swap(j, r.below(j as u64 + 1) as usize);
        }
        for k in left {
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        }
        for &(ix, _, _) in INDEXES {
            let s = Secondary::read(tb.host(), tb.root(), ix).unwrap();
            assert_eq!((s.root, s.nodes), (0, 0), "index {ix} is empty");
        }
        assert_eq!(check(&tb, &m), 0);
        (seals, sealed_updates, refused, keys)
    });
    eprintln!("indexes_model: {digest:?}");
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

/// A text whose key under an index passes `KEY_STACK` bytes is taken like any other (written into the index's
/// key log as a record first, and probed through it), counted, updated to and from a short text, and removed.
#[test]
fn an_indexed_key_past_the_key_buffer_is_taken() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::TEXT)];
    let (img, _) = schema_indexed(&cols, &[decl(1, 16, &[(id(1), false)])]);
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 3, &img).unwrap();
    let row = |k: u64, len: usize| vec![(id(0), V::U(k)), (id(1), V::Bytes(vec![b'x'; len]))];
    let count = |tb: &Table<'_, VecHost>, len: usize| {
        tb.count(&mut key_scratch(), 1, &mut Src::new(&[(id(1), V::Bytes(vec![b'x'; len]))])).unwrap()
    };
    // A text of n bytes with no zero byte encodes to n + 2.
    let fits = KEY_STACK - 2;
    let lens = [fits, fits + 1, 5_000, 20_000];
    for (k, &len) in lens.iter().enumerate() {
        let got = tb.upsert(&mut key_scratch(), &mut Src::new(&row(k as u64, len))).map(|x| x.0);
        assert!(matches!(got, Ok(Upsert::Inserted(_))), "a text of {len} bytes: {got:?}");
    }
    for len in lens {
        assert_eq!(count(&tb, len), 1, "a text of {len} bytes");
    }
    assert_eq!((count(&tb, 6_000), count(&tb, 4_999)), (0, 0));
    assert!(matches!(tb.upsert(&mut key_scratch(), &mut Src::new(&row(0, 5_000))), Ok((Upsert::Replaced(_), _))));
    assert_eq!((count(&tb, fits), count(&tb, 5_000)), (0, 2));
    assert!(matches!(tb.upsert(&mut key_scratch(), &mut Src::new(&row(2, 20_000))), Ok((Upsert::Replaced(_), _))));
    assert_eq!((count(&tb, 5_000), count(&tb, 20_000)), (1, 2));
    assert!(matches!(tb.upsert(&mut key_scratch(), &mut Src::new(&row(2, 7))), Ok((Upsert::Replaced(_), _))));
    assert_eq!((count(&tb, 20_000), count(&tb, 7)), (1, 1));
    for k in 0..4 {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
    }
    assert_eq!(Secondary::read(tb.host(), tb.root(), 1).unwrap().log, KeyLog::default(), "every record back");
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
                assert_ne!(upsert(&mut tb, &mut fill, 1_000 + j, row), Wrote::Refused, "a new row");
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

/// A canonical (signed) LEB128 form of `n` bytes, its 7-bit groups drawn from `r`; a negative one when `neg`.
fn leb_of(r: &mut XorShift, n: usize, signed: bool, neg: bool) -> Vec<u8> {
    let mut out: Vec<u8> = (0..n).map(|_| 0x80 | r.below(128) as u8).collect();
    out[n - 1] = match (signed, neg) {
        (false, _) => 1 + r.below(127) as u8,
        (true, false) => 1 + r.below(0x3F) as u8,
        (true, true) => 0x40 + r.below(0x3F) as u8,
    };
    out
}

/// Nats and Ints of up to 20,000 LEB bytes (keys past `KEY_STACK`, whose stored forms pass the buffer a stored
/// value is copied into) as index keys: a Nat alone, and an Int then an i64. Inserts, updates, removes and
/// seals against a model, with every key's count, the sorted walks and the ends through each index; removing
/// every row empties every tree and key log.
#[test]
fn huge_numbers_as_index_keys() {
    let cols = vec![t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::INT), t::scalar(ty::I64)];
    let decls = [decl(1, 16, &[(id(1), false)]), decl(2, 8, &[(id(2), false), (id(3), false)])];
    let parts: [&[u32]; 2] = [&[1], &[2, 3]];
    let (img, _) = schema_indexed(&cols, &decls);
    let mut h = VecHost::new(FILLS[1]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 3, &img).unwrap();
    let mut r = XorShift(0x4E_6A7);
    let lens = [1usize, 3, 12, 4_600, 4_700, 5_000, 6_000, 20_000];
    let mut nats: Vec<Vec<u8>> = lens.iter().map(|&n| leb_of(&mut r, n, false, false)).collect();
    let mut ints: Vec<Vec<u8>> =
        lens.iter().flat_map(|&n| [false, true].map(|neg| leb_of(&mut r, n, true, neg))).collect();
    // Numbers of one length differing in their top group only.
    let mut top = nats[6].clone();
    top[5_999] ^= 0x20;
    nats.push(top);
    let mut top = ints[13].clone();
    top[5_999] ^= 0x10;
    ints.push(top);
    let cells = |i: usize, row: &[(u32, V)]| -> Vec<(u32, V)> {
        parts[i].iter().map(|&c| row.iter().find(|(l, _)| *l == id(c)).unwrap().clone()).collect()
    };
    let check =
        |tb: &Table<'_, VecHost>, m: &Model, fill: &Model| {
            for (i, ix) in [1u8, 2].into_iter().enumerate() {
                let mut keys: Vec<Vec<(u32, V)>> = m.values().map(|(_, row)| cells(i, row)).collect();
                keys.extend(nats.iter().zip(&ints).map(|(n, x)| {
                    cells(i, &[(id(1), V::Nat(n.clone())), (id(2), V::Int(x.clone())), (id(3), V::I(9))])
                }));
                for k in keys {
                    let want = m.values().filter(|(_, row)| cells(i, row) == k).count() as i128;
                    assert_eq!(tb.count(&mut key_scratch(), ix, &mut Src::new(&k)).unwrap(), want, "index {ix}");
                }
                let mut buf = vec![0u8; 64_000];
                let mut rows: Vec<Row> = m
                    .values()
                    .chain(fill.values())
                    .map(|(rid, row)| {
                        let n = tb.index_key(ix, &mut Src::new(&cells(i, row)), &mut buf).unwrap();
                        (buf[..n].to_vec(), *rid)
                    })
                    .collect();
                rows.sort();
                let (min, out) = grown(|out| tb.min_key(ix, out));
                assert_eq!(min.map(|n| out[..n].to_vec()), rows.first().map(|x| x.0.clone()), "index {ix}: min");
                let (max, out) = grown(|out| tb.max_key(ix, out));
                assert_eq!(max.map(|n| out[..n].to_vec()), rows.last().map(|x| x.0.clone()), "index {ix}: max");
                assert!(walk_grown(tb, ix, Dir::Fwd, Start::End, u64::MAX).0 == rows, "index {ix}: the walk forward");
                rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                assert!(walk_grown(tb, ix, Dir::Rev, Start::End, u64::MAX).0 == rows, "index {ix}: the walk backward");
            }
        };
    let (mut m, mut fill) = (Model::new(), Model::new());
    let mut sealed = 0u64;
    for step in 0..300u64 {
        let op = r.below(10);
        if op < 5 || m.is_empty() {
            let k = r.below(24);
            let n = nats[r.below(nats.len() as u64) as usize].clone();
            let x = ints[r.below(ints.len() as u64) as usize].clone();
            let row = vec![(id(0), V::U(k)), (id(1), V::Nat(n)), (id(2), V::Int(x)), (id(3), V::I(r.below(2) as i64))];
            upsert(&mut tb, &mut m, k, row);
        } else if op < 8 {
            let k = r.below(24);
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        } else {
            maintain_all(&mut tb);
            let on = |rid| matches!(read::find(tb.host(), tb.root(), rid).unwrap(), Some(read::Row::Sealed { .. }));
            sealed = sealed.max(m.values().filter(|(rid, _)| on(*rid)).count() as u64);
        }
        if step % 60 == 59 {
            check(&tb, &m, &fill);
        }
        if step == 80 {
            // A group's worth of small rows outside the model, keys of their own, so the rows above are sealed
            // with their group.
            for j in 0..4_100u64 {
                let (n, x) = (V::U((1 << 40) + j), V::I(-(1 << 40)));
                let row = vec![(id(0), V::U(1_000 + j)), (id(1), n), (id(2), x), (id(3), V::I(0))];
                assert_ne!(upsert(&mut tb, &mut fill, 1_000 + j, row), Wrote::Refused, "a new row");
            }
            maintain_all(&mut tb);
        }
    }
    check(&tb, &m, &fill);
    assert!(sealed >= 1, "rows were sealed: {sealed}");
    for k in (0..24).chain(1_000..5_100) {
        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        assert_eq!(gone, m.remove(&k).or_else(|| fill.remove(&k)).map(|x| x.0));
    }
    for ix in 1..=2 {
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
        if !l.live() || l.key {
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

/// Updates of rows in the open group and in sealed groups, through `upsert` (every cell new, one cell new, or
/// only the cell no index holds) and through `set` of one leaf, among random inserts, removes and maintenance
/// steps: every index's counts and walks and every integer total equal the model's. Keys longer than
/// `KEY_STACK` move too.
#[test]
fn updates_of_buffered_and_sealed_rows() {
    let cols = columns();
    let (img, _) = schema_indexed(&cols, &decls());
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 9, &img).unwrap();
        let mut r = XorShift(0x5eed_b0f5);
        let mut texts = text_pool(&mut r);
        texts.extend((0..3u8).map(|i| vec![b'x' + i; KEY_STACK + 100 + 700 * usize::from(i)]));
        let mut m = Model::new();
        // Updates by way (upsert, set) and by where the row lay (see `place`).
        let mut by = [[0u64; 2]; 2];
        let mut units = 0u64;
        for step in 0..12_000u64 {
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
                let new = match r.below(3) {
                    0 => fresh,
                    1 => {
                        let c = 1 + r.below(7) as usize;
                        let mut x = old;
                        x[c] = fresh[c].clone();
                        x
                    }
                    _ => {
                        let mut x = old;
                        x[7].1 = V::I(r.next_u64() as i64);
                        x
                    }
                };
                by[0][place(&tb, rid)] += 1;
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                assert_eq!(u, Upsert::Replaced(rid), "an update keeps the rid");
                m.insert(k, (rid, new));
            } else if op < 85 {
                let k = some_key(&mut r, &m);
                let rid = m[&k].0;
                let c = [1usize, 2, 3, 4, 5, 7][r.below(6) as usize];
                let cell = [row(&mut r, &texts, k)[c].clone()];
                by[1][place(&tb, rid)] += 1;
                set_cell(&mut tb, &mut m, k, cell[0].0, cell[0].1.clone());
            } else if op < 96 {
                let k = if r.below(6) == 0 { r.below(1 << 40) } else { some_key(&mut r, &m) };
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, m.remove(&k).map(|x| x.0));
            } else {
                for _ in 0..1 + r.below(40) {
                    if tb.step().unwrap() == Ran::Nothing {
                        break;
                    }
                    units += 1;
                }
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
        (by, units)
    });
    eprintln!("updates_of_buffered_and_sealed_rows: {digest:?}");
}

// ---------------------------------------------------------------- updates in every row state

/// The schema image of version 1 (`columns`), 2 (a Nat column of default 7 and an ?Bool column added) or 3
/// (column 7 dropped too), with the indexes.
fn version(v: u32) -> Vec<u8> {
    let cols = columns();
    let mut vc: Vec<VCol> = cols.into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    if v >= 2 {
        vc.push(VCol { ty: t::scalar(ty::NAT), added: 2, dropped: 0, default: Some(vec![7]) });
        vc.push(VCol { ty: t::opt(t::scalar(ty::BOOL)), added: 2, dropped: 0, default: None });
    }
    if v >= 3 {
        vc[7].dropped = 3;
    }
    schema_versioned(v, &vc, &decls()).0
}

/// A row with key k at schema version v: version 1's cells (`row`), from version 2 a Nat and an ?Bool cell
/// more, from version 3 no cell of column 7.
fn row_at(v: u32, r: &mut XorShift, texts: &[Vec<u8>], k: u64) -> Vec<(u32, V)> {
    let mut x = row(r, texts, k);
    if v >= 2 {
        x.push((id(8), V::U(r.below(1 << 20))));
        x.push((id(9), if r.below(3) == 0 { V::Null } else { V::Bool(r.below(2) == 0) }));
    }
    if v >= 3 {
        x.retain(|(l, _)| *l != id(7));
    }
    x
}

/// The leaf ids no index holds at schema version v.
fn unindexed(v: u32) -> &'static [u32] {
    match v {
        1 => &[7 << 16],
        2 => &[7 << 16, 8 << 16, 9 << 16],
        _ => &[8 << 16, 9 << 16],
    }
}

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
/// through `set` of one leaf, among random inserts, removes, seals, maintenance steps (compactions, the column
/// release) and two schema changes
/// (a Nat column of default 7 and an ?Bool column added, then an indexed-free i64 column dropped): every index's
/// counts and walks, every integer total and every Bool, tag and buffered count equal the model's; the dropped
/// column has no total.
#[test]
fn updates_in_every_state() {
    let digest = per_fill(|h, lh| {
        let img = [version(1), version(2), version(3)];
        let (mut tb, _) = Table::create(h, 9, &img[0]).unwrap();
        let mut r = XorShift(0x5eed_57a7);
        let mut texts = text_pool(&mut r);
        texts.extend((0..3u8).map(|i| vec![b'x' + i; KEY_STACK + 100 + 700 * usize::from(i)]));
        let mut m = Model::new();
        let mut v = 1u32;
        let mut by = [[0u64; 2]; 2];
        let mut ran = BTreeMap::new();
        for step in 0..12_000u64 {
            if step == 4_000 || step == 8_000 {
                v += 1;
                tb.change_schema(&img[v as usize - 1]).unwrap();
                for (_, (_, row)) in m.iter_mut() {
                    if v == 2 {
                        row.extend([(id(8), V::U(7)), (id(9), V::Null)]);
                    } else {
                        row.retain(|(l, _)| *l != id(7));
                    }
                }
                check_running(&tb, &m);
                if v == 3 {
                    assert!(tb.sum(id(7), &mut [0u8; 64]).is_err(), "a dropped column has no total");
                }
            }
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
                let new = row_at(v, &mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                match u {
                    Upsert::Inserted(rid) => assert!(m.insert(k, (rid, new)).is_none()),
                    Upsert::Replaced(rid) => assert_eq!(m.insert(k, (rid, new)).unwrap().0, rid),
                }
            } else if op < 65 {
                let k = some_key(&mut r, &m);
                let (rid, old) = m[&k].clone();
                let fresh = row_at(v, &mut r, &texts, k);
                let pick = |ls: &[u32], r: &mut XorShift| ls[r.below(ls.len() as u64) as usize];
                let new = match r.below(3) {
                    0 => fresh,
                    w => {
                        let c = if w == 1 {
                            fresh[1 + r.below(fresh.len() as u64 - 1) as usize].0
                        } else {
                            pick(unindexed(v), &mut r)
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
                let fresh = row_at(v, &mut r, &texts, k);
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
        for kind in ["Compaction", "Release"] {
            assert!(ran.get(kind).copied().unwrap_or(0) >= 1, "{kind} steps ran: {ran:?}");
        }
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

/// Blocks of area 1, which holds every index tree and key log, in use: its pages' blocks less the never-used ones
/// of its newest page and its free stack.
fn area1_in_use(tb: &Table<'_, VecHost>) -> u64 {
    area_in_use(tb, 1)
}

/// Blocks of area `a` in use.
fn area_in_use(tb: &Table<'_, VecHost>, a: u32) -> u64 {
    let rec = native_table::area::AreaRecord::read(tb.host(), tb.root(), a).unwrap();
    let mut free = 0u64;
    let mut b = rec.free_head;
    while b != 0 {
        free += 1;
        b = rd_u64(tb.host().get(Region::Block(b)).unwrap(), 0).unwrap();
    }
    let never = if rec.newest != 0 { 16 - u64::from(rec.bump) } else { 0 };
    rec.pages * 16 - never - free
}

/// Blocks the primary index and the secondary indexes `live` hold: their trees' nodes and key-log blocks.
fn area1_held(tb: &Table<'_, VecHost>, live: &[u8]) -> u64 {
    let (h, tbl) = (tb.host(), tb.root());
    let p = Primary::read(h, tbl).unwrap().map_or(0, |p| p.nodes + p.log.blocks);
    p + live
        .iter()
        .map(|&ix| {
            let s = Secondary::read(h, tbl, ix).unwrap();
            s.nodes + s.log.blocks
        })
        .sum::<u64>()
}

/// Version v of a schema whose changes remove indexes: version 2 drops the tag column (index 1 holds it; its
/// declaration stays) and declares no index 3; version 3 drops column 7 too and declares no index 2.
fn removing(v: u32) -> Vec<u8> {
    let cols = columns();
    let mut vc: Vec<VCol> = cols.into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    let mut d = decls();
    if v >= 2 {
        vc[4].dropped = 2;
        d.retain(|x| x.id != 3);
    }
    if v >= 3 {
        vc[7].dropped = 3;
        d.retain(|x| x.id != 2);
    }
    schema_versioned(v, &vc, &d).0
}

/// The indexes version v of `removing` keeps.
fn kept(v: u32) -> &'static [u8] {
    match v {
        1 => &[1, 2, 3, 4, 5],
        2 => &[2, 4, 5],
        _ => &[4, 5],
    }
}

/// A row with key k at version v of `removing`.
fn row_removing(v: u32, r: &mut XorShift, texts: &[Vec<u8>], k: u64) -> Vec<(u32, V)> {
    let mut x = row(r, texts, k);
    x.retain(|(l, _)| (v < 2 || *l != id(4)) && (v < 3 || *l != id(7)));
    x
}

/// Two schema changes that remove indexes (by a dropped column and by declarations left out) among random
/// inserts, updates through `upsert` and `set`, removes and maintenance steps, each made while rows lie in every
/// state (the open group, sealed groups): the
/// indexes kept equal the model's at every check, the removed ones are gone, and once maintenance is done area
/// 1's blocks in use are exactly those the remaining trees and key logs hold; removing every row then leaves
/// none in use.
#[test]
fn removed_indexes_in_every_state() {
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 9, &removing(1)).unwrap();
        let mut r = XorShift(0x7e30_0e5d);
        let mut texts = text_pool(&mut r);
        texts.extend((0..3u8).map(|i| vec![b'x' + i; KEY_STACK + 100 + 700 * usize::from(i)]));
        let mut m = Model::new();
        let mut v = 1u32;
        let mut at_change = [0u64; 2];
        let mut ran = BTreeMap::new();
        for step in 0..9_000u64 {
            if step == 3_000 || step == 6_000 {
                // Inserts until a group seals, so that rows lie in a sealed group and in the open group.
                let g0 = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
                while root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap() == g0 {
                    let k = r.below(1 << 40);
                    let new = row_removing(v, &mut r, &texts, k);
                    let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                    let (Upsert::Inserted(rid) | Upsert::Replaced(rid)) = u;
                    m.insert(k, (rid, new));
                }
                for (rid, _) in m.values() {
                    at_change[place(&tb, *rid)] += 1;
                }
                let gone: Vec<u8> = kept(v).iter().copied().filter(|x| !kept(v + 1).contains(x)).collect();
                assert!(area1_held(&tb, &gone) > area1_held(&tb, &[]), "the removed indexes hold blocks");
                if v == 1 {
                    assert!(Secondary::read(tb.host(), tb.root(), 3).unwrap().log.blocks > 0, "index 3's key log");
                }
                v += 1;
                tb.change_schema(&removing(v)).unwrap();
                for (_, row) in m.values_mut() {
                    row.retain(|(l, _)| *l != id(if v == 2 { 4 } else { 7 }));
                }
                check_live(&tb, &m, kept(v));
            }
            let op = r.below(100);
            let some_key = |r: &mut XorShift, m: &Model| *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            if op < 40 || m.is_empty() {
                let k = r.below(1 << 40);
                let new = row_removing(v, &mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                match u {
                    Upsert::Inserted(rid) => assert!(m.insert(k, (rid, new)).is_none()),
                    Upsert::Replaced(rid) => assert_eq!(m.insert(k, (rid, new)).unwrap().0, rid),
                }
            } else if op < 65 {
                let k = some_key(&mut r, &m);
                let rid = m[&k].0;
                let new = row_removing(v, &mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                assert_eq!(u, Upsert::Replaced(rid), "an update keeps the rid");
                m.insert(k, (rid, new));
            } else if op < 85 {
                let k = some_key(&mut r, &m);
                let fresh = row_removing(v, &mut r, &texts, k);
                let cell = [fresh[1 + r.below(fresh.len() as u64 - 1) as usize].clone()];
                if cell[0].0 == id(6) {
                    continue;
                }
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
            if step % 1_000 == 999 {
                check_live(&tb, &m, kept(v));
                lh.take(tb.host_mut());
            }
        }
        for (at, &c) in at_change.iter().enumerate() {
            assert!(c >= 1, "rows at place {at} when the schema changed: {at_change:?}");
        }
        assert!(ran.get("Drain").copied().unwrap_or(0) >= 1, "drain units ran: {ran:?}");
        maintain_all(&mut tb);
        check_live(&tb, &m, kept(v));
        assert_eq!(area1_in_use(&tb), area1_held(&tb, kept(v)), "area 1 holds the remaining indexes alone");
        let mut left: Vec<u64> = m.keys().copied().collect();
        for j in (1..left.len()).rev() {
            left.swap(j, r.below(j as u64 + 1) as usize);
        }
        for k in left {
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        }
        maintain_all(&mut tb);
        assert_eq!((area1_in_use(&tb), area1_held(&tb, kept(v))), (0, 0), "every row removed");
        (at_change, ran)
    });
    eprintln!("removed_indexes_in_every_state: {digest:?}");
}

/// Version v of a schema whose changes add indexes: version 1 declares indexes 2 and 5; version 2 adds a Nat
/// column of default 7 (column 8) and an ?Bool column (9), and indexes 1 (the tag), 3 (the text, keys past
/// `KEY_STACK` among them) and 6 (the new Nat column); version 3 drops the tag column, which removes index 1,
/// and the unindexed i64 column, and adds index 4 (the Int).
fn adding(v: u32) -> Vec<u8> {
    let cols = columns();
    let mut vc: Vec<VCol> = cols.into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    if v >= 2 {
        vc.push(VCol { ty: t::scalar(ty::NAT), added: 2, dropped: 0, default: Some(vec![7]) });
        vc.push(VCol { ty: t::opt(t::scalar(ty::BOOL)), added: 2, dropped: 0, default: None });
    }
    if v >= 3 {
        vc[4].dropped = 3;
        vc[7].dropped = 3;
    }
    schema_versioned(v, &vc, &decls_of(declared(v))).0
}

/// The indexes version v of `adding` declares.
fn declared(v: u32) -> &'static [u8] {
    match v {
        1 => &[2, 5],
        2 => &[1, 2, 3, 5, 6],
        _ => &[2, 3, 4, 5, 6],
    }
}

/// A row with key k at version v of `adding`.
fn row_adding(v: u32, r: &mut XorShift, texts: &[Vec<u8>], k: u64) -> Vec<(u32, V)> {
    let mut x = row(r, texts, k);
    if v >= 2 {
        x.push((id(8), V::U(r.below(40))));
        x.push((id(9), if r.below(3) == 0 { V::Null } else { V::Bool(r.below(2) == 0) }));
    }
    if v >= 3 {
        x.retain(|(l, _)| *l != id(4) && *l != id(7));
    }
    x
}

/// The secondary indexes being built, by id.
fn being_built(tb: &Table<'_, VecHost>) -> Vec<u8> {
    let b = indexes::building(tb.host(), tb.root()).unwrap();
    (1..64u8).filter(|&x| b & (1 << x) != 0).collect()
}

/// `check_built` of the indexes version v of `adding` declares, those being built `Busy`; returns how many were.
fn check_adding(tb: &Table<'_, VecHost>, m: &Model, v: u32) -> u64 {
    let busy = being_built(tb);
    let live: Vec<u8> = declared(v).iter().copied().filter(|x| !busy.contains(x)).collect();
    check_built(tb, m, &live, &busy);
    busy.len() as u64
}

/// Two schema changes that add indexes (one on a column the change adds, one whose keys pass `KEY_STACK`), the
/// second also dropping a column, which removes an index, each made while rows lie in every state (the open
/// group, sealed groups), among random inserts, updates through `upsert` and `set`, removes and maintenance
/// steps: while an added index is built its
/// counts, walks and ends are `Busy`; every other index equals the model's at every check, and once built so
/// does the added one, with the writes made during its build; once maintenance is done area 1's blocks in use
/// are exactly those the trees and key logs hold, and removing every row then leaves none in use.
#[test]
fn added_indexes_in_every_state() {
    let digest = per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 9, &adding(1)).unwrap();
        let mut r = XorShift(0xadd1_de8e);
        let mut texts = text_pool(&mut r);
        texts.extend((0..3u8).map(|i| vec![b'x' + i; KEY_STACK + 100 + 700 * usize::from(i)]));
        let mut m = Model::new();
        let mut v = 1u32;
        let mut at_change = [0u64; 2];
        let (mut ran, mut busy) = (BTreeMap::new(), 0u64);
        for step in 0..9_000u64 {
            if step == 3_000 || step == 6_000 {
                // Inserts until a group seals, so that rows lie in a sealed group and in the open group.
                let g0 = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
                while root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap() == g0 {
                    let k = r.below(1 << 40);
                    let new = row_adding(v, &mut r, &texts, k);
                    let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                    let (Upsert::Inserted(rid) | Upsert::Replaced(rid)) = u;
                    m.insert(k, (rid, new));
                }
                for (rid, _) in m.values() {
                    at_change[place(&tb, *rid)] += 1;
                }
                v += 1;
                tb.change_schema(&adding(v)).unwrap();
                for (_, row) in m.values_mut() {
                    if v == 2 {
                        row.extend([(id(8), V::U(7)), (id(9), V::Null)]);
                    } else {
                        row.retain(|(l, _)| *l != id(4) && *l != id(7));
                    }
                }
                let added: Vec<u8> = declared(v).iter().copied().filter(|x| !declared(v - 1).contains(x)).collect();
                assert_eq!(being_built(&tb), added, "version {v}: the added indexes are being built");
                busy += check_adding(&tb, &m, v);
            }
            let op = r.below(100);
            let some_key = |r: &mut XorShift, m: &Model| *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            if op < 40 || m.is_empty() {
                let k = r.below(1 << 40);
                let new = row_adding(v, &mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                match u {
                    Upsert::Inserted(rid) => assert!(m.insert(k, (rid, new)).is_none()),
                    Upsert::Replaced(rid) => assert_eq!(m.insert(k, (rid, new)).unwrap().0, rid),
                }
            } else if op < 65 {
                let k = some_key(&mut r, &m);
                let rid = m[&k].0;
                let new = row_adding(v, &mut r, &texts, k);
                let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap();
                assert_eq!(u, Upsert::Replaced(rid), "an update keeps the rid");
                m.insert(k, (rid, new));
            } else if op < 85 {
                let k = some_key(&mut r, &m);
                let fresh = row_adding(v, &mut r, &texts, k);
                let cell = [fresh[1 + r.below(fresh.len() as u64 - 1) as usize].clone()];
                if cell[0].0 == id(6) {
                    continue;
                }
                set_cell(&mut tb, &mut m, k, cell[0].0, cell[0].1.clone());
            } else if op < 96 {
                let k = if r.below(6) == 0 { r.below(1 << 40) } else { some_key(&mut r, &m) };
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, m.remove(&k).map(|x| x.0));
            } else {
                let n = 1 + r.below(40);
                units(&mut tb, n, &mut ran);
            }
            if step % 1_000 == 700 {
                units(&mut tb, u64::MAX, &mut ran);
            }
            assert_eq!(tb.len().unwrap(), m.len() as u64, "step {step}");
            if step % 1_000 == 999 || step % 1_000 == 300 {
                busy += check_adding(&tb, &m, v);
                lh.take(tb.host_mut());
            }
        }
        for (at, &c) in at_change.iter().enumerate() {
            assert!(c >= 1, "rows at place {at} when the schema changed: {at_change:?}");
        }
        assert!(busy >= 4, "indexes were checked while being built: {busy}");
        assert!(ran.get("Build").copied().unwrap_or(0) >= 4, "build steps ran: {ran:?}");
        maintain_all(&mut tb);
        assert_eq!(check_adding(&tb, &m, v), 0);
        assert_eq!(area1_in_use(&tb), area1_held(&tb, declared(v)), "area 1 holds the indexes alone");
        let mut left: Vec<u64> = m.keys().copied().collect();
        for j in (1..left.len()).rev() {
            left.swap(j, r.below(j as u64 + 1) as usize);
        }
        for k in left {
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone, m.remove(&k).map(|x| x.0));
        }
        maintain_all(&mut tb);
        assert_eq!((area1_in_use(&tb), area1_held(&tb, declared(v))), (0, 0), "every row removed");
        (at_change, ran, busy)
    });
    eprintln!("added_indexes_in_every_state: {digest:?}");
}

/// A table of `n` rows at version 1 of `columns`, and its model.
fn filled(h: &mut VecHost, n: u64) -> (Table<'_, VecHost>, Model) {
    let (mut tb, _) = Table::create(h, 9, &version(1)).unwrap();
    let mut r = XorShift(3);
    let mut texts = text_pool(&mut r);
    texts.push(vec![b'q'; KEY_STACK + 300]);
    let mut m = Model::new();
    for k in 0..n {
        let x = row_at(1, &mut r, &texts, k);
        let (Upsert::Inserted(rid), _) = tb.upsert(&mut key_scratch(), &mut Src::new(&x)).unwrap() else { panic!() };
        m.insert(k, (rid, x));
    }
    (tb, m)
}

/// Dropping an indexed column removes exactly the indexes holding it, whether the new image still declares
/// them or not; an image differing only in its declarations removes the ones it leaves out. Maintenance then
/// frees every block of the removed indexes.
#[test]
fn schema_changes_remove_indexes() {
    let cols = columns();
    let base: Vec<VCol> = cols.into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    let all: Vec<u8> = INDEXES.iter().map(|x| x.0).collect();
    let holding =
        |c: u32| -> Vec<u8> { INDEXES.iter().filter(|x| x.2.iter().any(|p| p.0 == c)).map(|x| x.0).collect() };
    for c in [1u32, 2, 3, 4, 5, 6] {
        for declared in [true, false] {
            let mut h = VecHost::new(FILLS[0]);
            let (mut tb, mut m) = filled(&mut h, 2_000);
            let gone = holding(c);
            let live: Vec<u8> = all.iter().copied().filter(|x| !gone.contains(x)).collect();
            let mut vc = base.clone();
            vc[c as usize].dropped = 2;
            let mut d = decls();
            if !declared {
                d.retain(|x| live.contains(&x.id));
            }
            tb.change_schema(&schema_versioned(2, &vc, &d).0).unwrap();
            for (_, row) in m.values_mut() {
                row.retain(|(l, _)| *l != id(c));
            }
            check_live(&tb, &m, &live);
            maintain_all(&mut tb);
            check_live(&tb, &m, &live);
            assert_eq!(area1_in_use(&tb), area1_held(&tb, &live), "column {c} dropped, declared {declared}");
        }
    }
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, m) = filled(&mut h, 2_000);
    check(&tb, &m);
    // The same columns, index 3 (long text keys in its key log) and index 5 left out: the schema hash is the
    // table's, and the change removes them.
    let fewer: Vec<_> = decls().into_iter().filter(|x| x.id != 3 && x.id != 5).collect();
    assert!(Secondary::read(tb.host(), tb.root(), 3).unwrap().log.blocks > 0, "index 3's key log");
    tb.change_schema(&schema_versioned(2, &base, &fewer).0).unwrap();
    assert_eq!(tb.shape().version, 2);
    check_live(&tb, &m, &[1, 2, 4]);
    maintain_all(&mut tb);
    check_live(&tb, &m, &[1, 2, 4]);
    assert_eq!(area1_in_use(&tb), area1_held(&tb, &[1, 2, 4]), "indexes 3 and 5 freed");
}

/// Each key's count under index `ix`, whose one part is column `col`, against the model.
fn check_counts(tb: &Table<'_, VecHost>, m: &Model, ix: u8, col: u32) {
    let mut want: BTreeMap<String, (Vec<(u32, V)>, i128)> = BTreeMap::new();
    for (_, row) in m.values() {
        let cells = vec![row.iter().find(|(l, _)| *l == id(col)).unwrap().clone()];
        want.entry(format!("{cells:?}")).or_insert((cells, 0)).1 += 1;
    }
    for (k, (cells, c)) in &want {
        assert_eq!(tb.count(&mut key_scratch(), ix, &mut Src::new(cells)).unwrap(), *c, "index {ix}, key {k}");
    }
}

/// Runs one step. A build step takes one group's rids: its index's position moves on by `GROUP_SLOTS`, or the
/// build ends at its end.
fn one_step(tb: &mut Table<'_, VecHost>) -> Ran {
    let at = being_built(tb).first().map(|&ix| (ix, Secondary::build_state(tb.host(), tb.root(), ix).unwrap()));
    let ran = tb.step().unwrap();
    if let (Ran::Build, Some((ix, (next, end)))) = (ran, at) {
        let want = if next + GROUP_SLOTS < end { (next + GROUP_SLOTS, end) } else { (0, 0) };
        assert_eq!(Secondary::build_state(tb.host(), tb.root(), ix).unwrap(), want, "index {ix}'s build step");
    }
    ran
}

/// Rows of `filled` that fill groups: two sealed at least, and the open one.
const MANY: u64 = 2 * GROUP_SLOTS + 500;

/// The steps a build over the rids below `end` takes: one for each group.
fn steps_to(end: u64) -> u64 {
    end.div_ceil(GROUP_SLOTS)
}

/// A schema change adds indexes in any combination with the others. On a table without rows an added index
/// holds every row at once. On one of several groups, an index declared again under its id with another W (a
/// removal and an addition) and a new index on an unindexed column are `Busy` until steps, a group's rows
/// each, have built them, and then count each key as the model does. A change that removes an index being
/// built ends its build.
#[test]
fn schema_changes_add_indexes() {
    let cols = columns();
    let base: Vec<VCol> = cols.into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    let mut r = XorShift(9);
    let texts = text_pool(&mut r);
    {
        let mut h = VecHost::new(FILLS[0]);
        let (mut tb, _) = Table::create(&mut h, 9, &schema_versioned(1, &base, &decls_of(&[2])).0).unwrap();
        tb.change_schema(&schema_versioned(2, &base, &decls_of(&[1, 2])).0).unwrap();
        assert_eq!(being_built(&tb), Vec::<u8>::new(), "no rows: nothing to build");
        let mut m = Model::new();
        for k in 0..300 {
            let x = row_at(1, &mut r, &texts, k);
            let (Upsert::Inserted(rid), _) = tb.upsert(&mut key_scratch(), &mut Src::new(&x)).unwrap() else {
                panic!()
            };
            m.insert(k, (rid, x));
        }
        check_live(&tb, &m, &[1, 2]);
    }
    let mut d = decls();
    d[0].w = 16;
    d.push(decl(7, 8, &[(id(7), false)]));
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, m) = filled(&mut h, MANY);
    tb.change_schema(&schema_versioned(2, &base, &d).0).unwrap();
    assert_eq!(being_built(&tb), vec![1, 7]);
    check_built(&tb, &m, &[2, 3, 4, 5], &[1]);
    assert_eq!(tb.count(&mut key_scratch(), 7, &mut Src::new(&[(id(7), V::I(0))])), Err(Fault::Busy));
    let (_, end) = Secondary::build_state(tb.host(), tb.root(), 1).unwrap();
    assert!(steps_to(end) >= 3, "rows in {} groups", steps_to(end));
    let mut builds = 0u64;
    while !being_built(&tb).is_empty() {
        builds += u64::from(one_step(&mut tb) == Ran::Build);
    }
    assert_eq!(builds, 2 * steps_to(end), "a step for each group, for each index");
    check_counts(&tb, &m, 1, 4);
    check_counts(&tb, &m, 7, 7);
    assert_eq!(Secondary::read(tb.host(), tb.root(), 1).unwrap().shape.w, 16);
    maintain_all(&mut tb);
    assert_eq!(area1_in_use(&tb), area1_held(&tb, &[1, 2, 3, 4, 5, 7]), "the old index 1 is freed");
    // A change removing an index being built.
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, m) = filled(&mut h, MANY);
    tb.change_schema(&schema_versioned(2, &base, &d).0).unwrap();
    assert_eq!(one_step(&mut tb), Ran::Build);
    tb.change_schema(&schema_versioned(3, &base, &d[..5]).0).unwrap();
    assert_eq!(being_built(&tb), vec![1]);
    assert_eq!(tb.count(&mut key_scratch(), 7, &mut Src::new(&[(id(7), V::I(0))])), Err(Fault::BadRef));
    maintain_all(&mut tb);
    check_counts(&tb, &m, 1, 4);
    assert_eq!(area1_in_use(&tb), area1_held(&tb, &[1, 2, 3, 4, 5]), "index 7's blocks are freed");
}

/// An image of the table's own version that adds an index is accepted: the version stays, and the index is
/// built over the rows in steps and then counts each key as the model does.
#[test]
fn an_index_added_at_the_same_version() {
    let base: Vec<VCol> = columns().into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, m) = filled(&mut h, 2_000);
    let mut d = decls();
    d.push(decl(7, 8, &[(id(7), false)]));
    tb.change_schema(&schema_versioned(1, &base, &d).0).unwrap();
    assert_eq!(tb.shape().version, 1);
    assert_eq!(being_built(&tb), vec![7]);
    while !being_built(&tb).is_empty() {
        one_step(&mut tb);
    }
    check_counts(&tb, &m, 7, 7);
    check(&tb, &m);
}

/// The table root holds the shape its schema image gives, from the table's making on and after every schema
/// change (an index added and one removed at the same version, a column added, a column dropped); a reopened
/// table has it, reads its rows and keeps its indexes. An open never uses a stored shape of an earlier schema
/// (also of one whose change kept the version), of another table format or of another schema page, schema hash
/// or version, nor one with any byte of those four words changed: it derives the shape from the schema image
/// again and stores it. A stored shape that passes every check is the one an open uses, as it is: its hash is
/// not checked.
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
    let reopen = |h: &mut VecHost, root: u64, img: &[u8]| {
        let tb = Table::open(h, root, img).unwrap();
        assert!(!tb.stale());
        *tb.shape()
    };
    let base: Vec<VCol> = columns().into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    let mut with7 = decls();
    with7.push(decl(7, 8, &[(id(7), false)]));
    let mut v2 = base.clone();
    v2.push(VCol { ty: t::scalar(ty::NAT), added: 2, dropped: 0, default: Some(vec![7]) });
    let mut v3 = v2.clone();
    v3[7].dropped = 3;
    let changes = [
        ("an index added at the same version", schema_versioned(1, &base, &with7).0),
        ("an index removed at the same version", schema_versioned(1, &base, &decls()).0),
        ("a column added", schema_versioned(2, &v2, &decls()).0),
        ("a column dropped", schema_versioned(3, &v3, &decls()).0),
    ];
    let mut h = VecHost::new(FILLS[0]);
    let (tb, m) = filled(&mut h, 300);
    let root = tb.root();
    drop(tb);
    let mut img = version(1);
    assert_eq!(Shape::stored(&h, root).unwrap(), Some(derived(&h, root)), "made");
    for (what, next) in &changes {
        let before = bytes(&h, root);
        let old = derived(&h, root);
        {
            let mut tb = Table::open(&mut h, root, &img).unwrap();
            tb.change_schema(next).unwrap();
            maintain_all(&mut tb);
        }
        let s = derived(&h, root);
        assert_ne!(s, old, "{what}: the shape changes");
        assert_eq!(Shape::stored(&h, root).unwrap(), Some(s), "{what}: stored");
        img = next.clone();
        {
            let tb = Table::open(&mut h, root, &img).unwrap();
            assert_eq!((*tb.shape(), tb.stale(), tb.len().unwrap()), (s, false, 300), "{what}: reopened");
            check(&tb, &m);
        }
        set_bytes(&mut h, root, &before);
        assert_eq!(Shape::stored(&h, root).unwrap(), None, "{what}: the earlier schema's shape");
        assert_eq!(reopen(&mut h, root, &img), s, "{what}: the earlier schema's shape is not used");
        assert_eq!(Shape::stored(&h, root).unwrap(), Some(s), "{what}: stored again");
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
        assert_eq!(reopen(&mut h, root, &img), s, "{what}: not used");
        assert_eq!(bytes(&h, root), good, "{what}: stored again");
    }
    // Any byte of the four words checked.
    for k in 0..32 {
        let mut b = good.clone();
        b[k] ^= 0x10;
        set_bytes(&mut h, root, &b);
        assert_eq!(Shape::stored(&h, root).unwrap(), None, "byte {k} changed");
        assert_eq!(reopen(&mut h, root, &img), s, "byte {k} changed: not used");
        assert_eq!(bytes(&h, root), good, "byte {k} changed: stored again");
    }
    // A stored shape that passes the checks, here with another hot state layout, is what an open takes; so is
    // one whose hash changed, which an open does not check.
    let other = with_word(&good, 15, &|w| w + 8);
    set_bytes(&mut h, root, &other);
    let taken = reopen(&mut h, root, &img);
    let mut want = s;
    want.hot_h += 8;
    assert_eq!(taken, want);
    let hash = with_word(&good, 24, &|w| w ^ 1);
    set_bytes(&mut h, root, &hash);
    assert_eq!(reopen(&mut h, root, &img), s, "another hash: taken");
    assert_eq!(bytes(&h, root), hash, "another hash: not stored again");
    set_bytes(&mut h, root, &good);
    let tb = Table::open(&mut h, root, &img).unwrap();
    assert_eq!(*tb.shape(), s);
    check(&tb, &m);
}

/// An image of the table's own version that adds a column, and so a leaf, is refused before anything is
/// written; the table keeps its version and its indexes.
#[test]
fn a_leaf_added_at_the_same_version_is_refused() {
    let mut vc: Vec<VCol> = columns().into_iter().map(|ty| VCol { ty, added: 1, dropped: 0, default: None }).collect();
    vc.push(VCol { ty: t::scalar(ty::NAT), added: 1, dropped: 0, default: None });
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, m) = filled(&mut h, 2_000);
    tb.host_mut().end_message();
    let r = tb.change_schema(&schema_versioned(1, &vc, &decls()).0);
    assert_eq!(r, Err(Fault::Schema(SchemaError::ColumnChanged)));
    assert_eq!(tb.host_mut().end_message(), 0, "refused before anything is written");
    assert_eq!(tb.shape().version, 1);
    check(&tb, &m);
}

/// Texts for `builds_under_writes`, drawn from pools so that keys repeat: short ones (most of them); of 1,000
/// to 4,000 letters; past `KEY_STACK`; mostly `00` bytes, which the key escapes (seven in eight, every eighth,
/// or all but the last); and of 64 KiB to 96 KiB, two sharing their first 60 KiB and two with `00` bytes.
struct BuildTexts(Vec<Vec<Vec<u8>>>);

impl BuildTexts {
    fn new(r: &mut XorShift) -> BuildTexts {
        let letters = |r: &mut XorShift, n: u64| -> Vec<u8> { (0..n).map(|_| b'a' + r.below(26) as u8).collect() };
        let short = (0..50).map(|i| format!("s{i}").into_bytes()).collect();
        let mid = (0..20)
            .map(|_| {
                let n = 1_000 + r.below(3_000);
                letters(r, n)
            })
            .collect();
        let long = (0..6)
            .map(|_| {
                let n = KEY_STACK as u64 + 100 + r.below(4_000);
                letters(r, n)
            })
            .collect();
        let zeros = (0..3u64)
            .map(|z| {
                let n = 1_000 + r.below(3_000);
                (0..n)
                    .map(|i| match z {
                        0 if r.below(8) == 0 => b'a' + r.below(26) as u8,
                        1 if i % 8 != 5 => b'a' + r.below(26) as u8,
                        2 if i + 1 == n => b'z',
                        _ => 0,
                    })
                    .collect()
            })
            .collect();
        let huge = (0..4u64)
            .map(|h| {
                let n = 65_536 + r.below(32_768);
                if h < 2 {
                    let mut t = vec![b'p'; 61_440];
                    t.extend(letters(r, n - 61_440));
                    t
                } else {
                    (0..n).map(|i| if i % 997 == 3 { 0 } else { b'a' + r.below(26) as u8 }).collect()
                }
            })
            .collect();
        BuildTexts(vec![short, mid, long, zeros, huge])
    }

    fn pick(&self, r: &mut XorShift) -> Vec<u8> {
        let c = match r.below(100) {
            0..=1 => 4,
            2..=5 => 2,
            6..=9 => 3,
            10..=24 => 1,
            _ => 0,
        };
        let pool = &self.0[c];
        pool[r.below(pool.len() as u64) as usize].clone()
    }
}

/// An index of W 1,016 added on the Text of a table {Nat key, Text} of several groups (`BuildTexts`), built a
/// group's rows a step. Before each step, writes insert rows (taken at once), give rows new Texts and remove
/// rows, on both sides of the build's position: a row the build passed is kept by every write, a row it has
/// not reached yet is taken with its cells as the build finds them, or not at all once removed. The index then
/// counts every key as the model does, and once every row is removed area 1 holds no block.
#[test]
fn builds_under_writes() {
    let base: Vec<VCol> = [t::scalar(ty::NAT), t::scalar(ty::TEXT)]
        .into_iter()
        .map(|ty| VCol { ty, added: 1, dropped: 0, default: None })
        .collect();
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 9, &schema_versioned(1, &base, &[]).0).unwrap();
    let mut r = XorShift(0xb111_d5e7);
    let texts = BuildTexts::new(&mut r);
    let mut m = Model::new();
    let put = |tb: &mut Table<'_, VecHost>, m: &mut Model, r: &mut XorShift, k: u64| {
        let x = vec![(id(0), V::U(k)), (id(1), V::Bytes(texts.pick(r)))];
        let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&x)).unwrap();
        let (Upsert::Inserted(rid) | Upsert::Replaced(rid)) = u;
        m.insert(k, (rid, x));
    };
    for k in 0..MANY {
        put(&mut tb, &mut m, &mut r, k);
    }
    tb.change_schema(&schema_versioned(2, &base, &[decl(1, 1_016, &[(id(1), false)])]).0).unwrap();
    let (_, end) = Secondary::build_state(tb.host(), tb.root(), 1).unwrap();
    let (mut steps, mut behind, mut ahead, mut next_key) = (0u64, 0u64, 0u64, MANY);
    while !being_built(&tb).is_empty() {
        let (at, _) = Secondary::build_state(tb.host(), tb.root(), 1).unwrap();
        for _ in 0..40 {
            let op = r.below(3);
            if op == 0 {
                put(&mut tb, &mut m, &mut r, next_key);
                next_key += 1;
                continue;
            }
            let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
            if m[&k].0 < at {
                behind += 1;
            } else {
                ahead += 1;
            }
            if op == 1 {
                put(&mut tb, &mut m, &mut r, k);
            } else {
                let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
                assert_eq!(gone, m.remove(&k).map(|x| x.0));
            }
        }
        assert_eq!(one_step(&mut tb), Ran::Build);
        steps += 1;
    }
    assert!(steps >= 3 && steps == steps_to(end), "a step for each group: {steps}");
    assert!(behind >= 20 && ahead >= 20, "writes on both sides of the build: {behind} behind, {ahead} ahead");
    check_counts(&tb, &m, 1, 1);
    for k in m.keys().copied().collect::<Vec<_>>() {
        let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
        assert_eq!(gone, m.remove(&k).map(|x| x.0));
    }
    maintain_all(&mut tb);
    assert_eq!(area1_in_use(&tb), 0, "every row removed");
}

/// Nats and Ints of 100,000 to 140,000 LEB bytes (positive, and negative with their low groups zero for none,
/// half or all but the last of their bytes), among small ones and ones of a few thousand bytes, each key
/// repeated, under an index added on (Int, Nat): the build reads each key from the stored row a piece at a
/// time; the index then counts every key as the model does.
#[test]
fn builds_huge_numbers() {
    let base: Vec<VCol> = [t::scalar(ty::NAT), t::scalar(ty::NAT), t::scalar(ty::INT)]
        .into_iter()
        .map(|ty| VCol { ty, added: 1, dropped: 0, default: None })
        .collect();
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 9, &schema_versioned(1, &base, &[]).0).unwrap();
    let mut r = XorShift(0x1eb_b11d);
    // A canonical form of n bytes as `leb_of` makes, its lowest `zeros` groups zero.
    let mut big = |n: u64, signed: bool, neg: bool, zeros: u64| {
        let mut b = leb_of(&mut r, n as usize, signed, neg);
        b.iter_mut().take(zeros as usize).for_each(|x| *x = 0x80);
        b
    };
    let huge = 100_000;
    let pool: Vec<(V, V)> = vec![
        (V::Int(big(huge + 9_000, true, false, 0)), V::U(3)),
        (V::Int(big(huge + 31_000, true, true, huge + 30_999)), V::Nat(big(huge + 2_000, false, false, 0))),
        (V::Int(big(huge + 17_000, true, true, (huge + 17_000) / 2)), V::Nat(big(3_000, false, false, 0))),
        (V::Int(sleb(-77)), V::Nat(big(huge + 40_000, false, false, 0))),
        (V::Int(big(4_000, true, true, 2_000)), V::U(5)),
        (V::Int(sleb(-5)), V::Nat(big(2_500, false, false, 0))),
        (V::Int(big(20, true, true, 0)), V::Nat(big(20, false, false, 0))),
        (V::Int(big(huge + 1, true, true, 0)), V::Nat(big(huge, false, false, 0))),
    ];
    let mut m = Model::new();
    for k in 0..20u64 {
        let (int, nat) = pool[if k < 8 { k as usize } else { r.below(8) as usize }].clone();
        let x = vec![(id(0), V::U(k)), (id(1), nat), (id(2), int)];
        let (Upsert::Inserted(rid), _) = tb.upsert(&mut key_scratch(), &mut Src::new(&x)).unwrap() else { panic!() };
        m.insert(k, (rid, x));
    }
    maintain_all(&mut tb);
    tb.change_schema(&schema_versioned(2, &base, &[decl(1, 1_016, &[(id(2), false), (id(1), false)])]).0).unwrap();
    assert_eq!(one_step(&mut tb), Ran::Build);
    assert!(being_built(&tb).is_empty());
    let mut want: BTreeMap<String, (Vec<(u32, V)>, i128)> = BTreeMap::new();
    for (_, row) in m.values() {
        let cells = vec![row[2].clone(), row[1].clone()];
        want.entry(format!("{cells:?}")).or_insert((cells, 0)).1 += 1;
    }
    assert_eq!(want.len(), 8);
    for (cells, c) in want.values() {
        assert_eq!(
            tb.count(&mut key_scratch(), 1, &mut Src::new(cells)).unwrap(),
            *c,
            "key of {} bytes",
            format!("{cells:?}").len()
        );
    }
}
