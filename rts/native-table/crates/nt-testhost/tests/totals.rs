//! The integer leaves' totals against a model: random inserts, updates and deletes of buffered and sealed
//! rows, seals, nulls and integers of every size (Nat and Int past 64 bits, past i128 and out of
//! line), in a narrow schema whose totals lie in the hot state and in a wide one whose later totals lie in the
//! totals space; then huge Nat and Int values written, updated, deleted and sealed. `Table::sum` must equal
//! the model's exact sums and counts at every check.
//!
//! Every test runs once per fill pattern (`FILLS`): the results and the host-call logs must be equal, so no
//! byte is read that was not written.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use native_table::consts::{HUGE_VALUE, PAGE};
use native_table::fault::Fault;
use native_table::table::hot::RunCursor;
use native_table::table::layout::Ty;
use native_table::table::totals::{At, Total};
use native_table::table::{read, root, Table, Upsert};
use native_table::value::ty;
use native_table::{PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{rid_of, schema, t, uleb, Src, V};
use nt_testhost::{open_group, Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

/// A digest of host calls, folded as they are taken.
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

fn id(col: u32) -> u32 {
    col << 16
}

/// Scratch for keys longer than a point operation's stack.
fn key_scratch() -> VecScratch {
    VecScratch::new(FILLS[0])
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&TestBudget::new(u64::MAX)).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

/// The canonical signed LEB128 of an i128.
fn sleb(mut x: i128) -> Vec<u8> {
    let mut v = Vec::new();
    loop {
        let b = (x & 0x7F) as u8;
        x >>= 7;
        let done = (x == 0 && b & 0x40 == 0) || (x == -1 && b & 0x40 != 0);
        v.push(if done { b } else { b | 0x80 });
        if done {
            return v;
        }
    }
}

/// A canonical LEB128 form of `groups` 7-bit groups, signed or not, negative or not.
fn leb_of(r: &mut XorShift, groups: usize, signed: bool, negative: bool) -> Vec<u8> {
    let mut v: Vec<u8> = (0..groups).map(|_| (r.next_u64() as u8 & 0x7F) | 0x80).collect();
    let last = v.last_mut().unwrap();
    *last = match (signed, negative) {
        (true, true) => 0x40 | (*last & 0x3E),
        (true, false) => (*last & 0x3F) | 1,
        (false, _) => (*last & 0x7F) | 1,
    };
    v
}

/// One integer leaf's random value: null sometimes when nullable; Nat and Int of every size, past 64 bits,
/// past i128 and out of line.
fn value(r: &mut XorShift, t: Ty, nullable: bool) -> V {
    if nullable && r.below(8) == 0 {
        return V::Null;
    }
    let neg = r.below(2) == 0;
    match t {
        Ty::U(w) => V::U(r.next_u64() >> (64 - 8 * u32::from(w))),
        Ty::I(w) => V::I((r.next_u64() as i64) >> (64 - 8 * u32::from(w))),
        Ty::U128 => V::U128((u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) >> r.below(128)),
        Ty::I128 => V::I128(((u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) as i128) >> r.below(128)),
        Ty::Nat | Ty::Int => {
            let signed = t == Ty::Int;
            let more = r.below(2_000) as usize;
            let leb = match r.below(1_000) {
                0..=449 => {
                    let x = r.next_u64() >> r.below(64);
                    if signed {
                        sleb(if neg { -i128::from(x) } else { i128::from(x) })
                    } else {
                        uleb(u128::from(x))
                    }
                }
                450..=699 => {
                    let x = (u128::from(r.next_u64()) << 63 | u128::from(r.next_u64())) >> r.below(60);
                    if signed {
                        sleb(if neg { -(x as i128) } else { x as i128 })
                    } else {
                        uleb(x)
                    }
                }
                700..=969 => leb_of(r, 19 + more % 30, signed, neg),
                970..=989 => leb_of(r, 2_100 + more % 400, signed, neg),
                _ => leb_of(r, 300 + more % 100, signed, neg),
            };
            if signed {
                V::Int(leb)
            } else {
                V::Nat(leb)
            }
        }
        _ => unreachable!(),
    }
}

/// A schema: its image and, by column, the leaf type and whether it is nullable. Column 0 is the Nat key;
/// the last two are a Float and a Text, which have no total.
struct Schema {
    img: Vec<u8>,
    cols: Vec<(Ty, bool)>,
}

fn build(cols: &[(Ty, bool)]) -> Schema {
    let code = |t: Ty| match t {
        Ty::U(1) => ty::U8,
        Ty::U(2) => ty::U16,
        Ty::U(4) => ty::U32,
        Ty::U(_) => ty::U64,
        Ty::I(1) => ty::I8,
        Ty::I(2) => ty::I16,
        Ty::I(4) => ty::I32,
        Ty::I(_) => ty::I64,
        Ty::Nat => ty::NAT,
        Ty::Int => ty::INT,
        Ty::U128 => ty::U128,
        Ty::I128 => ty::I128,
        Ty::F64 => ty::F64,
        Ty::Text => ty::TEXT,
        _ => unreachable!(),
    };
    let mut all: Vec<(Ty, bool)> = vec![(Ty::Nat, false)];
    all.extend_from_slice(cols);
    all.push((Ty::F64, true));
    all.push((Ty::Text, false));
    let types: Vec<Vec<u8>> =
        all.iter().map(|&(t, n)| if n { t::opt(t::scalar(code(t))) } else { t::scalar(code(t)) }).collect();
    Schema { img: schema(&types).0, cols: all }
}

fn narrow() -> Schema {
    build(&[
        (Ty::U(1), true),
        (Ty::I(2), false),
        (Ty::U(8), false),
        (Ty::I(8), true),
        (Ty::Nat, true),
        (Ty::Int, false),
        (Ty::U128, true),
        (Ty::I128, false),
        (Ty::Int, true),
    ])
}

/// 86 integer leaves: the hot state holds the first totals, the totals space the rest.
fn wide() -> Schema {
    let mut cols: Vec<(Ty, bool)> =
        (0..80).map(|i| (if i % 2 == 0 { Ty::U(2) } else { Ty::I(4) }, i % 5 == 0)).collect();
    cols.extend([(Ty::Nat, true), (Ty::Int, true), (Ty::U128, false), (Ty::I128, true), (Ty::I(8), true)]);
    build(&cols)
}

fn row(s: &Schema, r: &mut XorShift, k: u64) -> Vec<(u32, V)> {
    let n = s.cols.len();
    let mut out = vec![(id(0), V::U(k))];
    for (c, &(t, nullable)) in s.cols.iter().enumerate().take(n - 2).skip(1) {
        out.push((id(c as u32), value(r, t, nullable)));
    }
    out.push((id(n as u32 - 2), V::F64((r.below(1_000) as f64 / 3.0).to_bits())));
    out.push((id(n as u32 - 1), V::Bytes(vec![b'a' + r.below(26) as u8; r.below(300) as usize])));
    out
}

/// The model's sum and non-null count of column c.
fn model_total(model: &BTreeMap<u64, Vec<(u32, V)>>, c: usize) -> (Big, u64) {
    let (mut sum, mut n) = (Big::default(), 0u64);
    for row in model.values() {
        let v = match &row[c].1 {
            V::Null => continue,
            V::U(x) => Big::from_i128(i128::from(*x)),
            V::I(x) => Big::from_i128(i128::from(*x)),
            V::Nat(b) => Big::from_leb(b, false),
            V::Int(b) => Big::from_leb(b, true),
            V::U128(x) => Big::from_u128(*x),
            V::I128(x) => Big::from_i128(*x),
            other => panic!("column {c}: {other:?}"),
        };
        sum = sum.add(&v);
        n += 1;
    }
    (sum, n)
}

/// Every integer column's sum and count against the model; the Float and Text columns have none. Returns
/// the number of totals that lay in the hot state and in the totals space, and the longest sum in bytes.
fn check(tb: &Table<'_, VecHost>, s: &Schema, model: &BTreeMap<u64, Vec<(u32, V)>>) -> (u64, u64, usize) {
    let (mut hot, mut space, mut longest) = (0u64, 0u64, 0usize);
    let h = tb.host();
    let tail = root::u64_at(h, tb.root(), root::TAIL).unwrap();
    let mut rc = RunCursor::open(h, tail).unwrap();
    let mut buf = vec![0u8; 1 << 17];
    for (c, &(t, _)) in s.cols.iter().enumerate() {
        let l = tb.shape().leaf(h, c as u64).unwrap();
        let at = rc.place(h.get(Region::Block(tail)).unwrap(), &l).unwrap();
        if matches!(t, Ty::F64 | Ty::Text) {
            assert!(!matches!(at, At::Space(_)));
            assert_eq!(tb.sum(id(c as u32), &mut buf), Err(Fault::BadRef), "column {c}: no total");
            continue;
        }
        match at {
            At::Hot(_) => hot += 1,
            At::Space(_) => space += 1,
            At::None => panic!("column {c} has no total"),
        }
        let (want, n) = model_total(model, c);
        let (got, gn) = tb.sum(id(c as u32), &mut buf).unwrap();
        let got = Big::of_total(got, &buf);
        assert_eq!((&got, gn), (&want, n), "column {c} ({t:?}): sum and count");
        longest = longest.max(got.0.len());
        // A big sum needs a buffer of its length.
        if got.0.len() > 16 {
            let short = got.0.len() - 1;
            assert!(matches!(tb.sum(id(c as u32), &mut buf[..short]), Err(Fault::Scratch { .. })));
        }
    }
    (hot, space, longest)
}

/// Random writes against the model, checked every few hundred steps.
fn run(s: &Schema, steps: u64, seed: u64, h: &mut VecHost, lh: &mut LogHash) -> Vec<(u64, u64, u64, usize)> {
    let (mut tb, _) = Table::create(h, 31, &s.img).unwrap();
    let mut r = XorShift(seed);
    let mut model: BTreeMap<u64, Vec<(u32, V)>> = BTreeMap::new();
    let mut trace = Vec::new();
    let mut sealed_updates = 0u64;
    let pick = |r: &mut XorShift, model: &BTreeMap<u64, Vec<(u32, V)>>| {
        *model.keys().nth(r.below(model.len() as u64) as usize).unwrap()
    };
    for step in 0..steps {
        let op = r.below(100);
        if op < 70 {
            let k = if model.is_empty() || r.below(4) != 0 { r.below(1 << 40) } else { pick(&mut r, &model) };
            let mut new = row(s, &mut r, k);
            // Some updates change one integer column alone.
            if let (Some(old), true) = (model.get(&k), r.below(3) == 0) {
                let c = 1 + r.below(s.cols.len() as u64 - 3) as usize;
                let mut one = old.clone();
                one[c] = new[c].clone();
                new = one;
            }
            let sealed = model.contains_key(&k) && {
                let rid = rid_of(&tb, &[(id(0), V::U(k))]).unwrap();
                matches!(read::find(tb.host(), tb.root(), rid).unwrap(), Some(read::Row::Sealed { .. }))
            };
            match tb.upsert(&mut key_scratch(), &mut Src::new(&new)) {
                Ok((Upsert::Inserted(_), _)) => assert!(model.insert(k, new).is_none()),
                Ok((Upsert::Replaced(_), _)) => {
                    sealed_updates += u64::from(sealed);
                    assert!(model.insert(k, new).is_some());
                }
                Err(Fault::Busy) => {
                    // A refused update of a sealed row: removed and inserted again.
                    assert!(sealed, "only a sealed row's update is refused");
                    sealed_updates += 1;
                    assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
                    assert!(matches!(
                        tb.upsert(&mut key_scratch(), &mut Src::new(&new)).unwrap().0,
                        Upsert::Inserted(_)
                    ));
                    model.insert(k, new);
                }
                Err(e) => panic!("upsert: {e:?}"),
            }
        } else if op < 92 {
            let k = if model.is_empty() || r.below(5) == 0 { r.below(1 << 40) } else { pick(&mut r, &model) };
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone.is_some(), model.remove(&k).is_some());
        } else {
            if r.below(2) == 0 {
                tb.write_step(&TestBudget::new(u64::MAX)).unwrap();
            } else {
                maintain_all(&mut tb);
            }
        }
        if step % 500 == 499 {
            let (hot, space, longest) = check(&tb, s, &model);
            lh.take(tb.host_mut());
            trace.push((step, hot, space, longest));
        }
    }
    maintain_all(&mut tb);
    let (hot, space, longest) = check(&tb, s, &model);
    // Every row removed: every total back at zero, its big-form sum included.
    for k in model.keys().copied().collect::<Vec<_>>() {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
        model.remove(&k);
    }
    assert_eq!(check(&tb, s, &BTreeMap::new()).2, 0);
    let seals = open_group(tb.host(), tb.root());
    assert!(seals >= 2, "groups sealed: {seals}");
    assert!(sealed_updates > 0, "sealed rows updated");
    trace.push((seals, hot, space, longest));
    trace
}

#[test]
fn narrow_totals_in_the_hot_state() {
    let s = narrow();
    let trace = per_fill(|h, lh| run(&s, 12_000, 0x5EED_0001, h, lh));
    let last = trace.last().unwrap();
    assert_eq!((last.1, last.2), (10, 0), "every total in the hot state");
    assert!(trace.iter().any(|t| t.3 > 1_800), "a sum past an out-of-line value: {trace:?}");
    eprintln!("narrow: {trace:?}");
}

#[test]
fn wide_totals_in_the_hot_state_and_the_totals_space() {
    let s = wide();
    let trace = per_fill(|h, lh| run(&s, 10_000, 0x5EED_0002, h, lh));
    let last = trace.last().unwrap();
    assert!(last.1 > 0 && last.2 >= 5, "totals in both places: {last:?}");
    eprintln!("wide: {trace:?}");
}

/// Huge Nat and Int values (long values of their own) inserted, updated to other huge and small values,
/// deleted while buffered, then sealed with the group, and every row deleted: the totals follow at every step.
#[test]
fn huge_values() {
    let s = narrow();
    per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 33, &s.img).unwrap();
        let mut r = XorShift(0x5EED_0003);
        let mut model: BTreeMap<u64, Vec<(u32, V)>> = BTreeMap::new();
        let mut longest = Vec::new();
        let huge = |r: &mut XorShift, signed: bool| {
            let n = HUGE_VALUE as usize + 1 + r.below(3_000) as usize;
            let neg = signed && r.below(2) == 0;
            leb_of(r, n, signed, neg)
        };
        let write = |tb: &mut Table<'_, VecHost>, model: &mut BTreeMap<u64, Vec<(u32, V)>>, row: Vec<(u32, V)>| {
            tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
            let V::U(k) = row[0].1 else { panic!("a Nat key") };
            model.insert(k, row);
        };
        // Huge values in 6 rows, among small rows.
        for k in 0..40u64 {
            let mut w = row(&s, &mut r, k);
            if k % 7 == 3 {
                w[5].1 = V::Nat(huge(&mut r, false));
                w[6].1 = V::Int(huge(&mut r, true));
            }
            write(&mut tb, &mut model, w);
        }
        longest.push(check(&tb, &s, &model).2);
        // Updates: huge to huge, huge to small, small to huge.
        for k in [3u64, 10, 17] {
            let mut w = model[&k].clone();
            w[5].1 = if k == 10 { V::Nat(uleb(5)) } else { V::Nat(huge(&mut r, false)) };
            w[6].1 = V::Int(huge(&mut r, true));
            write(&mut tb, &mut model, w);
            longest.push(check(&tb, &s, &model).2);
        }
        // Deletes of buffered huge rows.
        for k in [24u64, 31] {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
            model.remove(&k);
            longest.push(check(&tb, &s, &model).2);
        }
        // Small rows until the insert that fills the group seals it: the totals stay.
        let mut k = 40u64;
        let g0 = open_group(tb.host(), tb.root());
        while open_group(tb.host(), tb.root()) == g0 {
            let w = row(&s, &mut r, k);
            write(&mut tb, &mut model, w);
            k += 1;
        }
        maintain_all(&mut tb);
        longest.push(check(&tb, &s, &model).2);
        // Deletes of sealed rows without huge values.
        for k in (40..k).step_by(97) {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
            model.remove(&k);
        }
        longest.push(check(&tb, &s, &model).2);
        // Every other row deleted: every total back to zero.
        let keys: Vec<u64> = model.keys().copied().collect();
        for k in keys {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
            model.remove(&k);
        }
        longest.push(check(&tb, &s, &model).2);
        maintain_all(&mut tb);
        lh.take(tb.host_mut());
        assert!(longest[0] > 57_000, "{longest:?}");
        assert_eq!(*longest.last().unwrap(), 0);
        longest
    });
}

/// A sum past i128 from many values within 64 bits and a few beyond, kept exact through deletes that
/// cancel the big part back to nothing.
#[test]
fn big_part_cancels() {
    let s = build(&[(Ty::Nat, false), (Ty::Int, false)]);
    per_fill(|h, _| {
        let (mut tb, _) = Table::create(h, 32, &s.img).unwrap();
        let mut buf = vec![0u8; 64];
        let big = uleb(u128::MAX);
        let neg = sleb(i128::MIN);
        let put = |tb: &mut Table<'_, VecHost>, k: u64, a: V, b: V| {
            let r = vec![(id(0), V::U(k)), (id(1), a), (id(2), b), (id(3), V::F64(0)), (id(4), V::Bytes(vec![]))];
            tb.upsert(&mut key_scratch(), &mut Src::new(&r)).unwrap();
        };
        for k in 0..4u64 {
            put(&mut tb, k, V::Nat(big.clone()), V::Int(neg.clone()));
        }
        let (t, n) = tb.sum(id(1), &mut buf).unwrap();
        assert_eq!(
            (Big::of_total(t, &buf), n),
            (
                Big::from_u128(u128::MAX)
                    .add(&Big::from_u128(u128::MAX))
                    .add(&Big::from_u128(u128::MAX))
                    .add(&Big::from_u128(u128::MAX)),
                4
            )
        );
        let (t, _) = tb.sum(id(2), &mut buf).unwrap();
        assert_eq!(
            Big::of_total(t, &buf),
            Big::from_i128(i128::MIN)
                .add(&Big::from_i128(i128::MIN))
                .add(&Big::from_i128(i128::MIN))
                .add(&Big::from_i128(i128::MIN))
        );
        for k in 0..4u64 {
            put(&mut tb, k, V::U(u64::MAX), V::I(i64::MIN));
        }
        let (t, n) = tb.sum(id(1), &mut buf).unwrap();
        assert_eq!((t, n), (Total::Small(4 * i128::from(u64::MAX)), 4));
        assert_eq!(tb.sum(id(2), &mut buf).unwrap(), (Total::Small(4 * i128::from(i64::MIN)), 4));
        let tail = root::u64_at(tb.host(), tb.root(), root::TAIL).unwrap();
        let mut rc = RunCursor::open(tb.host(), tail).unwrap();
        let hot = tb.host().get(Region::Block(tail)).unwrap();
        for o in 0..3 {
            let l = tb.shape().leaf(tb.host(), o).unwrap();
            if let At::Hot(at) = rc.place(hot, &l).unwrap() {
                let len = u32::from_le_bytes(hot[at as usize + 24..at as usize + 28].try_into().unwrap());
                assert_eq!(len, 0, "leaf {o}: the big-form sum is back at nothing");
            }
        }
    });
}

/// A sum reads the same few blocks at any table size: the root, the hot state and the schema page, with the
/// totals space when the leaf's total lies there; never a group or a row.
#[test]
fn a_sum_reads_the_same_at_any_size() {
    for s in [narrow(), wide()] {
        let calls = |rows: u64| {
            per_fill(|h, _| {
                let (mut tb, _) = Table::create(h, 34, &s.img).unwrap();
                let mut r = XorShift(rows);
                for k in 0..rows {
                    tb.upsert(&mut key_scratch(), &mut Src::new(&row(&s, &mut r, k))).unwrap();
                }
                maintain_all(&mut tb);
                let mut buf = vec![0u8; 1 << 16];
                (1..s.cols.len() as u32 - 2)
                    .map(|c| {
                        tb.host_mut().take_log();
                        tb.sum(id(c), &mut buf).unwrap();
                        tb.host_mut().take_log().len()
                    })
                    .collect::<Vec<_>>()
            })
        };
        let (few, many) = (calls(50), calls(9_000));
        assert_eq!(few, many, "host calls of one sum per column, 50 rows and 9,000 rows");
    }
}

/// Nat and Int values of 100,000 LEB128 bytes whose sums carry and borrow across their whole length (an
/// all-ones Nat and the largest positive Int of that length, then 1 and the most negative Int), moved in and
/// out of the totals by inserts, the update of a sealed row and the delete of a sealed row: each write takes
/// its values' big-form sums a piece at a time, so its host calls stay below one per 8 bytes of its values,
/// and the totals stay exact.
#[test]
fn big_values_move_their_totals_in_pieces() {
    const N: usize = 100_000;
    let s = narrow();
    let ones = || {
        let mut v = vec![0xFFu8; N];
        v[N - 1] = 0x7F;
        v
    };
    let pos = || {
        let mut v = vec![0xFFu8; N];
        v[N - 1] = 0x3F;
        v
    };
    let neg = || {
        let mut v = vec![0x80u8; N];
        v[0] = 0x81;
        v[N - 1] = 0x40;
        v
    };
    // 3 · 2^(7·(N − 1)): every group but the top one zero.
    let high = || {
        let mut v = vec![0x80u8; N];
        v[N - 1] = 0x03;
        v
    };
    let mut h = VecHost::new(FILLS[0]);
    let (mut tb, _) = Table::create(&mut h, 35, &s.img).unwrap();
    let mut r = XorShift(0x5EED_0005);
    let mut model: BTreeMap<u64, Vec<(u32, V)>> = BTreeMap::new();
    // The host calls of one write: its lent writes and its reads of the pages that were there before it.
    let measured = |tb: &mut Table<'_, VecHost>, w: &mut dyn FnMut(&mut Table<'_, VecHost>)| -> u64 {
        let h = tb.host_mut();
        h.take_log();
        h.watched = (1..=h.memory().len() as u64 / PAGE).map(|k| k * PAGE).collect();
        h.watched_reads.set(0);
        w(tb);
        let h = tb.host_mut();
        let n = h.take_log().len() as u64 + h.watched_reads.get();
        h.watched.clear();
        n
    };
    let mut most = 0u64;
    for k in 0..40u64 {
        let mut w = row(&s, &mut r, k);
        match k % 8 {
            1 => (w[5].1, w[6].1) = (V::Nat(ones()), V::Int(pos())),
            2 => (w[5].1, w[6].1) = (V::Nat(uleb(1)), V::Int(neg())),
            _ => {}
        }
        let big = k % 8 == 1;
        let n = measured(&mut tb, &mut |tb| {
            tb.upsert(&mut key_scratch(), &mut Src::new(&w)).unwrap();
        });
        if big {
            most = most.max(n);
        }
        model.insert(k, w);
    }
    check(&tb, &s, &model);
    let mut k = 40u64;
    let g0 = open_group(tb.host(), tb.root());
    while open_group(tb.host(), tb.root()) == g0 {
        let w = row(&s, &mut r, k);
        tb.upsert(&mut key_scratch(), &mut Src::new(&w)).unwrap();
        model.insert(k, w);
        k += 1;
    }
    maintain_all(&mut tb);
    check(&tb, &s, &model);
    // The update of a sealed row's big values to others, then the delete of a sealed row holding such values.
    let mut w = model[&9].clone();
    let mut other = pos();
    other[0] = 0xFE;
    (w[5].1, w[6].1) = (V::Nat(high()), V::Int(other));
    most = most.max(measured(&mut tb, &mut |tb| {
        tb.upsert(&mut key_scratch(), &mut Src::new(&w)).unwrap();
    }));
    model.insert(9, w);
    check(&tb, &s, &model);
    most = most.max(measured(&mut tb, &mut |tb| {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(17))])).unwrap().0.is_some());
    }));
    model.remove(&17);
    check(&tb, &s, &model);
    eprintln!("most host calls of one write: {most}");
    assert!(most < (2 * N / 8) as u64, "{most} host calls for one write of two {N}-byte values");
}
