//! Key kinds and the key spec of an index.

use super::*;
use crate::consts::FORMAT_KEY;
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::hash::Hasher64;

/// One key kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Char,
    /// Nat of any magnitude.
    Nat,
    /// Int of any magnitude.
    Int,
    F64,
    F32,
    Text,
    Blob,
    Principal,
    /// `[u8; N]`.
    Fixed(u16),
    Unit,
}

impl KeyKind {
    /// The default width in W.
    pub const fn default_width(self) -> u64 {
        match self {
            KeyKind::Bool | KeyKind::U8 | KeyKind::I8 => 1,
            KeyKind::U16 | KeyKind::I16 => 2,
            KeyKind::U32 | KeyKind::I32 | KeyKind::Char | KeyKind::F32 => 4,
            KeyKind::U64 | KeyKind::I64 | KeyKind::F64 => 8,
            KeyKind::U128 | KeyKind::I128 | KeyKind::Nat | KeyKind::Int => 16,
            KeyKind::Text | KeyKind::Blob => 32,
            KeyKind::Principal => 30,
            KeyKind::Fixed(n) => n as u64,
            KeyKind::Unit => 0,
        }
    }

    /// The position just past this kind's encoding at `pos`, checked like E⁻¹.
    pub fn skip(self, key: &[u8], pos: u64, image: ImageId) -> Result<u64, Fault> {
        let fixed = |n: u64| -> Result<u64, Fault> {
            let end = pos.checked_add(n).ok_or(corrupt(image, Corruption::Encoding, pos))?;
            if end > key.len() as u64 {
                return Err(corrupt(image, Corruption::Encoding, pos));
            }
            Ok(end)
        };
        match self {
            KeyKind::Bool => dec_bool(key, pos, image).map(|(_, p)| p),
            KeyKind::Char => dec_char(key, pos, image).map(|(_, p)| p),
            KeyKind::Nat | KeyKind::Int => {
                let (_, n, at) = int_header(key, pos, image)?;
                let end = at.checked_add(n).ok_or(corrupt(image, Corruption::Encoding, at))?;
                if end > key.len() as u64 {
                    return Err(corrupt(image, Corruption::Encoding, at));
                }
                Ok(end)
            }
            KeyKind::Text | KeyKind::Blob => dec_bytes(key, pos, &mut LenSink(0), image),
            KeyKind::Principal => dec_principal(key, pos, image).map(|(_, _, p)| p),
            k => fixed(k.default_width()),
        }
    }
}

/// One part of an index's key: a leaf, its kind, and whether it is nullable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPart {
    pub leaf: u32,
    pub kind: KeyKind,
    pub nullable: bool,
}

/// The key spec of an index: its parts.
#[derive(Clone, Copy, Debug)]
pub struct KeySpec<'a> {
    pub parts: &'a [KeyPart],
}

impl KeySpec<'_> {
    /// The length every key of this spec encodes to, when every part has one width (no nullable part, none
    /// of a kind whose encoding varies in length).
    pub fn fixed_len(&self) -> Option<u64> {
        let mut n = 0u64;
        for p in self.parts {
            let varies =
                matches!(p.kind, KeyKind::Nat | KeyKind::Int | KeyKind::Text | KeyKind::Blob | KeyKind::Principal);
            if p.nullable || varies {
                return None;
            }
            n = n.saturating_add(p.kind.default_width());
        }
        Some(n)
    }

    /// The key spec hash: `nt_hash64` over {index kind u8; `FORMAT_KEY` u16; parts u16; per part: leaf u32,
    /// flags u8 (bit 0 nullable, the other bits 0)}, little-endian. Widths are per structure, so W is not
    /// part of it.
    pub fn hash(&self, index_kind: u8) -> u64 {
        let n = self.parts.len();
        let len = 5u64.saturating_add((n as u64).saturating_mul(5));
        let mut h = Hasher64::new(len);
        h.update(&[index_kind]);
        h.update(&FORMAT_KEY.to_le_bytes());
        h.update(&u16::try_from(n).unwrap_or(u16::MAX).to_le_bytes());
        for p in self.parts {
            h.update(&p.leaf.to_le_bytes());
            h.update(&[u8::from(p.nullable)]);
        }
        h.finish()
    }

    /// The end of the key `key` starts with: walks the parts with E⁻¹'s checks. The bytes after it (a key area's
    /// zero padding) are not read.
    pub fn end(&self, key: &[u8], image: ImageId) -> Result<u64, Fault> {
        let mut pos = 0u64;
        for p in self.parts {
            if p.nullable {
                let (some, next) = dec_bool(key, pos, image)?;
                pos = next;
                if !some {
                    continue;
                }
            }
            pos = p.kind.skip(key, pos, image)?;
        }
        Ok(pos)
    }
}
