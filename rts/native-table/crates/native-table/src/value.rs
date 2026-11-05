//! The schema's type codes, skipping a type expression, and the incremental UTF-8 validator for text read or
//! copied in pieces.
//!
//! VE is schema-directed, little-endian, without padding; every length and count is canonical ULEB128.
//! `skip_ty` has no depth cap: it takes its frame stack from the caller's scratch and returns
//! `Fault::Scratch { need }` when a type is deeper than it allows.

use crate::fault::{bounds, corrupt, Corruption, Fault, ImageId};
use crate::mem::{rd_u16, rd_u32, rd_u8};

/// Type codes of the schema's type pool.
pub mod ty {
    pub const BOOL: u8 = 0x01;
    pub const U8: u8 = 0x02;
    pub const U16: u8 = 0x03;
    pub const U32: u8 = 0x04;
    pub const U64: u8 = 0x05;
    pub const I8: u8 = 0x06;
    pub const I16: u8 = 0x07;
    pub const I32: u8 = 0x08;
    pub const I64: u8 = 0x09;
    pub const NAT: u8 = 0x0A;
    pub const INT: u8 = 0x0B;
    pub const F32: u8 = 0x0C;
    pub const F64: u8 = 0x0D;
    pub const CHAR: u8 = 0x0E;
    pub const TEXT: u8 = 0x0F;
    pub const BLOB: u8 = 0x10;
    pub const PRINCIPAL: u8 = 0x11;
    pub const U128: u8 = 0x12;
    pub const I128: u8 = 0x13;
    /// Bytes(u16 N).
    pub const BYTES: u8 = 0x14;
    pub const UNIT: u8 = 0x15;
    /// Option(T).
    pub const OPTION: u8 = 0x20;
    /// Record(u16 n main fields, u16 a added fields, (n + a) × {u32 hash, u32 name offset, u8 flags, u32
    /// default offset, u32 default length, T}): a field's declared default is its VE in the default pool
    /// (length 0: none).
    pub const RECORD: u8 = 0x21;
    /// Bytes of a record field's entry before its type.
    pub const FIELD_HDR: u32 = 17;
    /// Tuple(u16 n, T…).
    pub const TUPLE: u8 = 0x22;
    /// A record field's flags: bit 0, dropped.
    pub const FIELD_DROPPED: u8 = 1;
}

// ---------------------------------------------------------------- incremental UTF-8

/// The incremental UTF-8 validator for text read or copied in pieces: up to 3 carried bytes plus the
/// number the carried character still needs. A character may straddle pieces; a truncated one at the
/// value's end is invalid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Utf8Stream {
    carry: [u8; 4],
    carried: u8,
}

impl Utf8Stream {
    pub const fn new() -> Self {
        Utf8Stream { carry: [0; 4], carried: 0 }
    }

    /// Feeds the next piece; `false` when the text is invalid.
    pub fn feed(&mut self, piece: &[u8]) -> bool {
        let mut rest = piece;
        // Complete the carried character first, byte by byte.
        while self.carried > 0 {
            let Some((&b, tail)) = rest.split_first() else { return true };
            rest = tail;
            let k = usize::from(self.carried);
            if let Some(slot) = self.carry.get_mut(k) {
                *slot = b;
            }
            self.carried = self.carried.wrapping_add(1);
            let have = self.carry.get(..usize::from(self.carried)).unwrap_or(&[]);
            match core::str::from_utf8(have) {
                Ok(_) => self.carried = 0,
                Err(e) if e.error_len().is_some() => return false,
                Err(_) => {
                    if self.carried >= 4 {
                        return false;
                    }
                }
            }
        }
        match core::str::from_utf8(rest) {
            Ok(_) => true,
            Err(e) if e.error_len().is_some() => false,
            Err(e) => {
                let tail = rest.get(e.valid_up_to()..).unwrap_or(&[]);
                if tail.len() > 3 {
                    return false;
                }
                for (slot, &b) in self.carry.iter_mut().zip(tail) {
                    *slot = b;
                }
                // tail.len() ≤ 3.
                #[allow(clippy::cast_possible_truncation)]
                let n = tail.len() as u8;
                self.carried = n;
                true
            }
        }
    }

    /// The value ended: valid only with nothing carried.
    pub fn finish(&self) -> bool {
        self.carried == 0
    }
}

// ---------------------------------------------------------------- skipping a type expression

/// Bytes of one frame of a walk over a type expression.
pub const FRAME: usize = 16;

/// The frame kind of `skip_ty`.
const F_SKIP_TY: u8 = 5;

struct Stack<'s> {
    buf: &'s mut [u8],
    top: usize,
}

impl Stack<'_> {
    fn push(&mut self, kind: u8, a: u32, b: u64) -> Result<(), Fault> {
        let end = self.top.saturating_add(FRAME);
        let f = match self.buf.get_mut(self.top..end) {
            Some(f) => f,
            None => return Err(Fault::Scratch { need: (end as u64).saturating_mul(2) }),
        };
        f.fill(0);
        if let Some(k) = f.first_mut() {
            *k = kind;
        }
        crate::mem::put(f, 4, &a.to_le_bytes())?;
        crate::mem::put(f, 8, &b.to_le_bytes())?;
        self.top = end;
        Ok(())
    }

    fn pop(&mut self) -> Result<Option<(u8, u32, u64)>, Fault> {
        if self.top == 0 {
            return Ok(None);
        }
        self.top = self.top.saturating_sub(FRAME);
        let f = self.buf.get(self.top..self.top.saturating_add(FRAME)).ok_or(bounds(0))?;
        let kind = rd_u8(f, 0)?;
        let a = rd_u32(f, 4)?;
        let b = crate::mem::rd_u64(f, 8)?;
        Ok(Some((kind, a, b)))
    }
}

/// The offset just past the type expression at `t` in `pool`.
pub fn skip_ty(pool: &[u8], t: u32, scratch: &mut [u8]) -> Result<u32, Fault> {
    let rt = |at: u32| corrupt(ImageId::Runtime, Corruption::Encoding, u64::from(at));
    let mut st = Stack { buf: scratch, top: 0 };
    let mut p = t;
    st.push(F_SKIP_TY, 0, 1)?;
    // Each frame counts sub-expressions still to skip; a record's fields carry a header.
    while let Some((_, hdr, left)) = st.pop()? {
        if left == 0 {
            continue;
        }
        st.push(F_SKIP_TY, hdr, left.wrapping_sub(1))?;
        p = p.checked_add(hdr).ok_or(rt(p))?;
        let code = rd_u8(pool, u64::from(p))?;
        p = p.checked_add(1).ok_or(rt(p))?;
        match code {
            ty::BOOL..=ty::I128 | ty::UNIT => {}
            ty::BYTES => p = p.checked_add(2).ok_or(rt(p))?,
            ty::OPTION => st.push(F_SKIP_TY, 0, 1)?,
            ty::TUPLE => {
                let n = rd_u16(pool, u64::from(p))?;
                p = p.checked_add(2).ok_or(rt(p))?;
                st.push(F_SKIP_TY, 0, u64::from(n))?;
            }
            ty::RECORD => {
                let n = u32::from(rd_u16(pool, u64::from(p))?);
                let a = u32::from(rd_u16(pool, u64::from(p).wrapping_add(2))?);
                p = p.checked_add(4).ok_or(rt(p))?;
                st.push(F_SKIP_TY, ty::FIELD_HDR, u64::from(n.wrapping_add(a)))?;
            }
            _ => return Err(rt(p.wrapping_sub(1))),
        }
    }
    Ok(p)
}
