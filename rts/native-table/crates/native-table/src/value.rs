//! The schema's type codes, and the incremental UTF-8 validator for text read or copied in pieces.
//!
//! VE is schema-directed, little-endian, without padding; every length and count is canonical ULEB128.

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
