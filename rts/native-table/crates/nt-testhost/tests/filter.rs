//! Filtered scans against a model: the rows (`scan_where`, read in batches), their count (`count_where`) and
//! the sum of each integer column over them (`sum_where`) for equalities and ranges with included, excluded
//! and open ends, on Nat, Int, option Int, Text and Bool columns, nulls included; and only the columns asked
//! for, without the key and with it first: one column, several (the key and repeats among them, in any order),
//! every column, or more than a scan keeps found (`scan_cells`, read in batches), under each filter and under
//! none. The table goes through
//! inserts, updates (of buffered and sealed rows, moving values into and out of their groups' extremes),
//! removes, seals, a compaction run unit by unit with writes between the units, and a schema change that adds
//! a Nat column with a default and an option Bool column. Then a sealed group's minimum and maximum are seen
//! to skip and pass groups: by the reads a count takes, and by the answers after updates that move a value
//! into and out of a group's range.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;

use native_table::consts::{GROUP_SLOTS, PAGE};
use native_table::fault::Fault;
use native_table::table::filter::{Cond, Filter};
use native_table::table::jobs::Ran;
use native_table::table::{CellIn, CellOut, RowSink, Table, Upsert};
use native_table::value::ty;
use native_table::{Progress, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{schema_versioned, t, Sink, Src, VCol, V};
use nt_testhost::{open_group, TestBudget, VecHost, VecScratch, XorShift, FILLS};

fn id(col: u32) -> u32 {
    col << 16
}

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;
const S: u32 = 4;
const F: u32 = 5;
const D: u32 = 6;
const E: u32 = 7;

/// Version 1: {key Nat, a Nat, b ?Int, c Int, s Text, f Bool}; version 2 adds d Nat (default 7) and e ?Bool.
fn version(v: u32) -> Vec<u8> {
    let col = |ty: Vec<u8>, added: u32, default: Option<Vec<u8>>| VCol { ty, added, dropped: 0, default };
    let mut vc = vec![
        col(t::scalar(ty::NAT), 1, None),
        col(t::scalar(ty::NAT), 1, None),
        col(t::opt(t::scalar(ty::INT)), 1, None),
        col(t::scalar(ty::INT), 1, None),
        col(t::scalar(ty::TEXT), 1, None),
        col(t::scalar(ty::BOOL), 1, None),
    ];
    if v >= 2 {
        vc.push(col(t::scalar(ty::NAT), 2, Some(vec![7])));
        vc.push(col(t::opt(t::scalar(ty::BOOL)), 2, None));
    }
    schema_versioned(v, &vc, &[]).0
}

/// A canonical LEB128 form of `groups` 7-bit groups, signed: a value beyond i128 from 20 groups on.
fn big_leb(r: &mut XorShift, groups: usize, negative: bool) -> Vec<u8> {
    let mut v: Vec<u8> = (0..groups).map(|_| (r.next_u64() as u8 & 0x7F) | 0x80).collect();
    let last = v.last_mut().unwrap();
    *last = if negative { 0x40 | (*last & 0x3E) } else { (*last & 0x3F) | 1 };
    v
}

/// Column c's value: an Int beyond i128 one time in `big_one_in`, else small.
fn c_value(r: &mut XorShift, big_one_in: u64) -> V {
    if r.below(big_one_in) == 0 {
        let neg = r.below(2) == 0;
        let groups = 20 + r.below(20) as usize;
        V::Int(big_leb(r, groups, neg))
    } else {
        V::I(r.below(201) as i64 - 100)
    }
}

/// A row with key k at schema version v: a near k / 4 (so that groups hold narrow ranges of it), b null one
/// time in six, c rarely beyond i128.
fn row(r: &mut XorShift, v: u32, k: u64) -> Vec<(u32, V)> {
    let b = if r.below(6) == 0 { V::Null } else { V::I(r.below(1_001) as i64 - 500) };
    let mut x = vec![
        (id(0), V::U(k)),
        (id(A), V::U(k / 4 + r.below(50))),
        (id(B), b),
        (id(C), c_value(r, 5_000)),
        (id(S), V::text(&format!("k{}", r.below(20)))),
        (id(F), V::Bool(r.below(2) == 0)),
    ];
    if v >= 2 {
        x.push((id(D), V::U(r.below(15))));
        x.push((id(E), if r.below(3) == 0 { V::Null } else { V::Bool(r.below(2) == 0) }));
    }
    x
}

fn cell(v: &V) -> CellIn<'_> {
    match v {
        V::Null => CellIn::Null,
        V::Bool(b) => CellIn::Bool(*b),
        V::U(x) => CellIn::U64(*x),
        V::I(x) => CellIn::I64(*x),
        V::Bytes(b) => CellIn::Bytes(b),
        V::Nat(b) => CellIn::Nat(b),
        V::Int(b) => CellIn::Int(b),
        other => panic!("{other:?}"),
    }
}

/// A filter, owned.
#[derive(Clone, Debug)]
enum MCond {
    Eq(V),
    Range(Bound<V>, Bound<V>),
}

fn bound(b: &Bound<V>) -> Bound<CellIn<'_>> {
    match b {
        Bound::Included(v) => Bound::Included(cell(v)),
        Bound::Excluded(v) => Bound::Excluded(cell(v)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

fn filter(col: u32, c: &MCond) -> Filter<'_> {
    let cond = match c {
        MCond::Eq(v) => Cond::Eq(cell(v)),
        MCond::Range(a, b) => Cond::Range(bound(a), bound(b)),
    };
    Filter { leaf: id(col), cond }
}

fn big(v: &V) -> Big {
    match v {
        V::U(x) => Big::from_i128(i128::from(*x)),
        V::I(x) => Big::from_i128(i128::from(*x)),
        V::Nat(b) => Big::from_leb(b, false),
        V::Int(b) => Big::from_leb(b, true),
        other => panic!("{other:?}"),
    }
}

/// The model's order of two non-null values of one column.
fn order(a: &V, b: &V) -> Ordering {
    match (a, b) {
        (V::Bytes(x), V::Bytes(y)) => x.cmp(y),
        (V::Bool(x), V::Bool(y)) => x.cmp(y),
        _ => big(a).order(&big(b)),
    }
}

fn meets(v: &V, c: &MCond) -> bool {
    match c {
        MCond::Eq(V::Null) => *v == V::Null,
        _ if *v == V::Null => false,
        MCond::Eq(x) => order(v, x) == Ordering::Equal,
        MCond::Range(lo, hi) => {
            let lo_ok = match lo {
                Bound::Included(x) => order(v, x) != Ordering::Less,
                Bound::Excluded(x) => order(v, x) == Ordering::Greater,
                Bound::Unbounded => true,
            };
            let hi_ok = match hi {
                Bound::Included(x) => order(v, x) != Ordering::Greater,
                Bound::Excluded(x) => order(v, x) == Ordering::Less,
                Bound::Unbounded => true,
            };
            lo_ok && hi_ok
        }
    }
}

/// The model: key → (rid, cells in column order).
type Model = BTreeMap<u64, (u64, Vec<(u32, V)>)>;

/// The model's matching rows in rid order.
fn want_rows(m: &Model, col: u32, c: &MCond) -> Vec<(u64, Vec<(u32, V)>)> {
    let mut v: Vec<_> = m.values().filter(|(_, r)| meets(&r[col as usize].1, c)).cloned().collect();
    v.sort_by_key(|x| x.0);
    v
}

/// Collects the rows a scan hands out.
#[derive(Default)]
struct Collect {
    cur: Sink,
    rows: Vec<(u64, Vec<(u32, V)>)>,
}

impl RowSink for Collect {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        self.cur.cell(leaf, v)
    }
    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        let cells = std::mem::take(&mut self.cur.cells);
        self.rows.push((rid, cells));
        Ok(())
    }
}

/// Every matching row through `scan_where` in batches of random sizes (0 included), resumed where each ended.
fn scan_all(tb: &Table<'_, VecHost>, f: &Filter<'_>, r: &mut XorShift) -> Vec<(u64, Vec<(u32, V)>)> {
    batches(r, |next, limit, out| tb.scan_where(f, next, limit, out).unwrap())
}

/// Every row `scan` hands out from a rid at most a number of rows a call, in batches of random sizes (0
/// included), each resumed where the last ended.
fn batches(r: &mut XorShift, mut scan: impl FnMut(u64, u64, &mut Collect) -> Option<u64>) -> Vec<(u64, Vec<(u32, V)>)> {
    let mut out = Collect::default();
    let mut next = 0u64;
    loop {
        let limit = match r.below(10) {
            0 => 0,
            1 => 1,
            2..=5 => 64,
            _ => 1 + r.below(1_000),
        };
        let before = out.rows.len();
        match scan(next, limit, &mut out) {
            Some(n) => {
                assert!(n > next || (limit == 0 && n == next), "a scan goes on");
                assert_eq!(out.rows.len() - before, limit as usize, "a batch that stops early is full");
                next = n;
            }
            None => {
                assert!(out.rows.len() - before <= limit as usize);
                return out.rows;
            }
        }
    }
}

/// The filters a check runs, some from values the model holds.
fn filters(m: &Model, v: u32, r: &mut XorShift) -> Vec<(u32, MCond)> {
    use Bound::{Excluded as Ex, Included as In, Unbounded as Open};
    let rows: Vec<&Vec<(u32, V)>> = m.values().map(|(_, x)| x).collect();
    let pick = |r: &mut XorShift, c: u32| rows[r.below(rows.len() as u64) as usize][c as usize].1.clone();
    let a = pick(r, A);
    let top = m.keys().next_back().copied().unwrap_or(0) / 4 + 50;
    let lo = r.below(top);
    let span = r.below(3_000);
    let mut b = pick(r, B);
    if b == V::Null {
        b = V::I(7);
    }
    let bigs: Vec<V> = rows.iter().map(|x| x[C as usize].1.clone()).filter(|x| matches!(x, V::Int(_))).collect();
    let big_c =
        if bigs.is_empty() { V::Int(big_leb(r, 25, false)) } else { bigs[r.below(bigs.len() as u64) as usize].clone() };
    let neg_big = V::Int(big_leb(r, 22, true));
    let txt = |s: &str| V::text(s);
    let mut f = vec![
        (A, MCond::Eq(a.clone())),
        (A, MCond::Range(In(V::U(lo)), Ex(V::U(lo + span)))),
        (A, MCond::Range(Open, In(a.clone()))),
        (A, MCond::Range(Ex(a), Open)),
        (A, MCond::Range(In(V::U(0)), Open)),
        (A, MCond::Range(Open, Open)),
        (A, MCond::Eq(V::U(u64::MAX))),
        (A, MCond::Range(In(V::U(1 << 19)), Open)),
        (A, MCond::Range(In(V::U(lo)), In(V::U(lo.saturating_sub(1))))),
        (B, MCond::Eq(V::Null)),
        (B, MCond::Eq(b)),
        (B, MCond::Range(In(V::I(-100)), Ex(V::I(100)))),
        (B, MCond::Range(Open, Ex(V::I(0)))),
        (B, MCond::Range(Ex(V::I(400)), Open)),
        (C, MCond::Range(In(V::I(-50)), In(V::I(50)))),
        (C, MCond::Range(In(V::I(200)), Open)),
        (C, MCond::Range(Open, In(V::I(100)))),
        (C, MCond::Eq(big_c.clone())),
        (C, MCond::Range(Ex(big_c), Open)),
        (C, MCond::Range(Open, Ex(neg_big.clone()))),
        (C, MCond::Range(In(neg_big), In(V::I(-90)))),
        (S, MCond::Eq(txt("k7"))),
        (S, MCond::Range(In(txt("k1")), Ex(txt("k3")))),
        (S, MCond::Range(Ex(txt("k5")), Open)),
        (S, MCond::Range(Open, In(txt("k10")))),
        (F, MCond::Eq(V::Bool(true))),
        (F, MCond::Range(Open, Ex(V::Bool(true)))),
    ];
    if v >= 2 {
        f.extend([
            (D, MCond::Eq(V::U(7))),
            (D, MCond::Range(In(V::U(5)), In(V::U(9)))),
            (E, MCond::Eq(V::Null)),
            (E, MCond::Eq(V::Bool(false))),
        ]);
    }
    f
}

/// Columns for a cell scan, by `which`: 0 one column; 1 two to five, the key column and repeats among them, in
/// any order; 2 every column but the key in column order; 3 seventy, past the leaves a scan keeps found.
fn columns(v: u32, which: u64, r: &mut XorShift) -> Vec<u32> {
    let last = if v >= 2 { E } else { F };
    match which {
        0 => vec![1 + r.below(u64::from(last)) as u32],
        1 => (0..2 + r.below(4)).map(|_| r.below(u64::from(last) + 1) as u32).collect(),
        2 => (A..=last).collect(),
        _ => (0..70).map(|_| r.below(u64::from(last) + 1) as u32).collect(),
    }
}

/// The rows of `want` (whole, in rid order) as a cell scan of columns `cols` hands them out: `cols` in the order
/// given.
fn want_cells(want: &[(u64, Vec<(u32, V)>)], cols: &[u32]) -> Vec<(u64, Vec<(u32, V)>)> {
    let pick = |x: &Vec<(u32, V)>| cols.iter().map(|&c| x[c as usize].clone()).collect();
    want.iter().map(|(rid, x)| (*rid, pick(x))).collect()
}

/// The cell scans of columns `cols`, without the key and with the key first, under `f` (none: every row)
/// against the model's rows `want`.
fn check_cells(
    tb: &Table<'_, VecHost>,
    f: Option<&Filter<'_>>,
    want: &[(u64, Vec<(u32, V)>)],
    cols: &[u32],
    r: &mut XorShift,
    what: &str,
) {
    let keyed: Vec<u32> = std::iter::once(0).chain(cols.iter().copied()).collect();
    for cols in [cols, &keyed[..]] {
        let ids: Vec<u32> = cols.iter().map(|&c| id(c)).collect();
        let got = batches(r, |next, limit, out| tb.scan_cells(&ids, f, next, limit, out).unwrap());
        assert_eq!(got.len(), want.len(), "cell rows, columns {cols:?}: {what}");
        assert!(got == want_cells(want, cols), "cells differ, columns {cols:?}: {what}");
    }
}

/// Every filter's rows, count and sums, and the cell scans with and without a filter, against the model.
/// Returns the matches over all filters.
fn check(tb: &Table<'_, VecHost>, m: &Model, v: u32, r: &mut XorShift) -> u64 {
    let mut buf = vec![0u8; 4_096];
    let mut matched = 0u64;
    let mut every: Vec<_> = m.values().cloned().collect();
    every.sort_by_key(|x| x.0);
    for which in 0..4 {
        let cols = columns(v, which, r);
        check_cells(tb, None, &every, &cols, r, "no filter");
    }
    for (i, (col, c)) in filters(m, v, r).into_iter().enumerate() {
        let f = filter(col, &c);
        let want = want_rows(m, col, &c);
        let what = format!("column {col}, {c:?}");
        assert_eq!(tb.count_where(&f).unwrap(), want.len() as u64, "count: {what}");
        let got = scan_all(tb, &f, r);
        assert_eq!(got.len(), want.len(), "rows: {what}");
        assert!(got == want, "rows differ: {what}");
        let cols = columns(v, i as u64 % 3, r);
        check_cells(tb, Some(&f), &want, &cols, r, &what);
        // The sum of c (another leaf's, or the filter's own) and of the filter's own integer leaf.
        let own = [A, B, D].contains(&col).then_some(col);
        for s in [C].into_iter().chain(own) {
            let vals: Vec<&V> = want.iter().map(|(_, x)| &x[s as usize].1).filter(|x| **x != V::Null).collect();
            let sum = vals.iter().fold(Big::default(), |acc, x| acc.add(&big(x)));
            let (t, n) = tb.sum_where(&f, id(s), &mut buf).unwrap();
            assert_eq!((Big::of_total(t, &buf), n), (sum.clone(), vals.len() as u64), "sum of {s}: {what}");
            // A sum or a value past i128 needs room in the buffer: too little is `Scratch`, and the room it names
            // holds the answer.
            if vals.iter().any(|x| matches!(x, V::Int(_))) {
                let need = match tb.sum_where(&f, id(s), &mut buf[..8]) {
                    Err(Fault::Scratch { need }) => need as usize,
                    other => panic!("a short buffer: {other:?}"),
                };
                let mut room = vec![0u8; need];
                let (t, n) = tb.sum_where(&f, id(s), &mut room).unwrap();
                assert_eq!((Big::of_total(t, &room), n), (sum, vals.len() as u64), "sum of {s} in {need} bytes");
            }
        }
        matched += want.len() as u64;
    }
    matched
}

fn upsert(tb: &mut Table<'_, VecHost>, m: &mut Model, x: Vec<(u32, V)>) {
    let k = match x[0].1 {
        V::U(k) => k,
        _ => unreachable!(),
    };
    let rid = match tb.upsert(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x)) {
        Ok((Upsert::Inserted(rid) | Upsert::Replaced(rid), _)) => rid,
        Err(Fault::Busy) => {
            // A refused update of a sealed row: removed and inserted again.
            assert!(tb.remove(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x[..1])).unwrap().0.is_some());
            match tb.upsert(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x)).unwrap().0 {
                Upsert::Inserted(rid) => rid,
                u => panic!("{u:?}"),
            }
        }
        Err(e) => panic!("upsert: {e:?}"),
    };
    m.insert(k, (rid, x));
}

fn remove(tb: &mut Table<'_, VecHost>, m: &mut Model, k: u64) {
    let gone = tb.remove(&mut VecScratch::new(FILLS[0]), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0;
    assert_eq!(gone.is_some(), m.remove(&k).is_some());
}

/// An update of an existing row: new values, or one column moved, a's often far from its group's range.
fn update(tb: &mut Table<'_, VecHost>, m: &mut Model, v: u32, r: &mut XorShift) {
    let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
    let mut x = m[&k].1.clone();
    match r.below(5) {
        0 => x = row(r, v, k),
        1 => x[A as usize].1 = V::U(r.below(1 << 20)),
        2 => x[A as usize].1 = V::U(k / 4 + r.below(50)),
        3 => x[C as usize].1 = c_value(r, 50),
        _ => {
            let y = row(r, v, k);
            x[B as usize] = y[B as usize].clone();
            x[S as usize] = y[S as usize].clone();
            if v >= 2 {
                x[D as usize] = y[D as usize].clone();
            }
        }
    }
    upsert(tb, m, x);
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

#[test]
fn filtered_scans_model() {
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 41, &version(1)).unwrap();
    let mut m: Model = BTreeMap::new();
    let mut r = XorShift(0x5EED_F117);
    let mut v = 1u32;
    let mut next_key = 0u64;
    let (mut units, mut compaction_units, mut checks) = (0u64, 0u64, 0u64);
    let mut unit = |tb: &mut Table<'_, VecHost>| -> Ran {
        let ran = tb.step().unwrap();
        units += 1;
        ran
    };

    // Rows past three groups, keys rising, with updates, removes and maintenance steps between them.
    let mut step = 0u64;
    while next_key < 3 * GROUP_SLOTS + GROUP_SLOTS / 2 {
        step += 1;
        match r.below(100) {
            0..=79 => {
                let k = next_key;
                next_key += 1 + r.below(4) / 3;
                let x = row(&mut r, v, k);
                upsert(&mut tb, &mut m, x);
            }
            80..=91 if !m.is_empty() => update(&mut tb, &mut m, v, &mut r),
            92..=96 if !m.is_empty() => {
                let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
                remove(&mut tb, &mut m, k);
            }
            _ => {
                unit(&mut tb);
            }
        }
        if step % 4_000 == 0 {
            check(&tb, &m, v, &mut r);
            checks += 1;
        }
    }
    check(&tb, &m, v, &mut r);
    maintain_all(&mut tb);
    check(&tb, &m, v, &mut r);

    // Half of group 0's rows removed: its compaction, run a step at a time, with updates and removes between
    // the steps, checked as it goes.
    let in_group0: Vec<u64> = m.iter().filter(|(_, (rid, _))| rid / GROUP_SLOTS == 0).map(|(k, _)| *k).collect();
    for k in in_group0.iter().step_by(2) {
        remove(&mut tb, &mut m, *k);
    }
    let mut n = 0u64;
    while tb.wake().unwrap() != Wake::Idle {
        if unit(&mut tb) == Ran::Compaction {
            compaction_units += 1;
        }
        n += 1;
        if n <= 150 {
            for _ in 0..3 {
                update(&mut tb, &mut m, v, &mut r);
            }
            if r.below(4) == 0 {
                let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
                remove(&mut tb, &mut m, k);
            }
        }
        if n % 30 == 1 && n <= 150 {
            check(&tb, &m, v, &mut r);
            checks += 1;
        }
        assert!(n < 100_000, "maintenance ends");
    }
    check(&tb, &m, v, &mut r);

    // A schema change: d (default 7) and e (null) for every row there is, then rows of version 2.
    v = 2;
    tb.change_schema(&version(2)).unwrap();
    for (_, x) in m.values_mut() {
        x.push((id(D), V::U(7)));
        x.push((id(E), V::Null));
    }
    check(&tb, &m, v, &mut r);
    let end = next_key + GROUP_SLOTS + 500;
    while next_key < end {
        step += 1;
        match r.below(100) {
            0..=74 => {
                let k = next_key;
                next_key += 1;
                let x = row(&mut r, v, k);
                upsert(&mut tb, &mut m, x);
            }
            75..=91 => update(&mut tb, &mut m, v, &mut r),
            92..=96 => {
                let k = *m.keys().nth(r.below(m.len() as u64) as usize).unwrap();
                remove(&mut tb, &mut m, k);
            }
            _ => {
                unit(&mut tb);
            }
        }
        if step % 1_500 == 0 {
            check(&tb, &m, v, &mut r);
            checks += 1;
        }
    }
    check(&tb, &m, v, &mut r);
    maintain_all(&mut tb);
    let matched = check(&tb, &m, v, &mut r);
    let seals = open_group(tb.host(), tb.root());
    eprintln!(
        "rows {}, steps {units} (compaction {compaction_units}), seals {seals}, checks {checks}, matched {matched}",
        m.len()
    );
    assert!(seals >= 3, "seals: {seals}");
    assert!(compaction_units >= 1, "compaction steps: {compaction_units}");

    // Bad filters.
    let mut buf = [0u8; 64];
    let bad = |f: Filter<'_>| tb.count_where(&f).unwrap_err();
    assert_eq!(bad(Filter { leaf: id(9), cond: Cond::Eq(CellIn::U64(1)) }), Fault::BadRef, "no such leaf");
    assert_eq!(bad(Filter { leaf: id(A), cond: Cond::Eq(CellIn::Null) }), Fault::BadValue { leaf: id(A) });
    assert_eq!(
        bad(Filter { leaf: id(B), cond: Cond::Range(Bound::Included(CellIn::Null), Bound::Unbounded) }),
        Fault::BadValue { leaf: id(B) },
        "a null end"
    );
    assert_eq!(bad(Filter { leaf: id(A), cond: Cond::Eq(CellIn::Bytes(b"x")) }), Fault::BadValue { leaf: id(A) });
    let all = Filter { leaf: id(A), cond: Cond::Range(Bound::Unbounded, Bound::Unbounded) };
    assert_eq!(tb.sum_where(&all, id(S), &mut buf), Err(Fault::BadRef), "a sum of a Text column");
    let mut sink = Collect::default();
    assert_eq!(tb.scan_cells(&[id(A), id(9)], None, 0, 10, &mut sink), Err(Fault::BadRef), "a cell of no column");
    assert!(sink.rows.is_empty());
}

/// Reads the table takes for `run`, every page of the host watched.
fn reads<T>(tb: &mut Table<'_, VecHost>, run: impl FnOnce(&Table<'_, VecHost>) -> T) -> (T, u64) {
    let h = tb.host_mut();
    h.watched = (1..=h.memory().len() as u64 / PAGE).map(|k| k * PAGE).collect();
    h.watched_reads.set(0);
    let out = run(tb);
    let n = tb.host().watched_reads.get();
    tb.host_mut().watched.clear();
    (out, n)
}

/// Sealed groups skipped and passed by a's minimum and maximum: a count that every sealed group passes reads
/// a few blocks a group, and one that rules all groups but one out reads that group's rows alone; updates that
/// move a value into and out of a group's range change the answers as they should.
#[test]
fn groups_skipped_and_passed() {
    let mut h = VecHost::new(FILLS[0]);
    h.set_logging(false);
    let (mut tb, _) = Table::create(&mut h, 42, &version(1)).unwrap();
    let mut m: Model = BTreeMap::new();
    let n = 4 * GROUP_SLOTS + 10;
    let at = |k: u64, a: u64| {
        vec![
            (id(0), V::U(k)),
            (id(A), V::U(a)),
            (id(B), V::Null),
            (id(C), V::I(0)),
            (id(S), V::text("x")),
            (id(F), V::Bool(false)),
        ]
    };
    for k in 0..n {
        upsert(&mut tb, &mut m, at(k, k));
    }
    maintain_all(&mut tb);
    let count = |tb: &mut Table<'_, VecHost>, c: MCond| {
        let (got, reads) = reads(tb, |tb| tb.count_where(&filter(A, &c)).unwrap());
        assert_eq!(got, want_rows(&m, A, &c).len() as u64, "{c:?}");
        (got, reads)
    };
    use Bound::{Excluded as Ex, Included as In, Unbounded as Open};
    // Every sealed group passed, the 10 buffered rows tested.
    let (all, r_all) = count(&mut tb, MCond::Range(In(V::U(0)), Open));
    assert_eq!(all, n);
    // Every group but group 1 skipped.
    let (_, r_one) = count(&mut tb, MCond::Range(In(V::U(5_000)), Ex(V::U(5_100))));
    // Every row tested: a Text column has no extremes.
    let (_, r_text) =
        reads(&mut tb, |tb| tb.count_where(&Filter { leaf: id(S), cond: Cond::Eq(CellIn::Bytes(b"x")) }).unwrap());
    eprintln!("reads: all passed {r_all}, one group tested {r_one}, every row tested {r_text}");
    assert!(r_all < 200, "a passed group reads no row: {r_all}");
    assert!(r_one * 3 < r_text, "skipped groups read no row: {r_one} against {r_text}");

    // A value moved into a group's range from outside it, then out again, and a row removed.
    let mut m = m;
    let eq = MCond::Range(In(V::U(9_000)), In(V::U(9_001)));
    let low = MCond::Range(Open, Ex(V::U(GROUP_SLOTS)));
    let counts = |tb: &Table<'_, VecHost>, m: &Model| {
        let c = |c: &MCond| {
            let got = tb.count_where(&filter(A, c)).unwrap();
            assert_eq!(got, want_rows(m, A, c).len() as u64, "{c:?}");
            let mut buf = [0u8; 64];
            let (t, k) = tb.sum_where(&filter(A, c), id(A), &mut buf).unwrap();
            let sum = want_rows(m, A, c).iter().fold(Big::default(), |s, (_, x)| s.add(&big(&x[A as usize].1)));
            assert_eq!((Big::of_total(t, &buf), k), (sum, got));
            got
        };
        (c(&eq), c(&low))
    };
    assert_eq!(counts(&tb, &m), (2, GROUP_SLOTS));
    // Row 100 of group 0 updated to 9,000: group 0's maximum widens, so it is neither skipped for [9,000,
    // 9,001] nor passed for a < 4,096.
    upsert(&mut tb, &mut m, at(100, 9_000));
    assert_eq!(counts(&tb, &m), (3, GROUP_SLOTS - 1));
    // Back to 100: the maximum is loose and still bounds the group; the answers follow the row.
    upsert(&mut tb, &mut m, at(100, 100));
    assert_eq!(counts(&tb, &m), (2, GROUP_SLOTS));
    // Row 200 removed.
    remove(&mut tb, &mut m, 200);
    assert_eq!(counts(&tb, &m), (2, GROUP_SLOTS - 1));
    // Row 9,000 of group 2 updated out of [9,000, 9,001] and below 4,096: group 2 is now tested for both.
    upsert(&mut tb, &mut m, at(9_000, 5));
    assert_eq!(counts(&tb, &m), (1, GROUP_SLOTS));
}
