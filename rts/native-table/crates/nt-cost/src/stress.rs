//! The maintenance units' adversarial scenarios, from 11 on: row types, values and writes that drive a unit
//! towards its bound's edges (`units` runs them as it runs the others).
//!
//! - 11: every flat leaf kind (scalars and their options, Unit, `[u8; 16]` and `[u8; 1016]`, a record): sealed;
//!   a quarter of the rows deleted, a quarter updated, a quarter set one leaf (compaction); six columns dropped
//!   and three added (column release); rows of the new version; a second group whose rows are all deleted
//!   (drop).
//! - 12: scenario 11, with two columns added and 64 rows inserted and 64 updated when the first seal reaches
//!   its write step.
//! - 13: mixed rows whose Text and Blob are 1 MiB on every 64th row: sealed; a third deleted and a third
//!   updated (compaction); the Text column dropped (column release); a second group whose rows are all deleted.
//! - 14: scenario 13 without the 1 MiB values, the table opened with a newer schema image (two columns added,
//!   every third dropped) while its first seal runs: no unit adopts it.
//! - 15: Nat and Int values of 80,000 and 70,000 LEB128 bytes on every 128th row, whose sums pass 65,535
//!   bytes: sealed; a third deleted and a third updated to other such values (compaction).
//! - 16: 120 Bool and u8 leaves: sealed; nine rows in ten deleted and a twentieth updated (compaction); four
//!   columns dropped (column release).
//! - 17, 18, 19: 170, 90 and 60 Text leaves, then Bool and u8 leaves: sealed; a third deleted and the rest
//!   updated (compaction); four columns dropped (column release).
//! - 20: Nat and Int values of 60,000 LEB128 bytes on every 97th row and 3,000 on every 5th: sealed while 64
//!   rows are updated to other big values after its snapshot; 1,100 rows deleted (compaction), 64 rows
//!   updated again after its snapshot.
//! - 21: narrow rows, a Nat key and two Nats: a group sealed.
//! - 22: Nat and Int values of 100,000 LEB128 bytes on two rows in 64 (an all-ones Nat and a positive Int, then
//!   1 and the negative Int) whose sums carry and borrow across their length: sealed while 8 rows are updated
//!   to such values after its snapshot; a third deleted and the rest updated (compaction) with 8 more such
//!   updates after its snapshot; the Int column dropped (column release).
//! - 23: scenario 22 with values of 150,000 bytes on one row in 64 whose low groups are all zero (a Nat of
//!   2^(7·149,999)·m and the Int its negation, for small m).
//! - 24: scenario 22 with values of 50,000 bytes on one row in 64 that differ only in their lowest group.
//! - 25, 26, 27: an Int column whose least value, on one row, is a negative Int of 3,000, 40,000 or 20,000
//!   LEB128 bytes whose lowest group that is not zero lies 2,040, 1,000 or 30 bytes in, beside short negative
//!   and 20- to 49-byte positive Ints: sealed; a third deleted and a fifth updated (compaction); the Text column dropped (column release).
//! - 28: 120 Int leaves, each a running record, each with one negative value of 6,000 bytes: two groups
//!   sealed.
//! - 29: Nat and Int values of 39,600 and 19,800 bytes on two rows, beside Text: sealed; a third deleted
//!   (compaction); the Text dropped (column release).
//! - 30: 300 optional Text columns, half dropped and as many Nat added before the first row: sealed; a third
//!   deleted and a third updated (compaction); the Text columns left dropped (column release).
//! - 31: Nat values of 8,000 bytes equal but in their lowest group and Text of 5,000 bytes sharing long
//!   prefixes, every 36th and 38th row: sealed, a third deleted and a seventh updated (compaction), the i64
//!   dropped (column release).
//! - 32: Nat and Int values of 18 lengths at the edges of the pieces they are read and added in (167 to
//!   524,288 LEB128 bytes), updated after a seal's and a compaction's snapshot: sealed; a third deleted and a
//!   third updated (compaction); the Int dropped (column release).
//! - 33: an index added on a Nat of 1,000 values (W 16) to a sealed group: its build.
//! - 34: an index added on a Text of 1,000 to 4,000 letters (W 1,016, a tree about ten nodes high) to a sealed
//!   group: its build.
//! - 35: scenario 34 with every 64th row's Text of 64 KiB to 96 KiB.
//! - 36: scenario 35's index, built, then removed: the drain frees its tree and its key log.
//! - 37: scenario 34 with every 16th row's Text one of four keys of 64 KiB to 96 KiB sharing their first
//!   60 KiB, two of them given a row by a write once the index is added: the build raises a key's count
//!   through a descent of its own when its insert passes such a row, hands back the second record of a
//!   repeated key, and lowers a raised count back when the row is removed between units (four times).
//! - 38: an index added on a Nat and an Int (W 1,016) to a sealed group, the LEB128 forms of one of them of
//!   1,000 to 4,000 bytes (keys taken whole), every 32nd row's of 100,000 to 140,000 bytes (keys written over
//!   units), negatives with none or half their low groups zero: its build.
//! - 39: scenario 35 with every byte of the Text `00` but about one in eight, a letter: escaped, its keys
//!   take up to twice their stored bytes.

use std::cell::RefCell;
use std::collections::HashMap;

use native_table::consts::{HUGE_VALUE, LONG_VALUE};
use native_table::index::secondary::{Secondary, AT_FLAGS, RAISED};
use native_table::schema::{node, IndexDecl, Leaf, LEAF_KEY, LEAF_NULLABLE};
use native_table::table::jobs::{Ran, Spent};
use native_table::table::{seal, CellIn, RowSource, Table, Upsert};
use native_table::value::ty;
use native_table::Fault;

use crate::units::{big_nat, frozen, key_scratch, Evo, Gen, Key, State, VecPages, C, STATE};

/// The first scenario of this module.
pub(crate) const FIRST: u32 = 11;

thread_local! {
    /// Scenario 37: each row's key by its row id.
    static RIDS: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

/// A row as cells by leaf id.
struct Cells(Vec<(u32, C)>);

impl RowSource for Cells {
    fn cell(&mut self, leaf: u32) -> Result<CellIn<'_>, Fault> {
        Ok(self.0.iter().find(|(l, _)| *l == leaf).map(|(_, c)| c).ok_or(Fault::BadValue { leaf })?.cell())
    }
    fn fill(&mut self, _leaf: u32, _at: u64, _dst: &mut [u8]) -> Result<(), Fault> {
        Err(Fault::BadRef)
    }
}

fn prefixed(k: u64, len: usize) -> Vec<u8> {
    crate::units::prefixed(k % 1_000_000_000_000, len)
}

fn mix(k: u64, salt: u64, id: u32) -> u64 {
    let mut x = k.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ u64::from(id);
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 32;
    x
}

/// Every `huge_every`th row's Text and Blob are `huge_len` bytes sharing all but their last 12 (0: none).
#[derive(Clone, Copy)]
struct Knobs {
    huge_every: u64,
    huge_len: usize,
}

fn knobs(s: u32) -> Knobs {
    match s {
        13 => Knobs { huge_every: 64, huge_len: 1 << 20 },
        _ => Knobs { huge_every: 0, huge_len: 0 },
    }
}

/// A generated cell for leaf l of row k.
fn gen(l: &Leaf, k: u64, salt: u64, kn: Knobs) -> C {
    let x = mix(k, salt, l.id);
    if l.flags & LEAF_KEY != 0 {
        return C::U(k);
    }
    if l.flags & LEAF_NULLABLE != 0 && x % 5 == 0 {
        return C::Null;
    }
    if l.kind == node::UNIT {
        return C::Unit;
    }
    let ext = x % 23 == 0;
    match l.code {
        ty::BOOL => C::B(x % 2 == 0),
        ty::U8 => C::U(if ext { 255 } else { x % 256 }),
        ty::U16 => C::U(if ext { 65_535 } else { x % 65_536 }),
        ty::U32 => C::U(if ext { u64::from(u32::MAX) } else { x % (1 << 32) }),
        ty::U64 => C::U(if ext { u64::MAX } else { x }),
        ty::I8 => C::I(if ext { -128 } else { (x % 256) as i64 - 128 }),
        ty::I16 => C::I(if ext { -32_768 } else { (x % 65_536) as i64 - 32_768 }),
        ty::I32 => C::I(if ext { i64::from(i32::MIN) } else { (x % (1 << 32)) as i64 - (1 << 31) }),
        ty::I64 => C::I(if ext { i64::MIN } else { x as i64 }),
        ty::NAT => {
            if x % 7 == 0 {
                C::Nat(big_nat(x, 20 + (x % 30) as usize))
            } else if x % 211 == 0 {
                C::Nat(big_nat(x, 3_000))
            } else {
                C::U(x % 1_000_000)
            }
        }
        ty::INT => {
            if x % 7 == 0 {
                let n = 20 + (x % 30) as usize;
                let mut v = vec![0x80u8; n];
                v[n - 1] = 0x40;
                C::Int(v)
            } else if x % 11 == 0 {
                let n = 20 + (x % 30) as usize;
                let mut v = vec![0xFFu8; n];
                v[n - 1] = 0x3F;
                C::Int(v)
            } else {
                C::I(x as i64 >> (x % 40))
            }
        }
        ty::F32 => C::F32(match x % 13 {
            0 => -0.0,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => f32::MIN_POSITIVE,
            _ => (x % 100_000) as f32 / 7.0 - 5_000.0,
        }),
        ty::F64 => C::F(match x % 13 {
            0 => -0.0,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => f64::MAX,
            _ => (x % 1_000_000) as f64 / 7.0 - 50_000.0,
        }),
        ty::CHAR => C::U(match x % 3 {
            0 => 0x41 + x % 26,
            1 => 0x10FFFF,
            _ => 0x1F600 + x % 64,
        }),
        ty::TEXT | ty::BLOB => {
            if kn.huge_every != 0 && k % kn.huge_every == 5 {
                C::Bytes(prefixed(k.wrapping_mul(3).wrapping_add(salt).wrapping_add(u64::from(l.id)), kn.huge_len))
            } else if x % 97 == 3 {
                C::Bytes(vec![b'm'; LONG_VALUE as usize + 1 + (x % 300) as usize])
            } else if x % 1_009 == 7 {
                C::Bytes(prefixed(x, HUGE_VALUE as usize + 100))
            } else {
                C::Bytes(format!("v{k}.{salt}.{}", x % 1_000).into_bytes())
            }
        }
        ty::PRINCIPAL => C::Bytes((0..(x % 30)).map(|i| (x >> (i % 60)) as u8).collect()),
        ty::U128 => C::U128(if ext { u128::MAX } else { u128::from(x) << 40 }),
        ty::I128 => C::I128(if ext { i128::MIN } else { (i128::from(x) << 50) - (1 << 100) }),
        ty::BYTES => C::Bytes((0..l.width).map(|i| (x >> (i % 56)) as u8).collect()),
        _ => C::Null,
    }
}

fn row_of(e: &Evo, k: u64, salt: u64, kn: Knobs) -> Cells {
    Cells(e.leaves.iter().filter(|l| l.dropped == 0).map(|l| (l.id, gen(l, k, salt, kn))).collect())
}

fn opt(t: &[u8]) -> Vec<u8> {
    let mut v = vec![ty::OPTION];
    v.extend(t);
    v
}

fn bytes_t(n: u16) -> Vec<u8> {
    let mut v = vec![ty::BYTES];
    v.extend(n.to_le_bytes());
    v
}

/// The Candid idl hash of a field name: h = h · 223 + b mod 2^32 over its UTF-8 bytes.
fn idl_hash(name: &[u8]) -> u32 {
    name.iter().fold(0u32, |h, &b| h.wrapping_mul(223).wrapping_add(u32::from(b)))
}

fn record(fields: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut v = vec![ty::RECORD];
    v.extend((fields.len() as u16).to_le_bytes());
    v.extend(0u16.to_le_bytes());
    let mut fs: Vec<(u32, &Vec<u8>)> = fields.iter().map(|(n, f)| (idl_hash(n.as_bytes()), f)).collect();
    fs.sort_by_key(|x| x.0);
    for (h, f) in fs {
        v.extend(h.to_le_bytes());
        v.extend(0u32.to_le_bytes());
        v.push(0);
        v.extend([0u8; 8]);
        v.extend(f);
    }
    v
}

fn mixed_evo(e: &mut Evo) {
    e.add(&[ty::NAT]);
    e.add(&[ty::TEXT]);
    e.add(&[ty::BOOL]);
    e.add(&[ty::I64]);
    e.add(&opt(&[ty::U8]));
    e.add(&[ty::U8]);
    e.add(&[ty::F64]);
    e.add(&[ty::BLOB]);
}

fn all_kinds_evo(e: &mut Evo) {
    e.add(&[ty::NAT]);
    for c in [
        ty::BOOL,
        ty::U8,
        ty::U16,
        ty::U32,
        ty::U64,
        ty::I8,
        ty::I16,
        ty::I32,
        ty::I64,
        ty::NAT,
        ty::INT,
        ty::F32,
        ty::F64,
        ty::CHAR,
        ty::TEXT,
        ty::BLOB,
        ty::PRINCIPAL,
        ty::U128,
        ty::I128,
    ] {
        e.add(&[c]);
        e.add(&opt(&[c]));
    }
    e.add(&bytes_t(16));
    e.add(&bytes_t(1_016));
    e.add(&opt(&bytes_t(16)));
    e.add(&[ty::UNIT]);
    e.add(&record(&[("a", vec![ty::U32]), ("b", vec![ty::TEXT]), ("c", opt(&[ty::F64]))]));
}

/// Scenarios 17, 18 and 19's Text leaves.
fn texts(s: u32) -> usize {
    match s {
        18 => 90,
        19 => 60,
        _ => 170,
    }
}

/// Scenarios 22 to 24: the big values' LEB128 length.
fn big_len(s: u32) -> usize {
    match s {
        22 => 100_000,
        23 => 150_000,
        _ => 50_000,
    }
}

/// A Nat of `len` LEB128 groups, all ones: 2^(7·len) − 1.
fn ones_nat(len: usize) -> Vec<u8> {
    let mut v = vec![0xFFu8; len];
    v[len - 1] = 0x7F;
    v
}

/// An Int of `len` groups, 2^(7·len − 1) − 1, and its negation.
fn pos_int(len: usize) -> Vec<u8> {
    let mut v = vec![0xFFu8; len];
    v[len - 1] = 0x3F;
    v
}

fn neg_int(len: usize) -> Vec<u8> {
    let mut v = vec![0x80u8; len];
    v[0] = 0x81;
    v[len - 1] = 0x40;
    v
}

/// Scenarios 22 to 24's row k.
fn carry_row(s: u32, k: u64, salt: u64) -> Cells {
    let n = big_len(s);
    let x = (k + salt) % 64;
    let (nat, int) = match (s, x) {
        (22, 0) => (C::Nat(ones_nat(n)), C::Int(pos_int(n))),
        (22, 1) => (C::U(1), C::Int(neg_int(n))),
        (22, 2) => (C::U(0), C::I(-1)),
        (22, _) => (C::U(3), C::I(1)),
        (23, 1) => {
            let mut a = vec![0x80u8; n];
            a[n - 1] = 0x01 + ((k + salt) % 60) as u8;
            let mut b = vec![0x80u8; n];
            b[n - 1] = 0x7F - ((k + salt) % 60) as u8;
            (C::Nat(a), C::Int(b))
        }
        (24, 1) => (C::Nat(big_nat(k + salt, n)), C::Int(big_int(k + salt, n))),
        _ => (C::U(k + salt), C::I(k as i64 - salt as i64)),
    };
    let id = |c: u32| c << 16;
    Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int), (id(3), C::I(k as i64))])
}

/// Builds scenario s's table and leaves its first maintenance pending; returns its rows.
pub(crate) fn setup(s: u32) -> u64 {
    RIDS.with(|r| r.borrow_mut().clear());
    let host: &'static mut VecPages = Box::leak(Box::new(VecPages { mem: Vec::new(), free: Vec::new() }));
    let mut e = Evo::new();
    match s {
        11 | 12 => all_kinds_evo(&mut e),
        13 | 14 => mixed_evo(&mut e),
        15 | 20 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::INT]);
            e.add(&if s == 15 { opt(&[ty::NAT]) } else { vec![ty::I64] });
        }
        16 => {
            e.add(&[ty::NAT]);
            for i in 1..=120u32 {
                e.add(if i % 2 == 0 { &[ty::BOOL] } else { &[ty::U8] });
            }
        }
        17..=19 => {
            e.add(&[ty::NAT]);
            for _ in 0..texts(s) {
                e.add(&[ty::TEXT]);
            }
            e.add(&[ty::BOOL]);
            e.add(&[ty::U8]);
            e.add(&[ty::BOOL]);
            e.add(&[ty::U8]);
        }
        22..=24 | 32 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::INT]);
            e.add(&[ty::I64]);
        }
        25..=27 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::INT]);
            e.add(&[ty::I64]);
            e.add(&[ty::TEXT]);
        }
        28 => {
            e.add(&[ty::NAT]);
            for _ in 0..120 {
                e.add(&[ty::INT]);
            }
        }
        29 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::INT]);
            e.add(&[ty::TEXT]);
        }
        30 => {
            e.add(&[ty::NAT]);
            for _ in 0..300 {
                e.add(&opt(&[ty::TEXT]));
            }
        }
        31 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::TEXT]);
            e.add(&[ty::I64]);
        }
        33..=37 | 39 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::TEXT]);
        }
        38 => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::INT]);
        }
        _ => {
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
            e.add(&[ty::NAT]);
        }
    }
    let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
    let (mut t, _) = Table::create(host, 7, img).unwrap();
    if s == 30 {
        // A schema change before any row: half the Text columns dropped, as many Nat added.
        e.version += 1;
        for c in 1..=150 {
            e.drop_col(c);
        }
        for _ in 0..150 {
            e.add(&[ty::NAT]);
        }
        let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
        t.change_schema(img).unwrap();
    }
    let kn = knobs(s);
    let mut k = 0u64;
    while frozen(&t) == 0 {
        insert(&mut t, &e, s, k, 0, kn);
        k += 1;
    }
    STATE.with(|st| {
        *st.borrow_mut() = Some(State {
            table: t,
            scenario: s,
            phase: 0,
            rows: k,
            next_key: k,
            evo: Some(e),
            units: 0,
            gen: Gen(0x5DEE_CE66_D1CE_4E5B ^ u64::from(s)),
            last: (Ran::Nothing, 0, Spent::default()),
            done: 0,
        })
    });
    k
}

/// A negative Int of `len` LEB128 bytes whose lowest groups are 0 but its first's low six bits, `k`'s.
fn big_int(k: u64, len: usize) -> Vec<u8> {
    let mut v = vec![0x80u8; len];
    v[0] = 0x80 | (k % 64) as u8;
    v[len - 1] = 0x40;
    v
}

/// Scenario 15's row: every 128th row's Nat and Int of 80,000 LEB128 bytes, the next's of 70,000.
fn wide_nat_row(k: u64, salt: u64) -> Cells {
    let len = match k % 128 {
        3 => 80_000,
        4 => 70_000,
        _ => 0,
    };
    let nat = if len > 0 { C::Nat(big_nat(k + salt, len)) } else { C::U(k * 1_000 + salt) };
    let int = if len > 0 { C::Int(big_int(k + salt, len)) } else { C::I(-(k as i64) - salt as i64) };
    let o = if k % 3 == 0 { C::Null } else { C::Nat(big_nat(k * 3 + salt, 40)) };
    let id = |c: u32| c << 16;
    Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int), (id(3), o)])
}

/// Scenario 20's row: every 97th row's Nat and Int of 60,000 LEB128 bytes, every 5th's of 3,000.
fn big_row(k: u64, salt: u64) -> Cells {
    let len = if k % 97 == 0 {
        60_000
    } else if k % 5 == 0 {
        3_000
    } else {
        0
    };
    let nat = if len > 0 { C::Nat(big_nat(k + salt, len)) } else { C::U(k * 7 + salt) };
    let int = if len > 0 { C::Int(big_int(k + salt, len)) } else { C::I(-(k as i64) - salt as i64) };
    let id = |c: u32| c << 16;
    Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int), (id(3), C::I(k as i64))])
}

/// Scenarios 25 to 27's variant (the lengths, the low group's place and the rows' pattern).
fn deep_variant(s: u32) -> u64 {
    match s {
        25 => 1,
        26 => 6,
        _ => 7,
    }
}

/// Scenarios 25 to 27's row k: the Int of the long negative minimum on row 5.
fn deep_low_row(s: u32, k: u64, salt: u64) -> Cells {
    let v = deep_variant(s);
    let (n, g) = match v % 4 {
        1 => (3_000usize, 2_040usize),
        2 => (40_000, 1_000),
        _ => (20_000, 30),
    };
    let every = 3 + v % 7;
    let int = if k == 5 {
        // Groups 0..g zero, then the value's top: −(2^(7g)·m).
        let mut b = vec![0x80u8; n];
        b[g] = 0x80 | (1 + (salt % 60) as u8);
        b[n - 1] = 0x40;
        C::Int(b)
    } else if k % every == 0 {
        let m = 20 + (k % 30) as usize;
        let mut b = vec![0xFFu8; m];
        b[m - 1] = 0x3F;
        C::Int(b)
    } else {
        C::I(-((k * 31 + salt) as i64 % 100_000))
    };
    let txt = C::Bytes(vec![b'a' + (k % 26) as u8; ((k + v) % 40) as usize]);
    let id = |c: u32| c << 16;
    Cells(vec![(id(0), C::U(k)), (id(1), int), (id(2), C::I(k as i64)), (id(3), txt)])
}

/// Scenario 32's lengths, one row each (rows 10, 60, 110, ...).
const EDGE_LENS: [usize; 18] = [
    167, 168, 169, 2_047, 2_048, 2_049, 4_095, 4_096, 4_097, 65_527, 65_528, 65_529, 74_880, 74_890, 74_900, 149_800,
    300_001, 524_288,
];

fn edge_row(k: u64, salt: u64) -> Cells {
    let idx = if k >= 10 && (k - 10) % 50 == 0 { ((k - 10) / 50) as usize } else { usize::MAX };
    let (nat, int) = if idx < EDGE_LENS.len() {
        let n = EDGE_LENS[idx] + (salt % 2) as usize;
        let int = if idx % 2 == 0 {
            C::Int(big_int(k + salt, n))
        } else {
            let mut b = vec![0x80u8; n];
            b[n - 1] = 0x7F - ((k + salt) % 60) as u8;
            C::Int(b)
        };
        (C::Nat(big_nat(k + salt, n)), int)
    } else {
        (C::U(k + salt), C::I(k as i64 - salt as i64))
    };
    let id = |c: u32| c << 16;
    Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int), (id(3), C::I(k as i64))])
}

fn insert(t: &mut Table<'static, VecPages>, e: &Evo, s: u32, k: u64, salt: u64, kn: Knobs) {
    let id = |c: u32| c << 16;
    match s {
        25..=27 => {
            t.upsert(&mut key_scratch(), &mut deep_low_row(s, k, salt)).unwrap();
        }
        28 => {
            let mut cells = Vec::with_capacity(e.leaves.len());
            for l in e.leaves.iter().filter(|l| l.dropped == 0) {
                let c = u64::from(l.id >> 16);
                let v = if l.flags & LEAF_KEY != 0 {
                    C::U(k)
                } else if k == 3 + 7 * c {
                    C::Int(big_int(k + salt, 6_000))
                } else {
                    C::I(k as i64 - c as i64 - salt as i64)
                };
                cells.push((l.id, v));
            }
            t.upsert(&mut key_scratch(), &mut Cells(cells)).unwrap();
        }
        29 => {
            let (nat, int) = if k == 7 || k == 3_001 {
                (C::Nat(big_nat(k + salt, 39_600)), C::Int(big_int(k + salt, 19_800)))
            } else {
                (C::U(k * 13 + salt), C::I(k as i64 - salt as i64))
            };
            let txt = C::Bytes(vec![b'a' + (k % 26) as u8; ((k * 7 + 12) % 48) as usize]);
            t.upsert(&mut key_scratch(), &mut Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int), (id(3), txt)]))
                .unwrap();
        }
        31 => {
            let nat = if k % 36 == 3 {
                // All ones but the lowest group: every two such values differ in their lowest group alone.
                let mut b = vec![0xFFu8; 8_000];
                b[0] = 0x80 | ((k + salt) % 128) as u8;
                b[7_999] = 0x7F;
                C::Nat(b)
            } else {
                C::U(k * 7 + salt)
            };
            let txt = if k % 38 == 4 {
                C::Bytes(prefixed(k * 3 + salt, 5_000))
            } else {
                C::Bytes(format!("t{}.{salt}", k % 977).into_bytes())
            };
            t.upsert(
                &mut key_scratch(),
                &mut Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), txt), (id(3), C::I(k as i64))]),
            )
            .unwrap();
        }
        32 => {
            t.upsert(&mut key_scratch(), &mut edge_row(k, salt)).unwrap();
        }
        33..=37 | 39 => {
            let x = mix(k, salt, 1);
            let mut g = Gen(x | 1);
            let len = if s >= 35 && k % 64 == 0 { 65_536 + x % 32_768 } else { 1_000 + x % 3_000 };
            let txt: Vec<u8> = if s == 37 && k % 16 == 0 {
                pool_key((k / 16) % 4)
            } else if s == 39 {
                (0..len).map(|_| if g.next() % 8 == 0 { b'a' + (g.next() % 26) as u8 } else { 0 }).collect()
            } else {
                (0..len).map(|_| b'a' + (g.next() % 26) as u8).collect()
            };
            let mut r = Cells(vec![(id(0), C::U(k)), (id(1), C::U(x % 1_000)), (id(2), C::Bytes(txt))]);
            if let (Upsert::Inserted(rid), _) = t.upsert(&mut key_scratch(), &mut r).unwrap() {
                RIDS.with(|m| m.borrow_mut().insert(rid, k));
            }
        }
        38 => {
            let x = mix(k, salt, 1);
            let mut g = Gen(x | 1);
            let huge = |g: &mut Gen| 100_000 + (g.next() % 40_000) as usize;
            let few = |g: &mut Gen| 1_000 + (g.next() % 3_000) as usize;
            let (nat, int) = match k % 32 {
                0 => {
                    let n = huge(&mut g);
                    (C::Nat(rand_leb(&mut g, n, false, false, 0)), C::I(k as i64))
                }
                16 => {
                    let n = huge(&mut g);
                    (
                        C::U(x % 1_000),
                        C::Int(rand_leb(&mut g, n, true, k % 64 == 16, if k % 128 == 16 { n / 2 } else { 0 })),
                    )
                }
                j if j % 2 == 0 => {
                    let n = few(&mut g);
                    (C::Nat(rand_leb(&mut g, n, false, false, 0)), C::I(-(k as i64)))
                }
                j => {
                    let n = few(&mut g);
                    (C::U(x % 1_000), C::Int(rand_leb(&mut g, n, true, j % 4 == 1, if j % 8 == 1 { n / 2 } else { 0 })))
                }
            };
            let mut r = Cells(vec![(id(0), C::U(k)), (id(1), nat), (id(2), int)]);
            t.upsert(&mut key_scratch(), &mut r).unwrap();
        }
        15 => {
            t.upsert(&mut key_scratch(), &mut wide_nat_row(k, salt)).unwrap();
        }
        20 => {
            t.upsert(&mut key_scratch(), &mut big_row(k, salt)).unwrap();
        }
        22..=24 => {
            t.upsert(&mut key_scratch(), &mut carry_row(s, k, salt)).unwrap();
        }
        21 => {
            let id = |c: u32| c << 16;
            let x = mix(k, salt, 1);
            let mut r = Cells(vec![(id(0), C::U(k)), (id(1), C::U(x % 1_000_000)), (id(2), C::U(x >> 44))]);
            t.upsert(&mut key_scratch(), &mut r).unwrap();
        }
        _ => {
            t.upsert(&mut key_scratch(), &mut row_of(e, k, salt, kn)).unwrap();
        }
    }
}

/// Scenario 37's long key `i` (of four): 64 KiB to 96 KiB of letters, the first 60 KiB shared.
fn pool_key(i: u64) -> Vec<u8> {
    let mut g = Gen(0x59_9001);
    let mut v: Vec<u8> = (0..61_440).map(|_| b'a' + (g.next() % 26) as u8).collect();
    let mut g = Gen(0x59_9002 + i);
    let n = 4_096 + g.next() % 32_768;
    v.extend((0..n).map(|_| b'a' + (g.next() % 26) as u8));
    v
}

/// A canonical (signed) LEB128 form of `len` bytes of random groups; a signed one negative when `neg`, its
/// lowest `zeros` groups zero.
fn rand_leb(g: &mut Gen, len: usize, signed: bool, neg: bool, zeros: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| 0x80 | (g.next() % 128) as u8).collect();
    for b in v.iter_mut().take(zeros.min(len - 1)) {
        *b = 0x80;
    }
    let r = (g.next() % 128) as u8;
    v[len - 1] = match (signed, neg) {
        (false, _) => r | 1,
        (true, false) => (r & 0x3F) | 1,
        (true, true) => 0x40 | (r & 0x3E),
    };
    v
}

fn remove(t: &mut Table<'static, VecPages>, k: u64) {
    t.remove(&mut key_scratch(), &mut Key(k)).unwrap();
}

/// Fills a new group until it freezes; returns its first key and the next.
fn fill_group(st: &mut State, salt: u64) -> (u64, u64) {
    let s = st.scenario;
    let e = st.evo.take().unwrap();
    let first = st.next_key;
    let mut k = first;
    while frozen(&st.table) == 0 {
        insert(&mut st.table, &e, s, k, salt, knobs(s));
        k += 1;
    }
    st.next_key = k;
    st.evo = Some(e);
    (first, k)
}

/// A new schema version: the given columns dropped.
fn drop_cols(st: &mut State, cols: &[u32]) {
    let mut e = st.evo.take().unwrap();
    e.version += 1;
    for &c in cols {
        if (c as usize) < e.cols.len() {
            e.drop_col(c);
        }
    }
    let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
    st.table.change_schema(img).unwrap();
    st.evo = Some(e);
}

/// The next phase's writes, when the pending maintenance is done: `true` when a phase began.
pub(crate) fn next_phase(st: &mut State) -> bool {
    st.phase += 1;
    let s = st.scenario;
    let n = st.rows;
    let kn = knobs(s);
    match (s, st.phase) {
        (11 | 12, 1) => {
            let e = st.evo.take().unwrap();
            let ids: Vec<u32> =
                e.leaves.iter().filter(|l| l.dropped == 0 && l.flags & LEAF_KEY == 0).map(|l| l.id).collect();
            for k in 0..n {
                match k % 4 {
                    0 => remove(&mut st.table, k),
                    1 => insert(&mut st.table, &e, s, k, 1, kn),
                    2 => {
                        let id = ids[(k as usize / 4) % ids.len()];
                        let l = e.leaves.iter().find(|l| l.id == id).unwrap();
                        let v = gen(l, k, 99, kn);
                        crate::units::set_leaf(&mut st.table, k, id, v);
                    }
                    _ => {}
                }
            }
            st.evo = Some(e);
            true
        }
        (11 | 12, 2) => {
            let mut e = st.evo.take().unwrap();
            e.version += 1;
            for c in [2u32, 5, 15, 16, 31, 40] {
                if (c as usize) < e.cols.len() {
                    e.drop_col(c);
                }
            }
            e.add(&[ty::TEXT]);
            e.add(&opt(&[ty::NAT]));
            e.add(&opt(&[ty::I128]));
            let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
            st.table.change_schema(img).unwrap();
            st.evo = Some(e);
            true
        }
        (11 | 12, 3) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k % 4 == 3 {
                    insert(&mut st.table, &e, s, k, 3, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (11 | 12, 4) | (13 | 14, 3) => {
            let (first, end) = fill_group(st, 7);
            for j in first..end {
                remove(&mut st.table, j);
            }
            true
        }
        (13..=15, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k % 3 == 0 {
                    remove(&mut st.table, k);
                } else if k % 3 == 1 {
                    insert(&mut st.table, &e, s, k, 1_000, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (13 | 14, 2) => {
            drop_cols(st, &[1]);
            true
        }
        (16, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k % 10 != 9 {
                    remove(&mut st.table, k);
                } else if k % 20 == 9 {
                    insert(&mut st.table, &e, s, k, 31, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (16..=19, 2) => {
            drop_cols(st, &[3, 4, 5, 6]);
            true
        }
        (17..=19, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k % 3 == 0 {
                    remove(&mut st.table, k);
                } else {
                    insert(&mut st.table, &e, s, k, 41, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (20, 1) => {
            for k in 0..1_100u64 {
                remove(&mut st.table, (k * 3 + 1) % n);
            }
            true
        }
        (22..=24, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k % 3 == 0 {
                    remove(&mut st.table, k);
                } else {
                    insert(&mut st.table, &e, s, k, 1, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (22..=24, 2) | (32, 2) => {
            drop_cols(st, &[2]);
            true
        }
        (25..=27, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if k != 5 && mix(k, 300 + deep_variant(s), 2) % 3 == 0 {
                    remove(&mut st.table, k);
                } else if k % 5 == 1 {
                    insert(&mut st.table, &e, s, k, 1, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (25..=27, 2) | (29, 2) => {
            drop_cols(st, &[3]);
            true
        }
        (28, 1) => {
            fill_group(st, 5);
            true
        }
        (29, 1) => {
            for k in 0..n {
                if k != 7 && k != 3_001 && mix(k, 712, 6) % 3 == 0 {
                    remove(&mut st.table, k);
                }
            }
            true
        }
        (30 | 32, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                match k % 3 {
                    0 => remove(&mut st.table, k),
                    1 => insert(&mut st.table, &e, s, k, 1_000, kn),
                    _ => {}
                }
            }
            st.evo = Some(e);
            true
        }
        (30, 2) => {
            // The Text columns the first change kept.
            let cols: Vec<u32> = (151..=300).collect();
            drop_cols(st, &cols);
            true
        }
        (31, 1) => {
            let e = st.evo.take().unwrap();
            for k in 0..n {
                if mix(k, 507, 4) % 3 == 0 {
                    remove(&mut st.table, k);
                } else if k % 7 == 1 {
                    insert(&mut st.table, &e, s, k, 1, kn);
                }
            }
            st.evo = Some(e);
            true
        }
        (31, 2) => {
            drop_cols(st, &[3]);
            true
        }
        (33..=39, 1) => {
            // An index added: on the Nat of 10^3 values, W 16; or, scenarios 34 to 37 and 39, on the Text, W 1,016:
            // three entries a node, a tree ten nodes high; or, scenario 38, on the Nat and the Int, W 1,016.
            let mut e = st.evo.take().unwrap();
            let (col, w) = if s == 33 { (1u32, 16u16) } else { (if s == 38 { 1 } else { 2 }, 1_016) };
            let mut part = [(0u32, false); 7];
            part[0] = (col << 16, false);
            part[1] = (2 << 16, false);
            let parts = if s == 38 { 2 } else { 1 };
            e.indexes.push(IndexDecl { id: 1, flags: 0, w, parts, part });
            e.version += 1;
            let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
            st.table.change_schema(img).unwrap();
            if s == 37 {
                // Rows past the build's end under two of the long keys: the build's rows of those keys pass them.
                for i in 0..2u64 {
                    let k = st.next_key;
                    st.next_key += 1;
                    let mut r = Cells(vec![(0, C::U(k)), (1 << 16, C::U(0)), (2 << 16, C::Bytes(pool_key(i)))]);
                    st.table.upsert(&mut key_scratch(), &mut r).unwrap();
                }
            }
            st.evo = Some(e);
            true
        }
        (36, 2) => {
            // The index removed: the drain frees its tree and its key log.
            let mut e = st.evo.take().unwrap();
            e.indexes.clear();
            e.version += 1;
            let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
            st.table.change_schema(img).unwrap();
            st.evo = Some(e);
            true
        }
        _ => false,
    }
}

/// Writes between units (not metered): scenario 12's columns added and rows written once its first seal
/// reaches its write step; scenario 14's table opened with a newer schema image once its first seal has run 40
/// units of its pass 1; scenario 20's 64 rows updated to other big values once its seal, and later its
/// compaction, has taken its snapshot; scenario 32's long values written again after its seal's and its
/// compaction's snapshots.
pub(crate) fn between(slot: &mut Option<State>) {
    let Some(st) = slot.as_mut() else { return };
    let (h, tbl) = (st.table.host(), st.table.root());
    match st.scenario {
        12 | 14 if st.done == 0 => {
            let rw = seal::running(h, tbl, seal::SEAL).unwrap();
            if rw == 0 {
                return;
            }
            let step = seal::step_of(h, rw).unwrap();
            if st.scenario == 12 && step == seal::STEP_WRITE {
                st.done = 1;
                let mut e = st.evo.take().unwrap();
                e.version += 1;
                e.add(&[ty::TEXT]);
                e.add(&opt(&[ty::NAT]));
                let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
                st.table.change_schema(img).unwrap();
                for i in 0..64u64 {
                    let k = st.next_key;
                    st.next_key += 1;
                    insert(&mut st.table, &e, 12, k, 11, knobs(12));
                    insert(&mut st.table, &e, 12, (i * 61) % st.rows, 12, knobs(12));
                }
                st.evo = Some(e);
            } else if st.scenario == 14 && step == seal::STEP_PASS1 && seal::units_of(h, rw).unwrap() >= 40 {
                reopen(slot);
            }
        }
        22..=24 => {
            let kind = match st.done {
                0 => seal::SEAL,
                1 => seal::COMPACTION,
                _ => return,
            };
            let rw = seal::running(h, tbl, kind).unwrap();
            if rw == 0 || seal::step_of(h, rw).unwrap() < seal::STEP_PASS1 {
                return;
            }
            st.done += 1;
            // Rows that live on, updated to big values: (k + salt) % 64 is 0 for scenario 22's all-ones Nat,
            // 1 for scenarios 23's and 24's.
            let salt = 7 + u64::from(st.done);
            let want = if st.scenario == 22 { 0 } else { 1 };
            let mut k = 1u64;
            let mut i = 0;
            while i < 8 && k < st.rows {
                if (k + salt) % 64 == want && k % 3 != 0 {
                    st.table.upsert(&mut key_scratch(), &mut carry_row(st.scenario, k, salt)).unwrap();
                    i += 1;
                }
                k += 1;
            }
        }
        32 => {
            let kind = match st.done {
                0 => seal::SEAL,
                1 => seal::COMPACTION,
                _ => return,
            };
            let rw = seal::running(h, tbl, kind).unwrap();
            if rw == 0 || seal::step_of(h, rw).unwrap() < seal::STEP_PASS1 {
                return;
            }
            let salt = 7 + u64::from(st.done);
            st.done += 1;
            for idx in 0..EDGE_LENS.len() as u64 {
                let k = 10 + 50 * idx;
                if k < st.rows && k % 3 != 0 {
                    st.table.upsert(&mut key_scratch(), &mut edge_row(k, salt)).unwrap();
                }
            }
        }
        37 if st.phase == 1 && st.done < 4 => {
            // A row whose insert raised its key's count before the row is in, removed: the build lowers the
            // count back over units.
            let k0 = Secondary::build_key(h, tbl, 1).unwrap();
            if k0 == 0 {
                return;
            }
            let (next, _) = Secondary::build_state(h, tbl, 1).unwrap();
            let mut ix = Secondary::read(h, tbl, 1).unwrap();
            let word = ix.with_log(st.table.host_mut(), tbl, |l| l.key_word(k0, 4)).unwrap();
            if word & RAISED == 0 || word & !AT_FLAGS == 0 {
                return;
            }
            if let Some(k) = RIDS.with(|m| m.borrow().get(&next).copied()) {
                remove(&mut st.table, k);
                st.done += 1;
            }
        }
        20 => {
            let kind = match st.done {
                0 => seal::SEAL,
                1 => seal::COMPACTION,
                _ => return,
            };
            let rw = seal::running(h, tbl, kind).unwrap();
            if rw == 0 || seal::step_of(h, rw).unwrap() < seal::STEP_PASS1 {
                return;
            }
            st.done += 1;
            for i in 0..64u64 {
                let k = (i * 97) % st.rows;
                st.table.upsert(&mut key_scratch(), &mut big_row(k, u64::from(st.done))).unwrap();
            }
        }
        _ => {}
    }
}

/// Scenario 14's table opened again with a newer schema image (two columns added, every third dropped), as
/// after an upgrade: stale until its next write, and no unit adopts it.
fn reopen(slot: &mut Option<State>) {
    let Some(state) = slot.take() else { return };
    let State { table, scenario, phase, rows, next_key, evo, units, gen, last, .. } = state;
    let mut e = evo.unwrap();
    e.version += 1;
    e.add(&[ty::TEXT]);
    e.add(&opt(&[ty::NAT]));
    for c in 1..e.cols.len() as u32 - 2 {
        if e.cols[c as usize].flags & 1 == 0 && c % 3 == 0 {
            e.drop_col(c);
        }
    }
    let img: &'static [u8] = Box::leak(e.image().into_boxed_slice());
    let root = table.root();
    let table = Table::open(table.into_host(), root, img).unwrap();
    assert!(table.stale());
    *slot = Some(State { table, scenario, phase, rows, next_key, evo: Some(e), units, gen, last, done: 1 });
}
