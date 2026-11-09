//! The integer leaves' extremes in the cases a random mix rarely reaches: a schema wider than an extremes block
//! covers, and groups sealed after three rows each, so that the group directory is one entry, then one level,
//! then two levels. `Table::extreme` must equal the model's minimum and maximum of every integer leaf at every
//! check. Then each group's record is checked itself: exact after inserts alone, loose once a write takes its
//! extreme away, exact again once a value reaches it.
//!
//! Every test runs once per fill pattern (`FILLS`): the results and the host-call logs must be equal, so no
//! byte is read that was not written.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use native_table::consts::GROUP_SLOTS;
use native_table::fault::Fault;
use native_table::table::buffer::DESC_BIT;
use native_table::table::extremes::{self, Rec, CAP, HAS, MAX_LOOSE, MIN_LOOSE};
use native_table::table::layout::{LeafInfo, Ty};
use native_table::table::totals::Total;
use native_table::table::{read, root, write, Table, Upsert};
use native_table::value::ty;
use native_table::{Progress, Region, Wake};
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
/// past i128 and out of line (rarely, so that most groups keep their extremes in their records).
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
            let more = r.below(400) as usize;
            let leb = match r.below(1_000) {
                0..=799 => {
                    let x = r.next_u64() >> r.below(64);
                    if signed {
                        sleb(if neg { -i128::from(x) } else { i128::from(x) })
                    } else {
                        uleb(u128::from(x))
                    }
                }
                800..=979 => {
                    let x = (u128::from(r.next_u64()) << 63 | u128::from(r.next_u64())) >> r.below(60);
                    if signed {
                        sleb(if neg { -(x as i128) } else { x as i128 })
                    } else {
                        uleb(x)
                    }
                }
                980..=994 => leb_of(r, 19 + more % 30, signed, neg),
                _ => leb_of(r, 2_100 + more, signed, neg),
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
/// the last two are a Float and a Text, which have no extremes.
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

/// More integer leaves than an extremes block covers: the last ones' extremes are read from the rows.
fn wide() -> Schema {
    let mut cols: Vec<(Ty, bool)> =
        (0..104).map(|i| (if i % 2 == 0 { Ty::U(2) } else { Ty::I(4) }, i % 5 == 0)).collect();
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

/// An integer value as a `Big`.
fn big_of(v: &V) -> Option<Big> {
    match v {
        V::Null => None,
        V::U(x) => Some(Big::from_i128(i128::from(*x))),
        V::I(x) => Some(Big::from_i128(i128::from(*x))),
        V::Nat(b) => Some(Big::from_leb(b, false)),
        V::Int(b) => Some(Big::from_leb(b, true)),
        V::U128(x) => Some(Big::from_u128(*x)),
        V::I128(x) => Some(Big::from_i128(*x)),
        other => panic!("{other:?}"),
    }
}

/// The model's smallest (`largest`: largest) value of column c over the rows `rows` gives.
fn model_extreme<'a>(rows: impl Iterator<Item = &'a Vec<(u32, V)>>, c: usize, largest: bool) -> Option<Big> {
    let mut best: Option<Big> = None;
    for row in rows {
        let Some(v) = big_of(&row[c].1) else { continue };
        let better = match &best {
            None => true,
            Some(b) => v.order(b) == if largest { Ordering::Greater } else { Ordering::Less },
        };
        if better {
            best = Some(v);
        }
    }
    best
}

fn of_total(t: Option<Total>, buf: &[u8]) -> Option<Big> {
    t.map(|t| Big::of_total(t, buf))
}

/// Every integer column's minimum and maximum against the model; the Float and Text columns have none. Returns
/// how many of the groups' records gave their extremes, and how many were read from rows.
fn check(tb: &Table<'_, VecHost>, s: &Schema, model: &BTreeMap<u64, Vec<(u32, V)>>) -> (u64, u64) {
    let mut buf = vec![0u8; 1 << 13];
    for (c, &(t, _)) in s.cols.iter().enumerate() {
        if matches!(t, Ty::F64 | Ty::Text) {
            assert_eq!(tb.extreme(id(c as u32), false, &mut buf), Err(Fault::BadRef), "column {c}: no extremes");
            continue;
        }
        for largest in [false, true] {
            let want = model_extreme(model.values(), c, largest);
            let got = tb.extreme(id(c as u32), largest, &mut buf).unwrap();
            assert_eq!(of_total(got, &buf), want, "column {c} ({t:?}), largest {largest}");
            // A value past i128 needs a buffer of its length.
            if let Some(Total::Big(n)) = got {
                assert!(matches!(tb.extreme(id(c as u32), largest, &mut buf[..n - 1]), Err(Fault::Scratch { .. })));
            }
        }
    }
    let (mut kept, mut read) = (0u64, 0u64);
    for (_, blk) in groups(tb) {
        for c in 1..s.cols.len() - 2 {
            let l = leaf(tb, c);
            match (blk != 0).then(|| extremes::record(tb.host(), blk, &l).unwrap()).flatten() {
                Some(r) if r.flags & (MIN_LOOSE | MAX_LOOSE | extremes::BIG) == 0 => kept += 1,
                _ => read += 1,
            }
        }
    }
    (kept, read)
}

fn leaf(tb: &Table<'_, VecHost>, c: usize) -> LeafInfo {
    tb.shape().leaf(tb.host(), c as u64).unwrap()
}

/// Every group: its number and its extremes block.
fn groups(tb: &Table<'_, VecHost>) -> Vec<(u64, u64)> {
    let h = tb.host();
    let open = root::u64_at(h, tb.root(), root::OPEN_GROUP).unwrap();
    let mut out = Vec::new();
    for g in 0..=open {
        let e = root::group(h, tb.root(), g).unwrap();
        if e == 0 {
            continue;
        }
        let blk = if e & DESC_BIT != 0 {
            extremes::of_desc(h, e & !DESC_BIT).unwrap()
        } else {
            extremes::of_page(h, e).unwrap()
        };
        out.push((g, blk));
    }
    out
}

/// Random writes against the model, checked every few hundred steps.
fn run(s: &Schema, steps: u64, seed: u64, h: &mut VecHost, lh: &mut LogHash) -> Vec<(u64, u64, u64)> {
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
                Err(e) => panic!("upsert: {e:?}"),
            }
        } else if op < 92 {
            let k = if model.is_empty() || r.below(5) == 0 { r.below(1 << 40) } else { pick(&mut r, &model) };
            let (gone, _) = tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap();
            assert_eq!(gone.is_some(), model.remove(&k).is_some());
        } else {
            if r.below(2) == 0 {
                tb.write_step().unwrap();
            } else {
                maintain_all(&mut tb);
            }
        }
        if step % 500 == 499 {
            let (kept, read) = check(&tb, s, &model);
            lh.take(tb.host_mut());
            trace.push((step, kept, read));
        }
    }
    maintain_all(&mut tb);
    let (kept, read) = check(&tb, s, &model);
    // Every row removed: no extremes left.
    for k in model.keys().copied().collect::<Vec<_>>() {
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
        model.remove(&k);
    }
    check(&tb, s, &BTreeMap::new());
    maintain_all(&mut tb);
    check(&tb, s, &BTreeMap::new());
    let seals = open_group(tb.host(), tb.root());
    assert!(seals >= 2, "groups sealed: {seals}");
    assert!(sealed_updates > 0, "sealed rows updated");
    trace.push((seals, kept, read));
    trace
}

#[test]
fn wide_extremes() {
    let s = wide();
    assert!(s.cols.len() as u64 > CAP, "wider than a block covers");
    let trace = per_fill(|h, lh| run(&s, 8_000, 0x5EED_0012, h, lh));
    eprintln!("wide: {trace:?}");
}

/// The record of column c in each group, exact: the group's minimum and maximum over its live rows, from the
/// model by rid.
fn check_records_exact(tb: &Table<'_, VecHost>, model: &BTreeMap<u64, (u64, Vec<(u32, V)>)>, c: usize) -> usize {
    let l = leaf(tb, c);
    let mut n = 0;
    for (g, blk) in groups(tb) {
        assert_ne!(blk, 0, "group {g} has an extremes block");
        let rows = model.values().filter(|(rid, _)| rid / GROUP_SLOTS == g).map(|(_, r)| r);
        let lo = model_extreme(rows.clone(), c, false).map(|b| b.0);
        let hi = model_extreme(rows, c, true).map(|b| b.0);
        let r = extremes::record(tb.host(), blk, &l).unwrap().unwrap();
        assert_eq!(r.flags & (MIN_LOOSE | MAX_LOOSE | extremes::BIG), 0, "group {g}: exact");
        let rank = |x: u64| Big::from_i128(i128::from(x)).0;
        let got = (r.flags & HAS != 0).then(|| (rank(r.min as u64), rank(r.max as u64)));
        assert_eq!(got, lo.zip(hi), "group {g}");
        n += 1;
    }
    n
}

/// Inserts alone keep every record exact, through seals; a delete or an update that takes a
/// group's extreme away makes it loose (and the table's answer stays exact); a value that reaches it makes it
/// exact again.
#[test]
fn records_exact_and_loose() {
    let s = build(&[(Ty::U(8), false), (Ty::Nat, true)]);
    per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 34, &s.img).unwrap();
        let mut model: BTreeMap<u64, (u64, Vec<(u32, V)>)> = BTreeMap::new();
        let mut buf = [0u8; 64];
        let put = |tb: &mut Table<'_, VecHost>, model: &mut BTreeMap<u64, (u64, Vec<(u32, V)>)>, k: u64, a: u64| {
            let r = vec![
                (id(0), V::U(k)),
                (id(1), V::U(a)),
                (id(2), if k % 3 == 0 { V::Null } else { V::Nat(uleb(u128::from(a))) }),
                (id(3), V::F64(0)),
                (id(4), V::Bytes(vec![])),
            ];
            let (u, _) = tb.upsert(&mut key_scratch(), &mut Src::new(&r)).unwrap();
            let rid = match u {
                Upsert::Inserted(rid) | Upsert::Replaced(rid) => rid,
            };
            model.insert(k, (rid, r));
        };
        let answer = |tb: &Table<'_, VecHost>, c: u32, largest: bool, buf: &mut [u8]| match tb
            .extreme(id(c), largest, buf)
            .unwrap()
        {
            Some(Total::Small(x)) => Some(x),
            other => panic!("{other:?}"),
        };
        // Three groups and a half, a value per row from a permutation of the keys.
        let n = 3 * GROUP_SLOTS + GROUP_SLOTS / 2;
        for k in 0..n {
            put(&mut tb, &mut model, k, (k * 7_919) % 100_003 + 10);
        }
        maintain_all(&mut tb);
        assert_eq!(check_records_exact(&tb, &model, 1), 4);
        assert_eq!(check_records_exact(&tb, &model, 2), 4);
        let rows = || model.values().map(|(_, r)| r);
        let lo = |c| model_extreme(rows(), c, false);
        assert_eq!(answer(&tb, 1, false, &mut buf).map(|x| Big::from_i128(x)), lo(1));
        lh.take(tb.host_mut());

        // The sealed row holding group 1's minimum of column 1 deleted: loose there, and the answer exact.
        let in_group = |model: &BTreeMap<u64, (u64, Vec<(u32, V)>)>, g: u64| -> Vec<(u64, u64)> {
            model
                .iter()
                .filter(|(_, (rid, _))| rid / GROUP_SLOTS == g)
                .map(|(k, (_, r))| (*k, if let V::U(a) = r[1].1 { a } else { unreachable!() }))
                .collect()
        };
        let (k_min, a_min) = *in_group(&model, 1).iter().min_by_key(|(_, a)| *a).unwrap();
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k_min))])).unwrap().0.is_some());
        model.remove(&k_min);
        let blk1 = groups(&tb).iter().find(|(g, _)| *g == 1).unwrap().1;
        let rec = |tb: &Table<'_, VecHost>, blk: u64| -> Rec {
            extremes::record(tb.host(), blk, &leaf(tb, 1)).unwrap().unwrap()
        };
        assert_eq!(rec(&tb, blk1).flags & (MIN_LOOSE | MAX_LOOSE), MIN_LOOSE, "the minimum is loose");
        assert_eq!(rec(&tb, blk1).min, u128::from(a_min), "and still bounds the group");
        let want = |model: &BTreeMap<u64, (u64, Vec<(u32, V)>)>, c, largest| {
            model_extreme(model.values().map(|(_, r)| r), c, largest)
        };
        for largest in [false, true] {
            for c in [1u32, 2] {
                let got = tb.extreme(id(c), largest, &mut buf).unwrap();
                assert_eq!(of_total(got, &buf), want(&model, c as usize, largest));
            }
        }
        // A sealed row of group 1 updated to below the bound: exact again, at the new value.
        let (k2, _) = in_group(&model, 1)[5];
        put(&mut tb, &mut model, k2, 1);
        assert_eq!(rec(&tb, blk1).flags & (MIN_LOOSE | MAX_LOOSE), 0);
        assert_eq!(rec(&tb, blk1).min, 1);
        assert_eq!(answer(&tb, 1, false, &mut buf), Some(1));
        // The group's maximum updated away: loose; then a compaction moves the block to the group's next page.
        let (k3, _) = *in_group(&model, 1).iter().max_by_key(|(_, a)| *a).unwrap();
        put(&mut tb, &mut model, k3, 500);
        assert_eq!(rec(&tb, blk1).flags & (MIN_LOOSE | MAX_LOOSE), MAX_LOOSE);
        for largest in [false, true] {
            let got = tb.extreme(id(1), largest, &mut buf).unwrap();
            assert_eq!(of_total(got, &buf), want(&model, 1, largest));
        }
        maintain_all(&mut tb);
        lh.take(tb.host_mut());

        // The open group's rows: a delete of its maximum and an update past it.
        let g_open = root::u64_at(tb.host(), tb.root(), root::OPEN_GROUP).unwrap();
        let blk_open = groups(&tb).iter().find(|(g, _)| *g == g_open).unwrap().1;
        let (k4, a4) = *in_group(&model, g_open).iter().max_by_key(|(_, a)| *a).unwrap();
        assert_eq!(rec(&tb, blk_open).max, u128::from(a4));
        assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k4))])).unwrap().0.is_some());
        model.remove(&k4);
        assert_eq!(rec(&tb, blk_open).flags & MAX_LOOSE, MAX_LOOSE);
        let (k5, _) = in_group(&model, g_open)[0];
        put(&mut tb, &mut model, k5, 1_000_000);
        assert_eq!(rec(&tb, blk_open).flags & (MIN_LOOSE | MAX_LOOSE), 0);
        assert_eq!(answer(&tb, 1, true, &mut buf), Some(1_000_000));
        // Every row deleted, then the groups dropped: no answer, and the blocks go with the groups.
        for k in model.keys().copied().collect::<Vec<_>>() {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(k))])).unwrap().0.is_some());
        }
        model.clear();
        assert_eq!(tb.extreme(id(1), false, &mut buf).unwrap(), None);
        maintain_all(&mut tb);
        assert_eq!(tb.extreme(id(2), true, &mut buf).unwrap(), None);
        assert!(groups(&tb).len() <= 2, "the emptied sealed groups are dropped");
        lh.take(tb.host_mut());
    });
}

/// Many groups, each sealed after three rows (`write::freeze`, as a group whose bytes run out is), so that the
/// group directory is one entry (one group), then one level, then two levels (past 512 groups). Then random
/// inserts, updates and removals against the model, a row holding a column's minimum or maximum removed now and
/// then, more groups sealed, and values past i128 (an
/// Int of more than 19 bytes now and then) joining and leaving groups on both sides of 512. The first group's
/// values of the two Int columns are all past i128, negative in one and positive in the other, so that its
/// maximum of the first and its minimum of the second is one the later groups' records beat. `Table::extreme`
/// equals the model at every check, at every directory height.
#[test]
fn many_groups_and_directory_levels() {
    let s = build(&[(Ty::U(8), false), (Ty::Int, true), (Ty::I128, false), (Ty::Int, false)]);
    per_fill(|h, lh| {
        let (mut tb, _) = Table::create(h, 36, &s.img).unwrap();
        let mut r = XorShift(0x5EED_0014);
        let mut model: BTreeMap<u64, Vec<(u32, V)>> = BTreeMap::new();
        let height = |tb: &Table<'_, VecHost>| root::directory(tb.host(), tb.root()).unwrap().height;
        let remove = |tb: &mut Table<'_, VecHost>, key: u64| {
            assert!(tb.remove(&mut key_scratch(), &mut Src::new(&[(id(0), V::U(key))])).unwrap().0.is_some());
        };
        let freeze = |tb: &mut Table<'_, VecHost>| {
            let (shape, root) = (*tb.shape(), tb.root());
            write::freeze(tb.host_mut(), root, &shape).unwrap();
        };
        let mut heights = Vec::new();
        let mut k = 0u64;
        for g in 0..600u64 {
            for _ in 0..3 {
                let mut row = row(&s, &mut r, k);
                if g == 0 {
                    row[2].1 = V::Int(leb_of(&mut r, 20 + k as usize, true, true));
                    row[4].1 = V::Int(leb_of(&mut r, 20 + k as usize, true, false));
                }
                assert!(matches!(tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap().0, Upsert::Inserted(_)));
                model.insert(k, row);
                k += 1;
            }
            if matches!(g, 0 | 1 | 300 | 511 | 512 | 599) {
                check(&tb, &s, &model);
                heights.push(height(&tb));
                lh.take(tb.host_mut());
            }
            freeze(&mut tb);
            if g % 50 == 49 {
                maintain_all(&mut tb);
            }
        }
        assert_eq!(heights, [0, 1, 1, 1, 2, 2]);
        // A value past i128 of either sign joins the open group, past 512, and leaves again with its row.
        for neg in [true, false] {
            let mut row = row(&s, &mut r, k);
            row[2].1 = V::Int(leb_of(&mut r, 24, true, neg));
            tb.upsert(&mut key_scratch(), &mut Src::new(&row)).unwrap();
            model.insert(k, row);
            let mut buf = vec![0u8; 1 << 13];
            assert!(matches!(tb.extreme(id(2), !neg, &mut buf).unwrap(), Some(Total::Big(_))), "past i128");
            check(&tb, &s, &model);
            remove(&mut tb, k);
            model.remove(&k);
            check(&tb, &s, &model);
            k += 1;
        }
        let pick = |r: &mut XorShift, model: &BTreeMap<u64, Vec<(u32, V)>>| {
            *model.keys().nth(r.below(model.len() as u64) as usize).unwrap()
        };
        let (mut extremes_removed, mut sealed_updates) = (0u64, 0u64);
        for step in 0..1_500u64 {
            let op = r.below(100);
            if op < 45 {
                let key = if r.below(3) == 0 { pick(&mut r, &model) } else { k };
                k += u64::from(key == k);
                let new = row(&s, &mut r, key);
                let sealed = model.contains_key(&key) && {
                    let rid = rid_of(&tb, &[(id(0), V::U(key))]).unwrap();
                    matches!(read::find(tb.host(), tb.root(), rid).unwrap(), Some(read::Row::Sealed { .. }))
                };
                sealed_updates += u64::from(sealed);
                match tb.upsert(&mut key_scratch(), &mut Src::new(&new)) {
                    Ok(_) => {}
                    Err(e) => panic!("upsert: {e:?}"),
                }
                model.insert(key, new);
            } else if op < 70 {
                let key = pick(&mut r, &model);
                remove(&mut tb, key);
                model.remove(&key);
            } else if op < 85 {
                // A row holding a column's minimum or maximum removed.
                let c = 1 + r.below(4) as usize;
                let want = model_extreme(model.values(), c, r.below(2) == 0);
                let key = *model.iter().find(|(_, row)| big_of(&row[c].1) == want).unwrap().0;
                remove(&mut tb, key);
                model.remove(&key);
                extremes_removed += 1;
            } else if op < 97 {
                freeze(&mut tb);
            } else {
                maintain_all(&mut tb);
            }
            if step % 250 == 249 {
                check(&tb, &s, &model);
                lh.take(tb.host_mut());
            }
        }
        maintain_all(&mut tb);
        let (kept, read) = check(&tb, &s, &model);
        assert_eq!(height(&tb), 2);
        assert!(kept > 0 && read > 0, "records kept {kept}, read from rows {read}");
        assert!(extremes_removed > 100 && sealed_updates > 0, "{extremes_removed} {sealed_updates}");
        (kept, read)
    });
}
