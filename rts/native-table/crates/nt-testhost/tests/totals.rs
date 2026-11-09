//! A sum reads the same few blocks at any table size: the root, the hot state and the schema page, with the
//! totals space when the leaf's total lies there; never a group or a row.

use std::hash::{Hash, Hasher};

use native_table::table::layout::Ty;
use native_table::table::Table;
use native_table::value::ty;
use native_table::{Progress, Region, Wake};
use nt_testhost::rows::{schema, t, uleb, Src, V};
use nt_testhost::{Call, TestBudget, VecHost, VecScratch, XorShift, FILLS};

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
