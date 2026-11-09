//! A sealed group's minimum and maximum skip and pass groups under a filter: by the reads a count takes, and by
//! the answers after updates that move a value into and out of a group's range.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;

use native_table::consts::{GROUP_SLOTS, PAGE};
use native_table::table::filter::{Cond, Filter};
use native_table::table::{CellIn, Table, Upsert};
use native_table::value::ty;
use native_table::{Progress, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{schema_versioned, t, Src, VCol, V};
use nt_testhost::{TestBudget, VecHost, VecScratch, FILLS};

fn id(col: u32) -> u32 {
    col << 16
}

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;
const S: u32 = 4;
const F: u32 = 5;

/// Version 1: {key Nat, a Nat, b ?Int, c Int, s Text, f Bool}.
fn version(v: u32) -> Vec<u8> {
    let col = |ty: Vec<u8>, added: u32, default: Option<Vec<u8>>| VCol { ty, added, dropped: 0, default };
    let vc = vec![
        col(t::scalar(ty::NAT), 1, None),
        col(t::scalar(ty::NAT), 1, None),
        col(t::opt(t::scalar(ty::INT)), 1, None),
        col(t::scalar(ty::INT), 1, None),
        col(t::scalar(ty::TEXT), 1, None),
        col(t::scalar(ty::BOOL), 1, None),
    ];
    schema_versioned(v, &vc, &[]).0
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
    let MCond::Range(a, b) = c;
    Filter { leaf: id(col), cond: Cond::Range(bound(a), bound(b)) }
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
    let MCond::Range(lo, hi) = c;
    if *v == V::Null {
        return false;
    }
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

/// The model: key → (rid, cells in column order).
type Model = BTreeMap<u64, (u64, Vec<(u32, V)>)>;

/// The model's matching rows in rid order.
fn want_rows(m: &Model, col: u32, c: &MCond) -> Vec<(u64, Vec<(u32, V)>)> {
    let mut v: Vec<_> = m.values().filter(|(_, r)| meets(&r[col as usize].1, c)).cloned().collect();
    v.sort_by_key(|x| x.0);
    v
}

fn upsert(tb: &mut Table<'_, VecHost>, m: &mut Model, x: Vec<(u32, V)>) {
    let k = match x[0].1 {
        V::U(k) => k,
        _ => unreachable!(),
    };
    let rid = match tb.upsert(&mut VecScratch::new(FILLS[0]), &mut Src::new(&x)) {
        Ok((Upsert::Inserted(rid) | Upsert::Replaced(rid), _)) => rid,
        Err(e) => panic!("upsert: {e:?}"),
    };
    m.insert(k, (rid, x));
}

fn remove(tb: &mut Table<'_, VecHost>, m: &mut Model, k: u64) {
    let gone = tb.remove(&mut VecScratch::new(FILLS[0]), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0;
    assert_eq!(gone.is_some(), m.remove(&k).is_some());
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
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
