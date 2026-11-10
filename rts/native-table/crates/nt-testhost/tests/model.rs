//! One model test for the table, over the row types a Motoko native table has: a scalar key, and fields
//! that are scalars or options of scalars, with secondary indexes declared when the table is made. The
//! scalars the crate also takes from Rust rows (u128, i128, f32, fixed-size bytes and unit) are drawn too,
//! less often.
//!
//! Each table is drawn from the seed: its schema (1 to 8 fields, or 60 to 80 mostly integer ones; 0 to 4
//! indexes of 1 to 7 fields; the key's type goes round the types by table number), each field's pool of
//! values, and a mix of writes. Values lean to the sizes where the table's
//! layout changes: Text and Blob lengths around 8, 16, 1,012, 1,024, 2,048, 3,072, 4,096 and 65,536 bytes
//! (keys longer than 4,096 bytes among them), integers at the ends of every width, Nat and Int values past
//! i128 (some of thousands of bytes), floats at their edges (−0.0, infinities, NaN), and option fields that
//! are never, sometimes, mostly or always null. Text and Blob fields outside every index are given now and
//! then as cells the table copies from the message argument, and Nat and Int values within 64 bits now and
//! then as LEB128.
//!
//! The model is a map from key to row id and row. Writes are inserts of new keys, upserts of existing keys,
//! updates of one field, deletes (of absent keys too) and maintenance (single steps, a write step, a
//! maintenance call with no budget, and full maintenance), each write preceded by the write step most of
//! the time, as the runtime does. The table is opened again for every call. Enough rows are written that
//! row groups of 4,096 rows are sealed, updated in place and in patches, compacted and dropped. A Float key
//! equal to a row's in the key order but not in its bits (−0.0 and 0.0) is refused, and the host is put
//! back as it was, as a trapped message leaves it.
//!
//! After every write, the row of the written key is read back. Before the first write, about every
//! thousand writes (at most 25 times a table), after every seal, and once the table has been emptied at the
//! end, every read the table offers is checked against the model: the row count; the scan of every row by row id and by cells; point reads by key and by row id;
//! filtered scans, counts and sums (equality, null, ranges with each kind of end); cell scans of some fields,
//! with a filter and without; running sums and extremes of every integer field; and, for every index, the
//! count of keys present and absent, group counts whole and in pieces, walks forward and backward whole, in
//! pieces and from a key, and the smallest and largest keys whose first part is not null. Index keys are
//! also checked to order and group rows as the values they encode do. Misuse is refused: a field the table
//! does not have, a null where the field holds none or as a range's end, a value of another type, and a sum
//! of a field that is not an integer. The pages the host lends are those the table's areas hold. Once every row is deleted and maintenance has run, the table holds the image
//! pages it was made with, and the blocks it uses do not grow when it is filled and emptied again.
//!
//! Each run changes the table's row type 1 to 3 times, at points drawn from the seed, with the writes and the
//! maintenance of the run between them. A change adds option fields (integer ones often), drops fields
//! (indexed ones often, the indexes on them going with them), or both; an added field may have the type of a
//! field dropped before, as a field that comes back under a dropped one's name has: a new column, null in the
//! rows written before. Rows written after a drop give null for the dropped field. A read hands out a cell for
//! the dropped field, which the runtime throws away: null for a row written after the drop, else null or what
//! the bytes stored before the drop read as. Every read is checked against the model right after each change,
//! and the writes that follow update, delete and compact rows of groups packed under every earlier version.
//! The indexes kept are checked as before; a removed one is refused.
//!
//! `NT_MODEL_SEED` sets the seed (a default is fixed below), `NT_MODEL_TABLES` how many tables are drawn
//! from it (one per key type unless set), and `NT_MODEL_OPS` the writes per table; `NT_MODEL_TABLE` runs only the table of that number,
//! as the output names it.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::ops::Bound;

use native_table::area::AreaRecord;
use native_table::consts::{AREAS, GROUP_SLOTS};
use native_table::index::secondary::Start;
use native_table::mem::rd_u64;
use native_table::schema::IndexDecl;
use native_table::table::filter::{Cond, Filter};
use native_table::table::jobs::Ran;
use native_table::table::totals::Total;
use native_table::table::{buffer, root, CellIn, CellOut, RowSink, Table, Upsert};
use native_table::tree::Dir;
use native_table::value::ty;
use native_table::{Fault, PageHost, Progress, Region, Wake};
use nt_testhost::big::Big;
use nt_testhost::rows::{schema_versioned, t, Src, VCol, V};
use nt_testhost::{TestBudget, VecHost, VecScratch, XorShift, FILLS};

const DEFAULT_SEED: u64 = 0x6E74_6D6F_6465_6C31;

/// A number from the environment, decimal or hex with `0x`.
fn env_u64(name: &str, d: u64) -> u64 {
    let parse = |s: String| match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    };
    std::env::var(name).ok().and_then(parse).unwrap_or(d)
}

/// A generator whose first numbers do not echo a small seed (splitmix64 of it).
fn rng(seed: u64) -> XorShift {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    XorShift((z ^ (z >> 31)) | 1)
}

// ---------------------------------------------------------------- values

/// A field's type, as a Motoko row has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bool,
    /// Nat8 to Nat64, by bits.
    U(u32),
    /// Int8 to Int64, by bits.
    I(u32),
    Nat,
    Int,
    Float,
    Char,
    Text,
    Blob,
    Principal,
    U128,
    I128,
    F32,
    /// `[u8; N]`.
    Fixed(u16),
    Unit,
}

impl Kind {
    /// The field's type expression.
    fn ty(self) -> Vec<u8> {
        t::scalar(match self {
            Kind::Fixed(n) => return t::bytes(n),
            Kind::Bool => ty::BOOL,
            Kind::U(8) => ty::U8,
            Kind::U(16) => ty::U16,
            Kind::U(32) => ty::U32,
            Kind::U(_) => ty::U64,
            Kind::I(8) => ty::I8,
            Kind::I(16) => ty::I16,
            Kind::I(32) => ty::I32,
            Kind::I(_) => ty::I64,
            Kind::Nat => ty::NAT,
            Kind::Int => ty::INT,
            Kind::Float => ty::F64,
            Kind::Char => ty::CHAR,
            Kind::Text => ty::TEXT,
            Kind::Blob => ty::BLOB,
            Kind::Principal => ty::PRINCIPAL,
            Kind::U128 => ty::U128,
            Kind::I128 => ty::I128,
            Kind::F32 => ty::F32,
            Kind::Unit => ty::UNIT,
        })
    }

    /// Whether the table keeps sums and extremes of the field.
    fn integer(self) -> bool {
        matches!(self, Kind::U(_) | Kind::I(_) | Kind::Nat | Kind::Int | Kind::U128 | Kind::I128)
    }
}

/// Field types with weights: the common ones more often, the Rust-only ones least. Fixed-size bytes inline
/// (at most 1,016) and longer. Table t of a run has key type t (mod 22) of this list; a run has 22 tables
/// unless `NT_MODEL_TABLES` says otherwise.
const KINDS: [(Kind, u64); 22] = [
    (Kind::Text, 7),
    (Kind::Nat, 6),
    (Kind::Blob, 3),
    (Kind::Int, 6),
    (Kind::Char, 2),
    (Kind::Float, 3),
    (Kind::Bool, 3),
    (Kind::Principal, 2),
    (Kind::U(64), 3),
    (Kind::I(64), 3),
    (Kind::U(8), 2),
    (Kind::I(8), 2),
    (Kind::U(16), 1),
    (Kind::I(16), 1),
    (Kind::U(32), 1),
    (Kind::I(32), 1),
    (Kind::U128, 1),
    (Kind::I128, 1),
    (Kind::F32, 1),
    (Kind::Fixed(16), 1),
    (Kind::Fixed(1_017), 1),
    (Kind::Unit, 1),
];

/// A value in the model. Every integer type and Char is an integer of any size.
#[derive(Clone, Debug, PartialEq)]
enum Mv {
    Null,
    Bool(bool),
    Int(Big),
    Float(u64),
    F32(u32),
    Bytes(Vec<u8>),
    Unit,
}

fn big(x: i128) -> Big {
    Big::from_i128(x)
}

fn negative(b: &Big) -> bool {
    b.0.last().is_some_and(|&x| x & 0x80 != 0)
}

/// The value as a u128, when it is one.
fn unsigned(b: &Big) -> u128 {
    assert!(!negative(b) && b.0.len() <= 17, "{b:?} is a u128");
    let mut a = [0u8; 16];
    let n = b.0.len().min(16);
    a[..n].copy_from_slice(&b.0[..n]);
    u128::from_le_bytes(a)
}

/// The value as an i128, when it fits one.
fn small(b: &Big) -> Option<i128> {
    if b.0.len() > 16 {
        return None;
    }
    let mut a = [if negative(b) { 0xFF } else { 0 }; 16];
    a[..b.0.len()].copy_from_slice(&b.0);
    Some(i128::from_le_bytes(a))
}

fn cmp_int(a: &Big, b: &Big) -> Ordering {
    match (small(a), small(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.order(b),
    }
}

/// The canonical LEB128 form of the value, signed or not.
fn leb(b: &Big, signed: bool) -> Vec<u8> {
    let ext = if negative(b) { 0xFFu8 } else { 0 };
    // Every byte past `top` is the sign's.
    let top = b.0.iter().rposition(|&x| x != ext);
    let (mut acc, mut bits, mut i) = (0u32, 0u32, 0usize);
    let mut out = Vec::with_capacity(b.0.len() * 8 / 7 + 2);
    loop {
        while bits < 7 {
            acc |= u32::from(*b.0.get(i).unwrap_or(&ext)) << bits;
            (i, bits) = (i + 1, bits + 8);
        }
        let g = (acc & 0x7F) as u8;
        (acc, bits) = (acc >> 7, bits - 7);
        // Done when the bits left are all the sign's, and a signed form's last group carries it.
        let rest = acc == u32::from(ext) & ((1 << bits) - 1) && top.is_none_or(|t| i > t);
        if rest && (!signed || (g & 0x40 != 0) == (ext != 0)) {
            out.push(g);
            return out;
        }
        out.push(g | 0x80);
    }
}

/// The key order of an f64: −0.0 is 0.0, a NaN with sign 0 is above every value and one with sign 1 below.
fn f64_key(bits: u64) -> u64 {
    let f = f64::from_bits(bits);
    if f.is_nan() {
        return if bits >> 63 == 0 { u64::MAX } else { 0 };
    }
    let bits = if f == 0.0 { 0 } else { bits };
    if bits >> 63 == 1 {
        !bits
    } else {
        bits | 1 << 63
    }
}

/// The key order of an f32, by the same rule.
fn f32_key(bits: u32) -> u32 {
    let f = f32::from_bits(bits);
    if f.is_nan() {
        return if bits >> 31 == 0 { u32::MAX } else { 0 };
    }
    let bits = if f == 0.0 { 0 } else { bits };
    if bits >> 31 == 1 {
        !bits
    } else {
        bits | 1 << 31
    }
}

/// The order of two values of a field of kind `k` in an index: null first.
fn cmp_index(k: Kind, a: &Mv, b: &Mv) -> Ordering {
    match (a, b) {
        (Mv::Null, Mv::Null) => Ordering::Equal,
        (Mv::Null, _) => Ordering::Less,
        (_, Mv::Null) => Ordering::Greater,
        (Mv::Int(x), Mv::Int(y)) => cmp_int(x, y),
        (Mv::Float(x), Mv::Float(y)) => f64_key(*x).cmp(&f64_key(*y)),
        (Mv::F32(x), Mv::F32(y)) => f32_key(*x).cmp(&f32_key(*y)),
        (Mv::Bool(x), Mv::Bool(y)) => x.cmp(y),
        (Mv::Unit, Mv::Unit) => Ordering::Equal,
        // A Principal orders as its bytes padded to 29, then its length.
        (Mv::Bytes(x), Mv::Bytes(y)) if k == Kind::Principal => {
            let pad = |b: &[u8]| {
                let mut p = [0u8; 29];
                p[..b.len()].copy_from_slice(b);
                p
            };
            pad(x).cmp(&pad(y)).then(x.len().cmp(&y.len()))
        }
        (Mv::Bytes(x), Mv::Bytes(y)) => x.cmp(y),
        _ => panic!("{k:?}: {a:?} and {b:?} do not compare"),
    }
}

/// The order of two non-null values as a filter compares them; `None` for a NaN.
fn cmp_filter(a: &Mv, b: &Mv) -> Option<Ordering> {
    match (a, b) {
        (Mv::Int(x), Mv::Int(y)) => Some(cmp_int(x, y)),
        (Mv::Float(x), Mv::Float(y)) => f64::from_bits(*x).partial_cmp(&f64::from_bits(*y)),
        (Mv::F32(x), Mv::F32(y)) => f32::from_bits(*x).partial_cmp(&f32::from_bits(*y)),
        (Mv::Bool(x), Mv::Bool(y)) => Some(x.cmp(y)),
        (Mv::Unit, Mv::Unit) => Some(Ordering::Equal),
        (Mv::Bytes(x), Mv::Bytes(y)) => Some(x.cmp(y)),
        _ => panic!("{a:?} and {b:?} do not compare"),
    }
}

/// A cell as a host gives it: a Nat or Int within 64 bits as a word, else as LEB128; Text and Blob as cells
/// read from the message argument when `arg`.
fn cell_of(k: Kind, m: &Mv, arg: bool) -> V {
    let word = |x: &Big| small(x).unwrap();
    match (m, k) {
        (Mv::Null, _) => V::Null,
        (Mv::Bool(b), _) => V::Bool(*b),
        (Mv::Int(x), Kind::U(_) | Kind::Char) => V::U(u64::try_from(word(x)).unwrap()),
        (Mv::Int(x), Kind::U128) => V::U128(unsigned(x)),
        (Mv::Int(x), Kind::I128) => V::I128(word(x)),
        (Mv::Int(x), Kind::I(_)) => V::I(i64::try_from(word(x)).unwrap()),
        (Mv::Int(x), Kind::Nat) => match small(x).and_then(|y| u64::try_from(y).ok()) {
            Some(y) => V::U(y),
            None => V::Nat(leb(x, false)),
        },
        (Mv::Int(x), Kind::Int) => match small(x).and_then(|y| i64::try_from(y).ok()) {
            Some(y) => V::I(y),
            None => V::Int(leb(x, true)),
        },
        (Mv::Float(b), _) => V::F64(*b),
        (Mv::F32(b), _) => V::F32(*b),
        (Mv::Unit, _) => V::Unit,
        (Mv::Bytes(b), _) if arg => V::Arg(b.clone()),
        (Mv::Bytes(b), _) => V::Bytes(b.clone()),
        _ => panic!("{k:?}: no cell for {m:?}"),
    }
}

fn cell_in(v: &V) -> CellIn<'_> {
    match v {
        V::Null => CellIn::Null,
        V::Bool(b) => CellIn::Bool(*b),
        V::U(x) => CellIn::U64(*x),
        V::I(x) => CellIn::I64(*x),
        V::U128(x) => CellIn::U128(*x),
        V::I128(x) => CellIn::I128(*x),
        V::F32(b) => CellIn::F32(f32::from_bits(*b)),
        V::F64(b) => CellIn::F64(f64::from_bits(*b)),
        V::Unit => CellIn::Unit,
        V::Bytes(b) => CellIn::Bytes(b),
        V::Nat(b) => CellIn::Nat(b),
        V::Int(b) => CellIn::Int(b),
        _ => panic!("no filter cell for {v:?}"),
    }
}

/// A cell the table handed out, of a field of kind `k`, as a model value.
fn value_of(k: Kind, v: CellOut<'_>) -> Mv {
    let ok = match v {
        CellOut::Null | CellOut::Arg { .. } => true,
        CellOut::Bool(_) => k == Kind::Bool,
        CellOut::U64(_) => matches!(k, Kind::U(_) | Kind::Char | Kind::Nat),
        CellOut::I64(_) => matches!(k, Kind::I(_) | Kind::Int),
        CellOut::Nat(_) => k == Kind::Nat,
        CellOut::Int(_) => k == Kind::Int,
        CellOut::F64(_) => k == Kind::Float,
        CellOut::U128(_) => k == Kind::U128,
        CellOut::I128(_) => k == Kind::I128,
        CellOut::F32(_) => k == Kind::F32,
        CellOut::Unit => k == Kind::Unit,
        CellOut::Bytes(_) => matches!(k, Kind::Text | Kind::Blob | Kind::Principal | Kind::Fixed(_)),
    };
    assert!(ok, "{k:?}: the table handed out {v:?}");
    match v {
        // A long value is read once the row ends.
        CellOut::Null | CellOut::Arg { .. } => Mv::Null,
        CellOut::Bool(b) => Mv::Bool(b),
        CellOut::U64(x) => Mv::Int(Big::from_u128(x.into())),
        CellOut::I64(x) => Mv::Int(big(x.into())),
        CellOut::Nat(b) => Mv::Int(Big::from_leb(b, false)),
        CellOut::Int(b) => Mv::Int(Big::from_leb(b, true)),
        CellOut::F64(f) => Mv::Float(f.to_bits()),
        CellOut::U128(x) => Mv::Int(Big::from_u128(x)),
        CellOut::I128(x) => Mv::Int(big(x)),
        CellOut::F32(f) => Mv::F32(f.to_bits()),
        CellOut::Unit => Mv::Unit,
        CellOut::Bytes(b) => Mv::Bytes(b.to_vec()),
    }
}

// ---------------------------------------------------------------- generators

fn pick<T: Copy>(r: &mut XorShift, xs: &[T]) -> T {
    xs[r.below(xs.len() as u64) as usize]
}

fn weighted<T: Copy>(r: &mut XorShift, xs: &[(T, u64)]) -> T {
    let mut n = r.below(xs.iter().map(|x| x.1).sum());
    for &(x, w) in xs {
        if n < w {
            return x;
        }
        n -= w;
    }
    unreachable!()
}

/// Byte lengths where the layout changes: index key areas, inline and fixed limits, long values, blocks,
/// key buffers and image blocks.
const EDGES: [usize; 41] = [
    0, 1, 2, 7, 8, 9, 15, 16, 17, 29, 30, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1_011, 1_012, 1_013, 1_015, 1_016,
    1_017, 1_023, 1_024, 1_025, 2_047, 2_048, 2_049, 3_071, 3_072, 3_073, 4_095, 4_096, 4_097, 4_799, 4_800, 4_801,
];
const HUGE: [usize; 6] = [8_191, 8_192, 65_535, 65_536, 65_537, 70_000];

fn length(r: &mut XorShift) -> usize {
    match r.below(200) {
        0..=139 => r.below(12) as usize,
        140..=196 => pick(r, &EDGES),
        _ => pick(r, &HUGE),
    }
}

/// Text, with `00` bytes in half of them.
fn text(r: &mut XorShift) -> Vec<u8> {
    let n = length(r);
    let nul = r.below(2) == 0;
    let mut s = String::new();
    while s.len() < n {
        s.push(match r.below(40) {
            0 if nul => '\0',
            1 => 'é',
            2 => '€',
            3 => '😀',
            x => (b'a' + (x % 3) as u8) as char,
        });
    }
    s.into_bytes()
}

fn blob(r: &mut XorShift) -> Vec<u8> {
    let n = length(r);
    (0..n)
        .map(|_| {
            let x = r.next_u64() as u8;
            pick(r, &[0x00, 0xFF, 0x01, b'a', x])
        })
        .collect()
}

/// Nat edges: the ends of 7, 14, 32, 63, 64, 127 and 128 bits.
fn nat_edge(r: &mut XorShift) -> Big {
    let x = pick(r, &[1u128 << 7, 1 << 14, 1 << 31, 1 << 32, 1 << 63, 1 << 64, 1 << 127, u128::MAX]);
    let x = Big::from_u128(x);
    match r.below(3) {
        0 => x.sub(&big(1)),
        1 => x,
        _ => x.add(&big(1)),
    }
}

/// A Nat or Int past i128: LEB128 of 17 to 40 bytes, now and then of thousands.
fn huge_int(r: &mut XorShift, signed: bool) -> Big {
    let n = if r.below(10) == 0 { 300 + r.below(6_000) } else { 17 + r.below(24) } as usize;
    let mut b: Vec<u8> = (0..n).map(|_| 0x80 | r.next_u64() as u8).collect();
    b[n - 1] = 1 + r.below(0x7F) as u8;
    Big::from_leb(&b, signed)
}

fn int_value(r: &mut XorShift, k: Kind) -> Big {
    match k {
        Kind::U(bits) => {
            let max = u64::MAX >> (64 - bits);
            let x = match r.below(10) {
                0..=4 => r.below(8),
                5..=6 => pick(r, &[0, 1, max, max - 1, max / 2, max / 2 + 1]),
                _ => r.next_u64() & max,
            };
            Big::from_u128(x.into())
        }
        Kind::I(bits) => {
            let (min, max) = (i64::MIN >> (64 - bits), i64::MAX >> (64 - bits));
            let x = match r.below(10) {
                0..=4 => r.below(9) as i64 - 4,
                5..=6 => pick(r, &[min, min + 1, max, max - 1, -1, 0, 1]),
                _ => (r.next_u64() as i64) << (64 - bits) >> (64 - bits),
            };
            big(x.into())
        }
        Kind::Nat => match r.below(20) {
            0..=9 => big(r.below(16).into()),
            10..=13 => nat_edge(r),
            14..=16 => Big::from_u128(r.next_u64().into()),
            17 => Big::from_u128(u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())),
            _ => huge_int(r, false),
        },
        Kind::U128 => Big::from_u128(match r.below(4) {
            0 => r.below(8).into(),
            1 => pick(r, &[0, 1, u64::MAX.into(), 1 << 64, (1 << 127) - 1, 1 << 127, u128::MAX - 1, u128::MAX]),
            _ => u128::from(r.next_u64()) << 64 | u128::from(r.next_u64()),
        }),
        Kind::I128 => big(match r.below(4) {
            0 => r.below(9) as i128 - 4,
            1 => {
                pick(r, &[i128::MIN, i128::MIN + 1, i64::MIN.into(), -1, 0, i64::MAX.into(), i128::MAX - 1, i128::MAX])
            }
            _ => (u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) as i128,
        }),
        Kind::Int => match r.below(20) {
            0..=8 => big(r.below(17) as i128 - 8),
            9..=11 => {
                let x = nat_edge(r);
                if r.below(2) == 0 {
                    x
                } else {
                    big(0).sub(&x)
                }
            }
            12 => {
                let x = big(pick(r, &[i64::MIN.into(), i64::MAX.into(), i128::MIN, i128::MAX]));
                x.add(&big(r.below(3) as i128 - 1))
            }
            13..=16 => big((r.next_u64() as i64).into()),
            17 => big((u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) as i128),
            _ => huge_int(r, true),
        },
        _ => unreachable!(),
    }
}

const FLOATS: [u64; 14] = [
    0,
    1 << 63,
    0x3FF0_0000_0000_0000,
    0xBFF0_0000_0000_0000,
    0x3FE0_0000_0000_0000,
    0x0010_0000_0000_0000,
    1,
    0x7FEF_FFFF_FFFF_FFFF,
    0xFFEF_FFFF_FFFF_FFFF,
    0x7FF0_0000_0000_0000,
    0xFFF0_0000_0000_0000,
    0x7FF8_0000_0000_0000,
    0xFFF8_0000_0000_0000,
    0x7FF0_0000_0000_0001,
];

const FLOATS32: [u32; 10] =
    [0, 1 << 31, 0x3F80_0000, 0xBF80_0000, 1, 0x7F7F_FFFF, 0x7F80_0000, 0xFF80_0000, 0x7FC0_0000, 0xFFC0_0001];

const CHARS: [u32; 11] = [0, 0x41, 0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFFFF, 0x10000, 0x10FFFF];

/// A non-null value of kind `k`, leaning to the edges of its type.
fn value(r: &mut XorShift, k: Kind) -> Mv {
    match k {
        Kind::Bool => Mv::Bool(r.below(2) == 0),
        Kind::U(_) | Kind::I(_) | Kind::Nat | Kind::Int | Kind::U128 | Kind::I128 => Mv::Int(int_value(r, k)),
        Kind::Float => Mv::Float(match r.below(4) {
            0 => pick(r, &FLOATS),
            1 => (r.below(7) as f64 - 3.0).to_bits(),
            _ => r.next_u64(),
        }),
        Kind::Char => Mv::Int(big(match r.below(3) {
            0 => pick(r, &CHARS),
            1 => 0x61 + r.below(26) as u32,
            _ => char::from_u32(r.below(0x11_0000) as u32).map_or(0xFFFD, u32::from),
        }
        .into())),
        Kind::F32 => Mv::F32(match r.below(4) {
            0 => pick(r, &FLOATS32),
            1 => (r.below(7) as f32 - 3.0).to_bits(),
            _ => r.next_u64() as u32,
        }),
        Kind::Text => Mv::Bytes(text(r)),
        Kind::Blob => Mv::Bytes(blob(r)),
        Kind::Fixed(n) => Mv::Bytes(
            (0..n)
                .map(|_| {
                    let x = r.next_u64() as u8;
                    pick(r, &[0x00, 0xFF, x])
                })
                .collect(),
        ),
        Kind::Unit => Mv::Unit,
        Kind::Principal => {
            let any = r.below(30) as usize;
            let n = pick(r, &[0, 1, 2, 10, 28, 29, any]);
            Mv::Bytes(
                (0..n)
                    .map(|_| {
                        let x = r.next_u64() as u8;
                        pick(r, &[0, 1, 0xFF, x])
                    })
                    .collect(),
            )
        }
    }
}

/// A key that is most likely new: drawn from the whole of its type, short Text and Blob.
fn fresh(r: &mut XorShift, k: Kind) -> Mv {
    match k {
        Kind::U(bits) => Mv::Int(Big::from_u128((r.next_u64() >> (64 - bits)).into())),
        Kind::I(bits) => Mv::Int(big(((r.next_u64() as i64) >> (64 - bits)).into())),
        Kind::Nat => Mv::Int(Big::from_u128(r.next_u64().into())),
        Kind::Int => Mv::Int(big((r.next_u64() as i64).into())),
        Kind::Float => Mv::Float(r.next_u64()),
        Kind::U128 => Mv::Int(Big::from_u128(u128::from(r.next_u64()) << 64 | u128::from(r.next_u64()))),
        Kind::I128 => Mv::Int(big((u128::from(r.next_u64()) << 64 | u128::from(r.next_u64())) as i128)),
        Kind::F32 => Mv::F32(r.next_u64() as u32),
        Kind::Fixed(n) => Mv::Bytes((0..n).map(|_| r.next_u64() as u8).collect()),
        Kind::Char => Mv::Int(big(char::from_u32(r.below(0x11_0000) as u32).map_or(0xFFFD, u32::from).into())),
        Kind::Text | Kind::Blob => Mv::Bytes((0..4 + r.below(12)).map(|_| b'a' + r.below(26) as u8).collect()),
        Kind::Principal => Mv::Bytes((0..10 + r.below(20)).map(|_| r.next_u64() as u8).collect()),
        _ => value(r, k),
    }
}

// ---------------------------------------------------------------- schemas

/// A field: its kind, whether it is an option, how often it is null (percent), whether a Text or Blob is
/// given as an argument cell now and then, the pool most of its values come from, the version that added it,
/// and the version that dropped it (0 while it is a field of the row).
struct Field {
    kind: Kind,
    opt: bool,
    nulls: u64,
    arg: bool,
    pool: Vec<Mv>,
    added: u32,
    dropped: u32,
}

/// A table's schema: column 0 the key, then one column per field, dropped ones included; the indexes as (id,
/// columns); the ids of the indexes removed with a dropped field; the schema version.
struct Schema {
    cols: Vec<Field>,
    indexes: Vec<(u8, Vec<usize>)>,
    removed: Vec<u8>,
    version: u32,
    img: Vec<u8>,
}

impl Schema {
    /// The columns of the fields the row has: not the key, not a dropped one.
    fn fields(&self) -> Vec<usize> {
        (1..self.cols.len()).filter(|&c| self.cols[c].dropped == 0).collect()
    }

    /// The schema image of this version.
    fn image(&self) -> Vec<u8> {
        let cols: Vec<VCol> = self
            .cols
            .iter()
            .map(|f| VCol {
                ty: if f.opt { t::opt(f.kind.ty()) } else { f.kind.ty() },
                added: f.added,
                dropped: f.dropped,
                default: None,
            })
            .collect();
        let decls: Vec<IndexDecl> = self
            .indexes
            .iter()
            .map(|(id, on)| {
                let mut part = [(0u32, false); 7];
                for (p, &c) in part.iter_mut().zip(on) {
                    *p = (leaf(c), self.cols[c].opt);
                }
                IndexDecl { id: *id, flags: 0, w: 16, parts: on.len() as u8, part }
            })
            .collect();
        schema_versioned(self.version, &cols, &decls).0
    }
}

fn leaf(c: usize) -> u32 {
    (c as u32) << 16
}

/// The integer kinds a wide row's fields and added fields lean to.
const INTS: [Kind; 10] = [
    Kind::Nat,
    Kind::Int,
    Kind::I(64),
    Kind::U(8),
    Kind::I(16),
    Kind::U(16),
    Kind::I(32),
    Kind::U(32),
    Kind::U128,
    Kind::I128,
];

/// A field of kind `k`, an option when `opt`, added by version `added`: how often it is null, and its pool.
fn draw_field(r: &mut XorShift, kind: Kind, opt: bool, added: u32) -> Field {
    let nulls = if opt { pick(r, &[0, 25, 75, 100]) } else { 0 };
    let size = if r.below(6) == 0 { 1 } else { 2 + r.below(40) };
    // Values of thousands of bytes come fresh only: a pool hands its values to many rows.
    let pooled = |r: &mut XorShift| loop {
        match value(r, kind) {
            Mv::Bytes(b) if b.len() > 5_000 => {}
            Mv::Int(x) if x.0.len() > 64 => {}
            v => return v,
        }
    };
    let pool = (0..size).map(|_| pooled(r)).collect();
    Field { kind, opt, nulls, arg: false, pool, added, dropped: 0 }
}

fn draw_schema(r: &mut XorShift, key: Kind) -> Schema {
    // A wide row: more integer fields than the running totals the hot state has room for.
    let wide = r.below(10) == 0;
    let n = match r.below(10) {
        _ if wide => 60 + r.below(21) as usize,
        0 => 1,
        _ => 2 + r.below(7) as usize,
    };
    let mut cols = vec![Field { kind: key, opt: false, nulls: 0, arg: false, pool: Vec::new(), added: 1, dropped: 0 }];
    for _ in 0..n {
        let kind = if wide && r.below(5) != 0 { pick(r, &INTS) } else { weighted(r, &KINDS) };
        // A schema holds no option of unit.
        let opt = kind != Kind::Unit && r.below(5) < 2;
        cols.push(draw_field(r, kind, opt, 1));
    }
    let mut indexes: Vec<(u8, Vec<usize>)> = Vec::new();
    for _ in 0..pick(r, &[0, 1, 1, 2, 2, 3, 4]) {
        let parts = if r.below(8) == 0 { 4 + r.below(4) } else { 1 + r.below(3) } as usize;
        let mut on: Vec<usize> = Vec::new();
        while on.len() < parts.min(n) {
            let c = 1 + r.below(n as u64) as usize;
            if !on.contains(&c) {
                on.push(c);
            }
        }
        if !indexes.iter().any(|x| x.1 == on) {
            indexes.push((indexes.len() as u8 + 1, on));
        }
    }
    for (c, f) in cols.iter_mut().enumerate().skip(1) {
        let indexed = indexes.iter().any(|x| x.1.contains(&c));
        f.arg = matches!(f.kind, Kind::Text | Kind::Blob) && !indexed && r.below(2) == 0;
    }
    let mut s = Schema { cols, indexes, removed: Vec::new(), version: 1, img: Vec::new() };
    s.img = s.image();
    s
}

/// The next version of the schema: option fields added, fields dropped (with the indexes on them), or both,
/// at least one field left. An added field has a dropped field's type now and then. Every row of the model
/// gets null for the added fields; a dropped field keeps its values in the model.
fn draw_change(r: &mut XorShift, s: &mut Schema, m: &mut Model) {
    let fields = s.fields();
    let what = r.below(3);
    let mut adds = if what != 1 { 1 + r.below(3) } else { 0 };
    let most = if adds > 0 { fields.len() } else { fields.len() - 1 };
    let drops = if what != 0 { (1 + r.below(3) as usize).min(most) } else { 0 };
    if drops == 0 {
        adds = adds.max(1);
    }
    s.version += 1;
    let mut left = fields;
    for _ in 0..drops {
        let indexed: Vec<usize> = left.iter().copied().filter(|c| s.indexes.iter().any(|x| x.1.contains(c))).collect();
        let c = if !indexed.is_empty() && r.below(2) == 0 { pick(r, &indexed) } else { pick(r, &left) };
        left.retain(|&x| x != c);
        s.cols[c].dropped = s.version;
    }
    let (kept, removed): (Vec<_>, Vec<_>) =
        std::mem::take(&mut s.indexes).into_iter().partition(|x| x.1.iter().all(|&c| s.cols[c].dropped == 0));
    s.indexes = kept;
    s.removed.extend(removed.iter().map(|x| x.0));
    let gone: Vec<Kind> = s.cols.iter().filter(|f| f.dropped != 0).map(|f| f.kind).collect();
    for _ in 0..adds {
        // An option of any type but unit.
        let kind = loop {
            let k = match r.below(4) {
                0 if !gone.is_empty() => pick(r, &gone),
                0 | 1 => pick(r, &INTS),
                _ => weighted(r, &KINDS),
            };
            if k != Kind::Unit {
                break k;
            }
        };
        let mut f = draw_field(r, kind, true, s.version);
        f.arg = matches!(kind, Kind::Text | Kind::Blob) && r.below(2) == 0;
        s.cols.push(f);
        for e in m.values_mut() {
            e.1.push(Mv::Null);
        }
    }
    s.img = s.image();
}

/// A value of the field for a row: null for a dropped field and as often as the field is, else from its pool
/// most of the time.
fn field_value(r: &mut XorShift, f: &Field) -> Mv {
    if f.dropped != 0 {
        return Mv::Null;
    }
    if f.opt && r.below(100) < f.nulls {
        return Mv::Null;
    }
    if f.pool.is_empty() || r.below(4) == 0 {
        return value(r, f.kind);
    }
    f.pool[r.below(f.pool.len() as u64) as usize].clone()
}

// ---------------------------------------------------------------- the model

/// A primary key, ordered and equal as the table's key encoding orders it.
#[derive(Clone, Debug)]
struct Key(Kind, Mv);

impl PartialEq for Key {
    fn eq(&self, o: &Key) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Key) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Key) -> Ordering {
        cmp_index(self.0, &self.1, &o.1)
    }
}

/// key → (row id, row as written, one value per column).
type Model = BTreeMap<Key, (u64, Vec<Mv>)>;

/// A row's cells; a Nat or Int within 64 bits now and then as LEB128, as a host may give it.
fn row_cells(s: &Schema, row: &[Mv], r: &mut XorShift) -> Vec<(u32, V)> {
    s.cols
        .iter()
        .zip(row)
        .enumerate()
        .map(|(c, (f, m))| {
            let v = match (cell_of(f.kind, m, f.arg && r.below(3) == 0), m) {
                (V::U(_), Mv::Int(x)) if f.kind == Kind::Nat && r.below(8) == 0 => V::Nat(leb(x, false)),
                (V::I(_), Mv::Int(x)) if f.kind == Kind::Int && r.below(8) == 0 => V::Int(leb(x, true)),
                (v, _) => v,
            };
            (leaf(c), v)
        })
        .collect()
}

fn key_cells(s: &Schema, key: &Mv) -> Vec<(u32, V)> {
    vec![(0, cell_of(s.cols[0].kind, key, false))]
}

/// The row as a full read hands it out: every column's leaf, in order.
fn expected(row: &[Mv]) -> Vec<(u32, Mv)> {
    row.iter().enumerate().map(|(c, m)| (leaf(c), m.clone())).collect()
}

/// Rows a read handed out (row id and cells) against the model's: equal, but for the cell of a dropped field,
/// which the runtime throws away: null where the model holds null, and anything where it holds the value the
/// row had when the field was dropped.
fn same(s: &Schema, got: &[(u64, Vec<(u32, Mv)>)], want: &[(u64, Vec<(u32, Mv)>)]) -> bool {
    let cell = |(la, x): &(u32, Mv), (lb, y): &(u32, Mv)| {
        la == lb && if s.cols[(*la >> 16) as usize].dropped != 0 { *y != Mv::Null || *x == Mv::Null } else { x == y }
    };
    got.len() == want.len()
        && got
            .iter()
            .zip(want)
            .all(|((ra, a), (rb, b))| ra == rb && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| cell(x, y)))
}

/// A filter on one field, in the model.
#[derive(Clone, Debug)]
enum MCond {
    Eq(Mv),
    Range(Bound<Mv>, Bound<Mv>),
}

fn meets(v: &Mv, c: &MCond) -> bool {
    let end = |b: &Bound<Mv>, low: bool| match b {
        Bound::Unbounded => true,
        Bound::Included(x) => cmp_filter(v, x).is_some_and(|o| o == Ordering::Equal || (o == Ordering::Greater) == low),
        Bound::Excluded(x) => cmp_filter(v, x).is_some_and(|o| o != Ordering::Equal && (o == Ordering::Greater) == low),
    };
    match (v, c) {
        (Mv::Null, MCond::Eq(Mv::Null)) => true,
        (Mv::Null, _) | (_, MCond::Eq(Mv::Null)) => false,
        (v, MCond::Eq(x)) => cmp_filter(v, x) == Some(Ordering::Equal),
        (_, MCond::Range(lo, hi)) => end(lo, true) && end(hi, false),
    }
}

fn draw_filter(r: &mut XorShift, s: &Schema) -> (usize, MCond) {
    let c = pick(r, &s.fields());
    let f = &s.cols[c];
    let val = |r: &mut XorShift| {
        if f.pool.is_empty() || r.below(4) == 0 {
            value(r, f.kind)
        } else {
            f.pool[r.below(f.pool.len() as u64) as usize].clone()
        }
    };
    let cond = match r.below(5) {
        0 if f.opt => MCond::Eq(Mv::Null),
        0 | 1 => MCond::Eq(val(r)),
        _ => {
            let end = |r: &mut XorShift| match r.below(5) {
                0 => Bound::Unbounded,
                1 | 2 => Bound::Included(val(r)),
                _ => Bound::Excluded(val(r)),
            };
            MCond::Range(end(r), end(r))
        }
    };
    (c, cond)
}

// ---------------------------------------------------------------- reading the table

/// Collects the rows a read hands out, reading each long value with `read_large` in pieces of `piece` bytes.
/// A dropped field's cell is kept as null or not, `Mv::Unit` for any value, and never read further, as the
/// runtime leaves it.
struct Rows<'t, 'h> {
    tb: &'t Table<'h, VecHost>,
    kinds: Vec<Kind>,
    dropped: Vec<bool>,
    piece: usize,
    cur: Vec<(u32, Mv)>,
    long: Vec<(usize, u64)>,
    out: Vec<(u64, Vec<(u32, Mv)>)>,
}

impl<'t, 'h> Rows<'t, 'h> {
    fn new(tb: &'t Table<'h, VecHost>, s: &Schema, r: &mut XorShift) -> Self {
        let piece = pick(r, &[333, 4_096, 65_536, 1 << 20]);
        Rows {
            tb,
            kinds: s.cols.iter().map(|f| f.kind).collect(),
            dropped: s.cols.iter().map(|f| f.dropped != 0).collect(),
            piece,
            cur: Vec::new(),
            long: Vec::new(),
            out: Vec::new(),
        }
    }
}

impl RowSink for Rows<'_, '_> {
    fn cell(&mut self, leaf: u32, v: CellOut<'_>) -> Result<(), Fault> {
        let (k, dropped) = (self.kinds[(leaf >> 16) as usize], self.dropped[(leaf >> 16) as usize]);
        if dropped {
            self.cur.push((leaf, if let CellOut::Null = v { Mv::Null } else { Mv::Unit }));
            return Ok(());
        }
        if let CellOut::Arg { len } = v {
            self.long.push((self.cur.len(), len));
        }
        self.cur.push((leaf, value_of(k, v)));
        Ok(())
    }

    fn row_end(&mut self, rid: u64) -> Result<(), Fault> {
        for (i, len) in std::mem::take(&mut self.long) {
            let (leaf, _) = self.cur[i];
            let mut bytes = vec![0u8; len as usize];
            let mut at = 0usize;
            while at < bytes.len() {
                let end = (at + self.piece).min(bytes.len());
                let n = self.tb.read_large(rid, leaf, at as u64, &mut bytes[at..end])?;
                assert_eq!(n, end - at, "read_large hands out what it is asked for");
                at = end;
            }
            let k = self.kinds[(leaf >> 16) as usize];
            self.cur[i].1 = match k {
                Kind::Nat | Kind::Int => Mv::Int(Big::from_leb(&bytes, k == Kind::Int)),
                _ => Mv::Bytes(bytes),
            };
        }
        self.out.push((rid, std::mem::take(&mut self.cur)));
        Ok(())
    }
}

/// `f` with a buffer of `size` bytes that grows to the bytes each `Scratch` names.
fn grown<T>(mut size: usize, mut f: impl FnMut(&mut [u8]) -> Result<T, Fault>) -> (T, Vec<u8>) {
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

fn total(t: Total, buf: &[u8]) -> Big {
    Big::of_total(t, buf)
}

/// An index walk from `start`, at most `limit` rows, through a buffer of `size` bytes that grows to what
/// each `Scratch` names, resumed after the last row it handed out.
fn walk(
    tb: &Table<'_, VecHost>,
    ix: u8,
    dir: Dir,
    start: Start<'_>,
    limit: u64,
    mut size: usize,
) -> Vec<(Vec<u8>, u64)> {
    let mut out: Vec<(Vec<u8>, u64)> = Vec::new();
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
        let n = got.len() as u64;
        out.extend(got);
        match r {
            Ok(k) => {
                assert_eq!(k, n, "index {ix}: a walk counts the rows it hands out");
                return out;
            }
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size, "index {ix}: a need of {need} bytes for a buffer of {size}");
                size = need as usize;
            }
            Err(e) => panic!("index {ix}: walk: {e:?}"),
        }
    }
}

/// An index's group counts after `after`, at most `limit`, through a growing buffer.
fn groups(tb: &Table<'_, VecHost>, ix: u8, after: Option<&[u8]>, limit: u64, mut size: usize) -> Vec<(Vec<u8>, i128)> {
    let mut out: Vec<(Vec<u8>, i128)> = Vec::new();
    loop {
        let mut buf = vec![0u8; size];
        let last = out.last().map(|x| x.0.clone());
        let from = last.as_deref().or(after);
        let mut got = Vec::new();
        let r = tb.group_counts(ix, from, limit - out.len() as u64, &mut buf, |k, n| {
            got.push((k.to_vec(), n));
            Ok(())
        });
        let n = got.len() as u64;
        out.extend(got);
        match r {
            Ok(k) => {
                assert_eq!(k, n, "index {ix}: group counts count the keys they hand out");
                return out;
            }
            Err(Fault::Scratch { need }) => {
                assert!(need as usize > size, "index {ix}: a need of {need} bytes for a buffer of {size}");
                size = need as usize;
            }
            Err(e) => panic!("index {ix}: group counts: {e:?}"),
        }
    }
}

fn index_key(tb: &Table<'_, VecHost>, ix: u8, cells: &[(u32, V)]) -> Vec<u8> {
    let size: usize = cells
        .iter()
        .map(|(_, v)| match v {
            V::Bytes(b) | V::Nat(b) | V::Int(b) => 2 * b.len() + 32,
            _ => 32,
        })
        .sum();
    let (n, buf) = grown(size, |out| tb.index_key(ix, &mut Src::new(cells), out));
    buf[..n].to_vec()
}

/// The pages the host lends equal those the table's areas hold.
fn check_pages(tb: &Table<'_, VecHost>) {
    let held: u64 = (0..AREAS as u32).map(|a| AreaRecord::read(tb.host(), tb.root(), a).unwrap().pages).sum();
    assert_eq!(tb.host().live_pages() as u64, held, "the host's live pages are the table's");
}

/// Blocks in use in areas 0 and 1 (their pages' blocks, less the never-used ones and the free ones), and the
/// image pages of area 15.
fn held(tb: &Table<'_, VecHost>) -> (u64, u64, u64) {
    let used = |a: u32| {
        let rec = AreaRecord::read(tb.host(), tb.root(), a).unwrap();
        let mut free = 0u64;
        let mut b = rec.free_head;
        while b != 0 {
            free += 1;
            b = rd_u64(tb.host().get(Region::Block(b)).unwrap(), 0).unwrap();
        }
        let never = if rec.newest != 0 { 16 - u64::from(rec.bump) } else { 0 };
        rec.pages * 16 - never - free
    };
    (used(0), used(1), AreaRecord::read(tb.host(), tb.root(), 15).unwrap().pages)
}

// ---------------------------------------------------------------- the checker

/// Every read of the table against the model.
fn check(tb: &Table<'_, VecHost>, s: &Schema, m: &Model, dead: &[u64], r: &mut XorShift) {
    let (h, tbl) = (tb.host(), tb.root());
    assert_eq!(tb.len().unwrap(), m.len() as u64, "the row count");
    assert_eq!(tb.is_empty().unwrap(), m.is_empty());
    let by_rid: BTreeMap<u64, &(u64, Vec<Mv>)> = m.values().map(|e| (e.0, e)).collect();
    let all: Vec<(u64, Vec<(u32, Mv)>)> = by_rid.iter().map(|(rid, e)| (*rid, expected(&e.1))).collect();

    // The scan of every row as the runtime makes it: each row id of each group the directory holds.
    let (open, _, tail) = buffer::open_group(h, tbl).unwrap();
    let end = open * GROUP_SLOTS + buffer::u16_of(h, tail, buffer::H_OPEN_POS).unwrap();
    let mut rows = Rows::new(tb, s, r);
    let mut rid = 0;
    while rid < end {
        let g = rid / GROUP_SLOTS;
        if root::group(h, tbl, g).unwrap() == 0 {
            rid = (g + 1) * GROUP_SLOTS;
            continue;
        }
        tb.get_rid(rid, &mut rows).unwrap();
        rid += 1;
    }
    assert!(same(s, &rows.out, &all), "the scan by row id");

    // Point reads: by key, present and absent, and by row id, live and dead.
    let keys: Vec<&Key> = m.keys().collect();
    let mut scratch = VecScratch::new(FILLS[0]);
    for i in 0..keys.len().min(150) {
        let k = keys[if keys.len() <= 150 { i } else { r.below(keys.len() as u64) as usize }];
        let (rid, row) = &m[k];
        let mut out = Rows::new(tb, s, r);
        assert!(tb.get(&mut scratch, &mut Src::new(&key_cells(s, &k.1)), &mut out).unwrap(), "get {k:?}");
        assert!(same(s, &out.out, &[(*rid, expected(row))]), "get {k:?}");
    }
    for _ in 0..5 {
        let k = Key(s.cols[0].kind, value(r, s.cols[0].kind));
        let mut out = Rows::new(tb, s, r);
        let found = tb.get(&mut scratch, &mut Src::new(&key_cells(s, &k.1)), &mut out).unwrap();
        assert_eq!(found, m.contains_key(&k), "get {k:?}");
    }
    for &rid in dead.iter().rev().take(20) {
        let mut out = Rows::new(tb, s, r);
        assert!(!tb.get_rid(rid, &mut out).unwrap(), "row {rid} is deleted");
    }

    // Cell scans: some fields or every one, in any order, the key's included now and then, in pieces.
    let cell_scan = |f: Option<&Filter<'_>>, want: &dyn Fn(&Vec<Mv>) -> bool, r: &mut XorShift| {
        let every = r.below(4) == 0;
        let mut cols: Vec<usize> =
            (0..s.cols.len()).filter(|&c| s.cols[c].dropped == 0 && (every || r.below(3) == 0)).collect();
        for i in (1..cols.len()).rev() {
            cols.swap(i, r.below(i as u64 + 1) as usize);
        }
        let leaves: Vec<u32> = cols.iter().map(|&c| leaf(c)).collect();
        let limit = pick(r, &[1, 3, 64, 1_000, u64::MAX]);
        let mut out = Rows::new(tb, s, r);
        let mut next = Some(0);
        while let Some(n) = next {
            next = tb.scan_cells(&leaves, f, n, limit, &mut out).unwrap();
        }
        let exp: Vec<(u64, Vec<(u32, Mv)>)> = by_rid
            .iter()
            .filter(|(_, e)| want(&e.1))
            .map(|(rid, e)| (*rid, cols.iter().map(|&c| (leaf(c), e.1[c].clone())).collect()))
            .collect();
        assert!(out.out == exp, "a cell scan of columns {cols:?} with {f:?}");
    };
    cell_scan(None, &|_| true, r);

    // Misuse is refused: a field the table does not have, a null where the field holds none or as a range's
    // end, a value of another type, a sum of a field that is not an integer.
    let none = leaf(s.cols.len());
    let bad = |f: Filter<'_>| tb.count_where(&f).unwrap_err();
    assert_eq!(bad(Filter { leaf: none, cond: Cond::Eq(CellIn::U64(1)) }), Fault::BadRef, "no such field");
    let mut out = Rows::new(tb, s, r);
    assert_eq!(tb.scan_cells(&[leaf(0), none], None, 0, 10, &mut out), Err(Fault::BadRef), "a cell of no field");
    let c = pick(r, &s.fields());
    let (k, refused) = (s.cols[c].kind, Fault::BadValue { leaf: leaf(c) });
    if !s.cols[c].opt {
        assert_eq!(bad(Filter { leaf: leaf(c), cond: Cond::Eq(CellIn::Null) }), refused, "column {c}: null");
    }
    let null_end = Cond::Range(Bound::Included(CellIn::Null), Bound::Unbounded);
    assert_eq!(bad(Filter { leaf: leaf(c), cond: null_end }), refused, "column {c}: a null end");
    let other = if k == Kind::Bool { CellIn::Bytes(b"x") } else { CellIn::Bool(true) };
    assert_eq!(bad(Filter { leaf: leaf(c), cond: Cond::Eq(other) }), refused, "column {c}: another type");
    if !k.integer() {
        let all = Filter { leaf: leaf(0), cond: Cond::Range(Bound::Unbounded, Bound::Unbounded) };
        assert_eq!(tb.sum_where(&all, leaf(c), &mut [0u8; 64]), Err(Fault::BadRef), "a sum of column {c}");
    }

    // Filtered scans, counts, sums and cell scans.
    let ints: Vec<usize> = s.fields().into_iter().filter(|&c| s.cols[c].kind.integer()).collect();
    for _ in 0..6 {
        let (c, mc) = draw_filter(r, s);
        let k = s.cols[c].kind;
        let bound = |b: &Bound<Mv>| b.as_ref().map(|x| cell_of(k, x, false));
        let (eq, lo, hi) = match &mc {
            MCond::Eq(x) => (Some(cell_of(k, x, false)), Bound::Unbounded, Bound::Unbounded),
            MCond::Range(lo, hi) => (None, bound(lo), bound(hi)),
        };
        let cond = match &eq {
            Some(v) => Cond::Eq(cell_in(v)),
            None => Cond::Range(lo.as_ref().map(cell_in), hi.as_ref().map(cell_in)),
        };
        let f = Filter { leaf: leaf(c), cond };
        let want = |row: &Vec<Mv>| meets(&row[c], &mc);
        let exp: Vec<(u64, Vec<(u32, Mv)>)> =
            by_rid.iter().filter(|(_, e)| want(&e.1)).map(|(rid, e)| (*rid, expected(&e.1))).collect();
        let limit = pick(r, &[1, 2, 64, 500, u64::MAX]);
        let mut out = Rows::new(tb, s, r);
        let mut next = Some(0);
        while let Some(n) = next {
            next = tb.scan_where(Some(&f), n, limit, &mut out).unwrap();
        }
        assert!(same(s, &out.out, &exp), "a filtered scan of column {c} ({k:?}) with {mc:?}");
        assert_eq!(tb.count_where(&f).unwrap(), exp.len() as u64, "a filtered count, column {c} with {mc:?}");
        if let Some(&sc) = ints.get(r.below(ints.len() as u64 + 1) as usize) {
            let (mut sum, mut n) = (big(0), 0u64);
            for e in by_rid.values().filter(|e| want(&e.1)) {
                if let Mv::Int(x) = &e.1[sc] {
                    sum = sum.add(x);
                    n += 1;
                }
            }
            let ((t, got_n), buf) = grown(pick(r, &[0, 16, 64]), |buf| tb.sum_where(&f, leaf(sc), buf));
            assert_eq!((total(t, &buf), got_n), (sum, n), "the sum of column {sc} where column {c} meets {mc:?}");
        }
        cell_scan(Some(&f), &want, r);
    }

    // Running sums and extremes of every integer field.
    for &c in &ints {
        let vals: Vec<&Big> =
            by_rid.values().filter_map(|e| if let Mv::Int(x) = &e.1[c] { Some(x) } else { None }).collect();
        let sum = vals.iter().fold(big(0), |a, x| a.add(x));
        let ((t, n), buf) = grown(pick(r, &[0, 16, 64]), |buf| tb.sum(leaf(c), buf));
        assert_eq!((total(t, &buf), n), (sum, vals.len() as u64), "the sum of column {c}");
        for largest in [false, true] {
            let want = vals.iter().copied().reduce(|a, b| {
                let o = cmp_int(a, b);
                if (o == Ordering::Less) == largest {
                    b
                } else {
                    a
                }
            });
            let (got, buf) = grown(pick(r, &[0, 16, 64]), |buf| tb.extreme(leaf(c), largest, buf));
            assert_eq!(got.map(|t| total(t, &buf)), want.cloned(), "the extreme of column {c}, largest {largest}");
        }
    }

    // Every index, and none of those removed with a dropped field.
    for (ix, on) in &s.indexes {
        check_index(tb, s, *ix, on, &by_rid, r);
    }
    for &ix in &s.removed {
        let got = tb.walk(ix, Dir::Fwd, Start::End, 1, &mut [0u8; 64], |_, _| Ok(()));
        assert_eq!(got, Err(Fault::BadRef), "index {ix} was removed");
    }
    check_pages(tb);
}

/// One key of an index in the model: its encoding, its cells, and the ids of its rows, rising.
struct Group {
    enc: Vec<u8>,
    key: Vec<Mv>,
    rids: Vec<u64>,
}

/// The walk's rows over `groups` in their order: each key's encoding with each of its row ids, rising.
fn walk_rows<'a>(groups: impl Iterator<Item = &'a Group>) -> Vec<(Vec<u8>, u64)> {
    groups.flat_map(|g| g.rids.iter().map(|&rid| (g.enc.clone(), rid))).collect()
}

fn check_index(
    tb: &Table<'_, VecHost>,
    s: &Schema,
    ix: u8,
    on: &[usize],
    by_rid: &BTreeMap<u64, &(u64, Vec<Mv>)>,
    r: &mut XorShift,
) {
    let cmp = |a: &[Mv], b: &[Mv]| {
        on.iter()
            .zip(a.iter().zip(b))
            .map(|(&c, (x, y))| cmp_index(s.cols[c].kind, x, y))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal)
    };
    let cells = |key: &[Mv]| -> Vec<(u32, V)> {
        on.iter().zip(key).map(|(&c, m)| (leaf(c), cell_of(s.cols[c].kind, m, false))).collect()
    };
    // Rows in key order, the rows of one key in row id order.
    let mut entries: Vec<(Vec<Mv>, u64)> =
        by_rid.values().map(|e| (on.iter().map(|&c| e.1[c].clone()).collect(), e.0)).collect();
    entries.sort_by(|a, b| cmp(&a.0, &b.0).then(a.1.cmp(&b.1)));
    // The keys, each encoded: encodings rise with the keys, and equal keys encode equally.
    let mut grouped: Vec<Group> = Vec::new();
    for (key, rid) in entries {
        match grouped.last_mut() {
            Some(g) if cmp(&g.key, &key).is_eq() => {
                if r.below(8) == 0 {
                    assert_eq!(index_key(tb, ix, &cells(&key)), g.enc, "index {ix}: equal keys encode equally");
                }
                g.rids.push(rid);
            }
            last => {
                let enc = index_key(tb, ix, &cells(&key));
                if let Some(g) = last {
                    assert!(g.enc < enc, "index {ix}: keys encode in their order: {:?} then {key:?}", g.key);
                }
                grouped.push(Group { enc, key, rids: vec![rid] });
            }
        }
    }
    let size = |r: &mut XorShift| pick(r, &[0, 48, 200, 4_096, 1 << 18]);
    for dir in [Dir::Fwd, Dir::Rev] {
        let want = if dir == Dir::Fwd { walk_rows(grouped.iter()) } else { walk_rows(grouped.iter().rev()) };
        assert!(walk(tb, ix, dir, Start::End, u64::MAX, size(r)) == want, "index {ix}, {dir:?}: the walk");
        // In pieces, each resumed after the last row of the one before.
        let limit = 1 + r.below(200);
        let mut got = walk(tb, ix, dir, Start::End, limit, size(r));
        while let Some((k, rid)) = got.last().cloned() {
            let more = walk(tb, ix, dir, Start::After(&k, rid), limit, size(r));
            if more.is_empty() {
                break;
            }
            got.extend(more);
        }
        assert!(got == want, "index {ix}, {dir:?}: the walk in pieces of {limit}");
        assert!(walk(tb, ix, dir, Start::End, 0, size(r)).is_empty(), "index {ix}, {dir:?}: a walk of no rows");
        // From a key: a row's, or a fresh one.
        for _ in 0..2 {
            let key: Vec<Mv> = match grouped.get(r.below(grouped.len() as u64 + 1) as usize) {
                Some(g) if r.below(2) == 0 => g.key.clone(),
                _ => on.iter().map(|&c| field_value(r, &s.cols[c])).collect(),
            };
            let e = index_key(tb, ix, &cells(&key));
            // The key's rows, then those of the keys past it in the walk's direction.
            let past = |g: &&Group| {
                let o = cmp(&g.key, &key);
                o.is_eq() || (o == Ordering::Greater) == (dir == Dir::Fwd)
            };
            let from = if dir == Dir::Fwd {
                walk_rows(grouped.iter().filter(past))
            } else {
                walk_rows(grouped.iter().rev().filter(past))
            };
            let limit = pick(r, &[1, 30, u64::MAX]);
            let got = walk(tb, ix, dir, Start::Key(&e), limit, size(r));
            assert!(got[..] == from[..from.len().min(limit as usize)], "index {ix}, {dir:?}: from {key:?}");
            if grouped.is_empty() {
                assert!(walk(tb, ix, dir, Start::After(&e, 1), limit, size(r)).is_empty(), "index {ix}: empty");
            }
            // The count of the key.
            let n = grouped.iter().find(|g| cmp(&g.key, &key).is_eq()).map_or(0, |g| g.rids.len() as i128);
            let mut scratch = VecScratch::new(FILLS[1]);
            assert_eq!(
                tb.count(&mut scratch, ix, &mut Src::new(&cells(&key))).unwrap(),
                n,
                "index {ix}: count {key:?}"
            );
        }
    }
    // Counts of present keys.
    let mut scratch = VecScratch::new(FILLS[0]);
    for i in 0..grouped.len().min(100) {
        let g = &grouped[if grouped.len() <= 100 { i } else { r.below(grouped.len() as u64) as usize }];
        let n = tb.count(&mut scratch, ix, &mut Src::new(&cells(&g.key))).unwrap();
        assert_eq!(n, g.rids.len() as i128, "index {ix}: count {:?}", g.key);
    }
    // Group counts, whole and in pieces.
    let want: Vec<(Vec<u8>, i128)> = grouped.iter().map(|g| (g.enc.clone(), g.rids.len() as i128)).collect();
    assert!(groups(tb, ix, None, u64::MAX, size(r)) == want, "index {ix}: group counts");
    let limit = 1 + r.below(64);
    let mut got = groups(tb, ix, None, limit, size(r));
    while let Some((k, _)) = got.last().cloned() {
        let more = groups(tb, ix, Some(&k), limit, size(r));
        if more.is_empty() {
            break;
        }
        got.extend(more);
    }
    assert!(got == want, "index {ix}: group counts in pieces of {limit}");
    // The smallest and largest keys whose first part is not null.
    let firsts: Vec<&Vec<u8>> = grouped.iter().filter(|g| g.key[0] != Mv::Null).map(|g| &g.enc).collect();
    let (min, buf) = grown(size(r), |out| tb.min_key(ix, out));
    assert_eq!(min.map(|n| buf[..n].to_vec()).as_ref(), firsts.first().copied(), "index {ix}: the smallest key");
    let (max, buf) = grown(size(r), |out| tb.max_key(ix, out));
    assert_eq!(max.map(|n| buf[..n].to_vec()).as_ref(), firsts.last().copied(), "index {ix}: the largest key");
}

// ---------------------------------------------------------------- the run

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Insert,
    Upsert,
    Update,
    Delete,
    Maintain,
}

/// The mix of operations in each part of a run: the table grows (the first half), churns (three tenths),
/// then shrinks.
const MIXES: [[(Op, u64); 5]; 3] = [
    [(Op::Insert, 65), (Op::Upsert, 5), (Op::Update, 12), (Op::Delete, 6), (Op::Maintain, 12)],
    [(Op::Insert, 30), (Op::Upsert, 10), (Op::Update, 20), (Op::Delete, 25), (Op::Maintain, 15)],
    [(Op::Insert, 10), (Op::Upsert, 5), (Op::Update, 15), (Op::Delete, 55), (Op::Maintain, 15)],
];

fn everything() -> TestBudget {
    TestBudget::new(u64::MAX)
}

fn maintain_all(tb: &mut Table<'_, VecHost>) {
    assert_eq!(tb.maintain(&everything()).unwrap(), Progress::Done);
    assert_eq!(tb.wake().unwrap(), Wake::Idle);
}

/// What a run saw, for comparing the runs under the two fill patterns.
#[derive(Debug, PartialEq, Eq)]
struct Digest {
    /// The key's type, the number of fields, and each index's number of fields.
    schema: (Kind, usize, Vec<usize>),
    inserts: u64,
    deletes: usize,
    refused: u64,
    /// The groups sealed before the table was emptied.
    sealed: u64,
    pages: usize,
    /// A hash of the row ids the inserts took.
    rids: u64,
}

/// A row of the schema under this key, its fields drawn.
fn row_of(s: &Schema, r: &mut XorShift, key: Mv) -> Vec<Mv> {
    std::iter::once(key).chain(s.cols[1..].iter().map(|f| field_value(r, f))).collect()
}

/// One table drawn from `seed`, `ops` writes, under fill pattern `fill`.
fn run(seed: u64, key: Kind, ops: u64, fill: u8) -> Digest {
    let mut r = rng(seed);
    let mut s = draw_schema(&mut r, key);
    // A wide row costs its fields on every read: fewer writes.
    let ops = if s.cols.len() > 20 { ops / 4 } else { ops };
    let mut h = VecHost::new(fill);
    h.set_logging(false);
    let (tb, tbl) = Table::create(&mut h, seed, &s.img).unwrap();
    let empty = held(&tb);
    let mut m = Model::new();
    let mut dead: Vec<u64> = Vec::new();
    check(&tb, &s, &m, &dead, &mut r);
    let mut scratch = VecScratch::new(fill);
    let mut rids = std::collections::hash_map::DefaultHasher::new();
    let mut last_rid: Option<u64> = None;
    let mut sealed = 0u64;
    let (mut inserts, mut refused) = (0u64, 0u64);
    let key_kind = s.cols[0].kind;
    // About every thousand writes, and no more than 25 times in a long run.
    let check_every = (700 + r.below(600)).max(ops / 25);
    // The row type changes 1 to 3 times, each before a write drawn from the run's.
    let mut changes: Vec<u64> = (0..1 + r.below(3)).map(|_| r.below(ops.max(1))).collect();
    changes.sort_unstable();
    changes.dedup();
    let mut i = 0u64;
    let (mut burst, mut hot): (u64, Vec<Key>) = (0, Vec::new());
    // The writes, then deletes of every row left.
    loop {
        let draining = i >= ops;
        if draining && m.is_empty() {
            break;
        }
        // A change of the row type, then every read against the model.
        if !draining && changes.first() == Some(&i) {
            changes.remove(0);
            draw_change(&mut r, &mut s, &mut m);
            let mut tb = Table::open(&mut h, tbl).unwrap();
            tb.evolve(&s.img).unwrap();
            assert_eq!(tb.shape().version, s.version);
            sealed = nt_testhost::open_group(tb.host(), tbl);
            check(&tb, &s, &m, &dead, &mut r);
        }
        let existing = |r: &mut XorShift, m: &Model| m.keys().nth(r.below(m.len() as u64) as usize).cloned().unwrap();
        // Now and then a burst of updates of a few rows with no maintenance, so that patches pile up.
        if burst == 0 && !draining && !m.is_empty() && r.below(3_000) == 0 {
            burst = 50 + r.below(500);
            hot = (0..1 + r.below(300)).map(|_| existing(&mut r, &m)).collect();
        }
        let op = if burst > 0 {
            burst -= 1;
            if r.below(4) == 0 {
                Op::Upsert
            } else {
                Op::Update
            }
        } else if draining {
            if r.below(10) == 0 {
                Op::Maintain
            } else {
                Op::Delete
            }
        } else {
            let part = match i * 10 / ops {
                0..=4 => 0,
                5..=7 => 1,
                _ => 2,
            };
            let op = weighted(&mut r, &MIXES[part]);
            if m.is_empty() && matches!(op, Op::Upsert | Op::Update | Op::Delete) {
                Op::Insert
            } else {
                op
            }
        };
        // The write, drawn from the model: a key (a hot one in a burst), and the row to put under it (none for
        // a delete).
        let existing = |r: &mut XorShift, m: &Model| {
            if burst > 0 {
                hot[r.below(hot.len() as u64) as usize].clone()
            } else {
                existing(r, m)
            }
        };
        let write: Option<(Key, Option<Vec<Mv>>)> = match op {
            Op::Insert => {
                let k = if r.below(4) == 0 { value(&mut r, key_kind) } else { fresh(&mut r, key_kind) };
                Some((Key(key_kind, k.clone()), Some(row_of(&s, &mut r, k))))
            }
            Op::Upsert => {
                let k = existing(&mut r, &m);
                let row = row_of(&s, &mut r, k.1.clone());
                Some((k, Some(row)))
            }
            Op::Update => {
                // The row as it is, a dropped field null as every write gives it, one field drawn anew.
                let k = existing(&mut r, &m);
                let mut row = m[&k].1.clone();
                for (v, f) in row.iter_mut().zip(&s.cols) {
                    if f.dropped != 0 {
                        *v = Mv::Null;
                    }
                }
                if r.below(10) != 0 {
                    let c = pick(&mut r, &s.fields());
                    row[c] = field_value(&mut r, &s.cols[c]);
                }
                Some((k, Some(row)))
            }
            Op::Delete if r.below(10) == 0 => Some((Key(key_kind, value(&mut r, key_kind)), None)),
            Op::Delete => Some((existing(&mut r, &m), None)),
            Op::Maintain => None,
        };
        let step_first = op != Op::Maintain && burst == 0 && r.below(8) != 0;
        // A Float key equal to a row's in the key order (−0.0 and 0.0, or two NaNs of one sign) but not in
        // its bits is refused, since a row's key never changes; the message traps, and the host is as before.
        if let Some((key, Some(row))) = &write {
            if let Some(e) = m.get(key).filter(|e| e.1[0] != row[0]) {
                let before = h.clone();
                let mut tb = Table::open(&mut h, tbl).unwrap();
                if step_first {
                    tb.write_step().unwrap();
                }
                let got = tb.upsert(&mut scratch, &mut Src::new(&row_cells(&s, row, &mut r)));
                assert_eq!(got.map(|x| x.0), Err(Fault::BadValue { leaf: 0 }), "upsert {key:?} over {:?}", e.1[0]);
                h = before;
                refused += 1;
                i += 1;
                continue;
            }
        }
        let mut tb = Table::open(&mut h, tbl).unwrap();
        if step_first {
            tb.write_step().unwrap();
        }
        match &write {
            Some((key, Some(row))) => {
                let cells = row_cells(&s, row, &mut r);
                let got = tb.upsert(&mut scratch, &mut Src::new(&cells));
                match (got, m.get(key)) {
                    (Ok((Upsert::Inserted(rid), _)), None) => {
                        assert!(last_rid.is_none_or(|l| rid > l), "row ids rise: {rid} after {last_rid:?}");
                        last_rid = Some(rid);
                        rid.hash(&mut rids);
                        inserts += 1;
                        m.insert(key.clone(), (rid, row.clone()));
                    }
                    (Ok((Upsert::Replaced(rid), _)), Some(e)) => {
                        assert_eq!(rid, e.0, "an upsert of {key:?} keeps its row id");
                        m.insert(key.clone(), (rid, row.clone()));
                    }
                    (got, e) => panic!("upsert {key:?}: {got:?}, the model holds {e:?}"),
                }
            }
            Some((key, None)) => {
                let (gone, _) = tb.remove(&mut scratch, &mut Src::new(&key_cells(&s, &key.1))).unwrap();
                let was = m.remove(key).map(|e| e.0);
                assert_eq!(gone, was, "remove {key:?}");
                dead.extend(gone);
            }
            None => match r.below(4) {
                0 => {
                    tb.write_step().unwrap();
                }
                1 => {
                    tb.maintain(&TestBudget::new(0)).unwrap();
                }
                2 => maintain_all(&mut tb),
                _ => {
                    for _ in 0..1 + r.below(40) {
                        if tb.step().unwrap() == Ran::Nothing {
                            break;
                        }
                    }
                }
            },
        }
        // The written key reads back as the model has it.
        if let Some((key, _)) = &write {
            let mut out = Rows::new(&tb, &s, &mut r);
            let found = tb.get(&mut scratch, &mut Src::new(&key_cells(&s, &key.1)), &mut out).unwrap();
            match m.get(key) {
                Some((rid, row)) => assert!(found && same(&s, &out.out, &[(*rid, expected(row))]), "read back {key:?}"),
                None => assert!(!found, "{key:?} is deleted"),
            }
            assert_eq!(tb.len().unwrap(), m.len() as u64);
        }
        let open = nt_testhost::open_group(tb.host(), tbl);
        if open != sealed || (i + 1) % check_every == 0 || (draining && m.is_empty()) {
            sealed = open;
            check(&tb, &s, &m, &dead, &mut r);
        }
        i += 1;
    }
    // Every row deleted and all maintenance run, the open group is filled with rows deleted as they go in,
    // until it is sealed and dropped; three times, the last two with the same rows from an empty open group.
    // The table then holds the image pages it was made with, and the blocks in use do not grow.
    let mut tb = Table::open(&mut h, tbl).unwrap();
    maintain_all(&mut tb);
    check(&tb, &s, &m, &dead, &mut r);
    let mut cycle = |tb: &mut Table<'_, VecHost>, mut r: XorShift| {
        let open = nt_testhost::open_group(tb.host(), tbl);
        while nt_testhost::open_group(tb.host(), tbl) == open {
            let k = value(&mut r, key_kind);
            let cells = row_cells(&s, &row_of(&s, &mut r, k.clone()), &mut r);
            tb.upsert(&mut scratch, &mut Src::new(&cells)).unwrap();
            tb.remove(&mut scratch, &mut Src::new(&key_cells(&s, &k))).unwrap();
        }
        maintain_all(tb);
        assert_eq!(tb.len().unwrap(), 0);
        held(tb)
    };
    let rows = r.next_u64() | 1;
    cycle(&mut tb, XorShift(rows));
    let first = cycle(&mut tb, XorShift(rows));
    let second = cycle(&mut tb, XorShift(rows));
    assert_eq!(first.2, empty.2, "image pages once every row is deleted");
    assert!(
        second.0 <= first.0 && second.1 <= first.1,
        "blocks in use after emptying twice: {first:?}, then {second:?}"
    );
    check_pages(&tb);
    Digest {
        schema: (key_kind, s.cols.len() - 1, s.indexes.iter().map(|x| x.1.len()).collect()),
        inserts,
        deletes: dead.len(),
        refused,
        sealed,
        pages: tb.host().live_pages(),
        rids: rids.finish(),
    }
}

#[test]
fn model() {
    let seed = env_u64("NT_MODEL_SEED", DEFAULT_SEED);
    let tables = env_u64("NT_MODEL_TABLES", KINDS.len() as u64);
    let ops = env_u64("NT_MODEL_OPS", 16_000);
    let mut r = rng(seed);
    for t in 0..tables {
        // `NT_MODEL_TABLE` runs only the table of that number, as a failure names it.
        let s = r.next_u64();
        if env_u64("NT_MODEL_TABLE", t) != t {
            continue;
        }
        let start = std::time::Instant::now();
        let d = run(s, KINDS[t as usize % KINDS.len()].0, ops, FILLS[(t % 2) as usize]);
        eprintln!("table {t} (seed {s:#x}): {d:?}, {:.1?}", start.elapsed());
    }
}

/// The two fill patterns give one table the same row ids, rows and pages.
#[test]
fn model_fills() {
    let seed = env_u64("NT_MODEL_SEED", DEFAULT_SEED) ^ 0xF111;
    let ops = env_u64("NT_MODEL_OPS", 16_000) / 4;
    assert_eq!(run(seed, Kind::Text, ops, FILLS[0]), run(seed, Kind::Text, ops, FILLS[1]));
}
