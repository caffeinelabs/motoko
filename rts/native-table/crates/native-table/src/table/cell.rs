//! Cells: a host's `CellIn` checked against its leaf's type into a `Val`, the bytes a value takes in a row
//! image, its key encoding E, and the canonical LEB128 forms of Nat and Int.

use super::layout::{LeafInfo, Ty};
use super::CellIn;
use crate::fault::Fault;
use crate::key::{self, Sink};

/// A checked value of one leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Val<'a> {
    Null,
    Bool(bool),
    /// Unsigned integers, Char (its scalar value), and a Nat below 2^64.
    U(u64),
    /// Signed integers, and an Int inside i64.
    I(i64),
    /// A Nat or Int beyond 64 bits that fits i128.
    Wide(i128),
    U128(u128),
    I128(i128),
    F32(u32),
    F64(u64),
    /// Text, Blob, Principal (at most 29 bytes) and `[u8; N]`.
    Bytes(&'a [u8]),
    /// A Nat or Int beyond i128: its canonical (signed) LEB128 form, a *big* value.
    Big(&'a [u8]),
    Unit,
    /// A Text or Blob of this many bytes still in the message's argument.
    Arg(u64),
}

impl Val<'_> {
    pub fn is_null(&self) -> bool {
        matches!(self, Val::Null)
    }
}

/// Whether `b` is a canonical (shortest) LEB128 form, signed or not, ending exactly at its last byte.
pub fn leb_canonical(b: &[u8], signed: bool) -> bool {
    let Some((&last, init)) = b.split_last() else { return false };
    if last & 0x80 != 0 || init.iter().any(|&x| x & 0x80 == 0) {
        return false;
    }
    match init.last() {
        None => true,
        Some(&prev) => {
            if signed {
                !((last == 0x00 && prev & 0x40 == 0) || (last == 0x7F && prev & 0x40 != 0))
            } else {
                last != 0
            }
        }
    }
}

/// The value of a canonical LEB128 form when it fits i128 (an unsigned form up to i128's maximum).
pub fn leb_value(b: &[u8], signed: bool) -> Option<i128> {
    let mut v: u128 = 0;
    let mut shift = 0u32;
    for &x in b {
        let low = u128::from(x & 0x7F);
        if shift >= 128 {
            if low != 0 && !(signed && low == 0x7F) {
                return None;
            }
        } else {
            let part = low << shift;
            if part >> shift != low {
                // Bits fell off the top.
                if !signed {
                    return None;
                }
            }
            v |= part;
        }
        shift = shift.saturating_add(7);
    }
    if signed {
        let last = b.last().copied().unwrap_or(0);
        if shift < 128 && last & 0x40 != 0 {
            v |= u128::MAX << shift;
        }
        let x = v as i128;
        // Check the round trip: the value's own canonical form must be these bytes.
        let mut buf = [0u8; 19];
        let n = put_sleb128(&mut buf, x);
        if buf.get(..n) == Some(b) {
            Some(x)
        } else {
            None
        }
    } else if v > i128::MAX as u128 {
        None
    } else {
        Some(v as i128)
    }
}

/// Writes x as canonical unsigned LEB128 into `out` (at least 19 bytes); returns the length.
pub fn put_uleb128(out: &mut [u8], x: u128) -> usize {
    let mut v = x;
    let mut n = 0usize;
    loop {
        // The low 7 bits.
        #[allow(clippy::cast_possible_truncation)]
        let low = (v & 0x7F) as u8;
        v >>= 7;
        let done = v == 0;
        if let Some(slot) = out.get_mut(n) {
            *slot = if done { low } else { low | 0x80 };
        }
        n = n.saturating_add(1);
        if done {
            return n;
        }
    }
}

/// Writes x as canonical signed LEB128 into `out` (at least 19 bytes); returns the length.
pub fn put_sleb128(out: &mut [u8], x: i128) -> usize {
    let mut v = x;
    let mut n = 0usize;
    loop {
        // The low 7 bits.
        #[allow(clippy::cast_possible_truncation)]
        let low = (v & 0x7F) as u8;
        v >>= 7;
        let done = (v == 0 && low & 0x40 == 0) || (v == -1 && low & 0x40 != 0);
        if let Some(slot) = out.get_mut(n) {
            *slot = if done { low } else { low | 0x80 };
        }
        n = n.saturating_add(1);
        if done {
            return n;
        }
    }
}

/// The LEB128 form of a Nat or Int value that is not big, into `out` (19 bytes); returns its length.
pub fn small_leb(v: &Val<'_>, signed: bool, out: &mut [u8; 19]) -> usize {
    let x: i128 = match *v {
        Val::U(x) => i128::from(x),
        Val::I(x) => i128::from(x),
        Val::Wide(x) => x,
        _ => 0,
    };
    if signed {
        put_sleb128(out, x)
    } else {
        put_uleb128(out, x as u128)
    }
}

/// Checks a host cell against leaf `l` (its type and its nullability) and returns the value, or
/// `BadValue { leaf: l.id }`. Values are never refused for their size.
pub fn check<'a>(l: &LeafInfo, c: CellIn<'a>) -> Result<Val<'a>, Fault> {
    let bad = Fault::BadValue { leaf: l.id };
    let fits = |x: u64, w: u8| w >= 8 || x >> (u32::from(w).wrapping_mul(8)) == 0;
    let fits_i = |x: i64, w: u8| {
        let bits = u32::from(w).wrapping_mul(8);
        bits >= 64 || (x.wrapping_shl(64u32.wrapping_sub(bits))).wrapping_shr(64u32.wrapping_sub(bits)) == x
    };
    let v = match (l.ty, c) {
        (_, CellIn::Null) => {
            if !l.nullable {
                return Err(bad);
            }
            Val::Null
        }
        (Ty::Bool, CellIn::Bool(b)) => Val::Bool(b),
        (Ty::U(w), CellIn::U64(x)) if fits(x, w) => Val::U(x),
        (Ty::I(w), CellIn::I64(x)) if fits_i(x, w) => Val::I(x),
        (Ty::Char, CellIn::U64(x)) => {
            let c = u32::try_from(x).ok().and_then(char::from_u32).ok_or(bad)?;
            Val::U(u64::from(u32::from(c)))
        }
        (Ty::Nat, CellIn::U64(x)) => Val::U(x),
        (Ty::Int, CellIn::I64(x)) => Val::I(x),
        (Ty::Int, CellIn::U64(x)) => match i64::try_from(x) {
            Ok(y) => Val::I(y),
            Err(_) => Val::Wide(i128::from(x)),
        },
        (Ty::Nat, CellIn::Nat(b)) => {
            if !leb_canonical(b, false) {
                return Err(bad);
            }
            match leb_value(b, false) {
                Some(x) => match u64::try_from(x) {
                    Ok(y) => Val::U(y),
                    Err(_) => Val::Wide(x),
                },
                None => Val::Big(b),
            }
        }
        (Ty::Int, CellIn::Int(b)) => {
            if !leb_canonical(b, true) {
                return Err(bad);
            }
            match leb_value(b, true) {
                Some(x) => match i64::try_from(x) {
                    Ok(y) => Val::I(y),
                    Err(_) => Val::Wide(x),
                },
                None => Val::Big(b),
            }
        }
        (Ty::F32, CellIn::F32(f)) => Val::F32(f.to_bits()),
        (Ty::F64, CellIn::F64(f)) => Val::F64(f.to_bits()),
        (Ty::U128, CellIn::U128(x)) => Val::U128(x),
        (Ty::I128, CellIn::I128(x)) => Val::I128(x),
        (Ty::Text, CellIn::Bytes(b)) => {
            if core::str::from_utf8(b).is_err() {
                return Err(bad);
            }
            Val::Bytes(b)
        }
        (Ty::Blob, CellIn::Bytes(b)) => Val::Bytes(b),
        (Ty::Principal, CellIn::Bytes(b)) if b.len() <= 29 => Val::Bytes(b),
        (Ty::Fixed(n), CellIn::Bytes(b)) if b.len() == usize::from(n) => Val::Bytes(b),
        (Ty::Text | Ty::Blob, CellIn::Arg { len }) => Val::Arg(len),
        (Ty::Unit, CellIn::Unit) => Val::Unit,
        _ => return Err(bad),
    };
    Ok(v)
}

/// Bytes of a variable leaf's value: Text and Blob bytes, a Nat's or Int's LEB128 form; 0 for null.
pub fn var_len(v: &Val<'_>, t: Ty) -> u64 {
    match *v {
        Val::Bytes(b) => b.len() as u64,
        Val::Arg(n) => n,
        Val::Big(b) => b.len() as u64,
        Val::U(_) | Val::I(_) | Val::Wide(_) => {
            let mut buf = [0u8; 19];
            small_leb(v, matches!(t, Ty::Int), &mut buf) as u64
        }
        _ => 0,
    }
}

/// A Nat or Int value that does not fit i128: it is a variable item of its row image.
pub fn is_big(v: &Val<'_>) -> bool {
    matches!(v, Val::Big(_))
}

/// Writes the fixed-region cell of `v` (a leaf of fixed width at most 30 bytes) into `out`, zero-filled;
/// returns the width. A null, and a big Nat or Int, write zeros. `[u8; N]` cells wider than 30 bytes are
/// written by the caller from the value's bytes.
pub fn fixed_cell(v: &Val<'_>, t: Ty, out: &mut [u8; 30]) -> u64 {
    out.fill(0);
    let w = t.fixed_width();
    let mut put = |b: &[u8]| crate::mem::copy_bytes(out, b);
    match (*v, t) {
        (Val::Null, _) | (Val::Big(_), _) => {}
        (Val::Bool(b), _) => put(&[u8::from(b)]),
        (Val::U(x), Ty::Nat) => put(&i128::from(x).to_le_bytes()),
        (Val::I(x), Ty::Int) => put(&i128::from(x).to_le_bytes()),
        (Val::Wide(x), _) => put(&x.to_le_bytes()),
        (Val::U(x), _) => put(&x.to_le_bytes()),
        (Val::I(x), _) => put(&x.to_le_bytes()),
        (Val::U128(x), _) => put(&x.to_le_bytes()),
        (Val::I128(x), _) => put(&x.to_le_bytes()),
        (Val::F32(b), _) => put(&b.to_le_bytes()),
        (Val::F64(b), _) => put(&b.to_le_bytes()),
        (Val::Bytes(b), Ty::Principal) => {
            put(b);
            // A principal's cell: its bytes zero-padded to 29, then its length (at most 29).
            #[allow(clippy::cast_possible_truncation)]
            if let Some(last) = out.last_mut() {
                *last = b.len() as u8;
            }
        }
        (Val::Bytes(b), _) => put(b),
        _ => {}
    }
    w
}

/// The value of a fixed-region cell of leaf type `t` (not for Text, Blob or `[u8; N]` wider than 30, and not
/// for a null or big value, which the bitmaps tell).
pub fn from_fixed(t: Ty, cell: &[u8]) -> Val<'_> {
    let le = |n: usize| -> [u8; 16] {
        let mut a = [0u8; 16];
        crate::mem::copy_bytes(&mut a, cell.get(..n).unwrap_or(&[]));
        a
    };
    match t {
        Ty::Bool => Val::Bool(cell.first().copied().unwrap_or(0) != 0),
        Ty::U(w) => Val::U(u64::from_le_bytes(first8(&le(usize::from(w))))),
        Ty::I(w) => {
            let raw = u64::from_le_bytes(first8(&le(usize::from(w))));
            let bits = u32::from(w).wrapping_mul(8);
            let sh = 64u32.saturating_sub(bits);
            let x = (raw.wrapping_shl(sh) as i64).wrapping_shr(sh);
            Val::I(x)
        }
        Ty::Char | Ty::F32 => {
            let x = u32::from_le_bytes(first4(&le(4)));
            match t {
                Ty::Char => Val::U(u64::from(x)),
                _ => Val::F32(x),
            }
        }
        Ty::F64 => Val::F64(u64::from_le_bytes(first8(&le(8)))),
        Ty::Nat => {
            let x = i128::from_le_bytes(le(16));
            match u64::try_from(x) {
                Ok(y) => Val::U(y),
                Err(_) => Val::Wide(x),
            }
        }
        Ty::Int => {
            let x = i128::from_le_bytes(le(16));
            match i64::try_from(x) {
                Ok(y) => Val::I(y),
                Err(_) => Val::Wide(x),
            }
        }
        Ty::U128 => Val::U128(u128::from_le_bytes(le(16))),
        Ty::I128 => Val::I128(i128::from_le_bytes(le(16))),
        Ty::Principal => {
            let n = usize::from(cell.get(29).copied().unwrap_or(0)).min(29);
            Val::Bytes(cell.get(..n).unwrap_or(&[]))
        }
        Ty::Fixed(n) => Val::Bytes(cell.get(..usize::from(n)).unwrap_or(&[])),
        Ty::Text | Ty::Blob | Ty::Unit | Ty::Dropped => Val::Unit,
    }
}

fn first8(a: &[u8; 16]) -> [u8; 8] {
    let mut b = [0u8; 8];
    crate::mem::copy_bytes(&mut b, a);
    b
}

fn first4(a: &[u8; 16]) -> [u8; 4] {
    let mut b = [0u8; 4];
    crate::mem::copy_bytes(&mut b, a);
    b
}

/// Encodes key leaf `l`'s value as E (with the null tag of a nullable part) into `s`. Text and Blob values
/// given as argument values cannot be keys: `BadValue`.
pub fn enc_key<S: Sink>(s: &mut S, l: &LeafInfo, v: &Val<'_>) -> Result<(), Fault> {
    let bad = Fault::BadValue { leaf: l.id };
    if l.nullable {
        if v.is_null() {
            return key::enc_opt_tag(s, false);
        }
        key::enc_opt_tag(s, true)?;
    }
    let r = match (*v, l.ty) {
        (Val::Bool(b), _) => key::enc_bool(s, b),
        (Val::U(x), Ty::U(w)) => key::enc_u64(s, x, w),
        (Val::I(x), Ty::I(w)) => key::enc_i64(s, x, w),
        (Val::U(x), Ty::Char) => s.put(&u32::try_from(x).unwrap_or(0).to_be_bytes()),
        (Val::U(x), Ty::Nat) => key::enc_nat_u64(s, x),
        (Val::I(x), Ty::Int) => key::enc_int_i64(s, x),
        (Val::Wide(x), _) => key::enc_int_be(s, x < 0, &x.unsigned_abs().to_be_bytes()),
        (Val::Big(b), Ty::Nat) => key::enc_int_leb(s, b, false, crate::fault::ImageId::Runtime),
        (Val::Big(b), Ty::Int) => key::enc_int_leb(s, b, true, crate::fault::ImageId::Runtime),
        (Val::F32(b), _) => key::enc_f32(s, b),
        (Val::F64(b), _) => key::enc_f64(s, b),
        (Val::U128(x), _) => key::enc_u128(s, x),
        (Val::I128(x), _) => key::enc_i128(s, x),
        (Val::Bytes(b), Ty::Text | Ty::Blob) => key::enc_bytes(s, b),
        (Val::Bytes(b), Ty::Principal) => key::enc_principal(s, b),
        (Val::Bytes(b), Ty::Fixed(_)) => key::enc_fixed(s, b),
        (Val::Unit, _) => Ok(()),
        _ => return Err(bad),
    };
    r.map_err(|f| if f == (Fault::BadValue { leaf: key::NO_LEAF }) { bad } else { f })
}
