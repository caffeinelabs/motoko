//! The key encoding E: the order of every index key is memcmp over
//! E(x), never a runtime's native order. Every E is prefix-free within its kind, so parts concatenate.
//! Integers are big-endian. Keys have no length cap.
//!
//! Encoders write through a [`Sink`]; the same encoder with a [`LenSink`] gives a key's length. Decoders
//! (E⁻¹) read stored key bytes, so every decode is checked and returns `Corrupt` on malformed bytes, never a
//! panic.

pub mod enc;
pub mod probe;
pub mod spec;
pub mod words;

pub use enc::*;
pub use spec::{KeyKind, KeyPart, KeySpec};

use crate::fault::{bounds, Fault};

/// Where encoded bytes go.
pub trait Sink {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault>;

    #[inline]
    fn put_byte(&mut self, b: u8) -> Result<(), Fault> {
        self.put(&[b])
    }

    /// Puts `bytes` with each `00` written `00 FF` (a piece of an escaped Text or Blob part): by default each
    /// run of bytes without `00` and each `00 FF` a put of its own (`escape_runs`).
    #[inline]
    fn put_escaped(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        escape_runs(self, bytes)
    }

    /// The next bytes whose values the sink keeps: a sink that keeps a prefix only counts the bytes past it,
    /// and an encoder may hand it those with `skip`, not computing them.
    #[inline]
    fn wants(&self) -> u64 {
        u64::MAX
    }

    /// Counts `n` more bytes without their values; an encoder calls it only when `wants` is 0.
    #[inline]
    fn skip(&mut self, n: u64) -> Result<(), Fault> {
        let _ = n;
        Ok(())
    }
}

/// Counts bytes only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LenSink(pub u64);

impl Sink for LenSink {
    #[inline]
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        // A slice's length fits u64, and no key reaches 2^64 bytes.
        self.0 = self.0.saturating_add(bytes.len() as u64);
        Ok(())
    }
}

/// Writes into a slice; a key longer than the slice is `Fault::Scratch { need }` with the length so far.
pub struct SliceSink<'a> {
    pub buf: &'a mut [u8],
    pub len: usize,
}

impl<'a> SliceSink<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        SliceSink { buf, len: 0 }
    }

    pub fn written(&self) -> &[u8] {
        self.buf.get(..self.len).unwrap_or(&[])
    }
}

impl Sink for SliceSink<'_> {
    #[inline]
    fn put(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        let end = self.len.checked_add(bytes.len()).ok_or(bounds(0))?;
        match self.buf.get_mut(self.len..end) {
            Some(d) => {
                crate::mem::copy_bytes(d, bytes);
                self.len = end;
                Ok(())
            }
            None => Err(Fault::Scratch { need: end as u64 }),
        }
    }

    /// Written straight into the slice when twice `bytes` fits what is left of it: 8 bytes at a time while they
    /// hold no `00` (`has_zero`), else a byte at a time; else a run at a time (`escape_runs`).
    fn put_escaped(&mut self, bytes: &[u8]) -> Result<(), Fault> {
        if bytes.len().saturating_mul(2) > self.buf.len().saturating_sub(self.len) {
            return escape_runs(self, bytes);
        }
        let mut j = self.len;
        let mut words = bytes.chunks_exact(8);
        for w in &mut words {
            let x = <[u8; 8]>::try_from(w).unwrap_or([0; 8]);
            let v = u64::from_le_bytes(x);
            if has_zero(v) {
                // Its 8 bytes escaped take at most 16, which the room checked above holds.
                let mut n = 0usize;
                if let Some(d) = self.buf.get_mut(j..j.wrapping_add(16)).and_then(|d| <&mut [u8; 16]>::try_from(d).ok())
                {
                    for k in 0..8u32 {
                        // The low 8 bits.
                        #[allow(clippy::cast_possible_truncation)]
                        let b = v.wrapping_shr(k.wrapping_mul(8)) as u8;
                        if let Some(c) = d.get_mut(n & 15) {
                            *c = b;
                        }
                        n = n.wrapping_add(1);
                        if b == 0 {
                            if let Some(c) = d.get_mut(n & 15) {
                                *c = 0xFF;
                            }
                            n = n.wrapping_add(1);
                        }
                    }
                }
                j = j.wrapping_add(n);
            } else {
                if let Some(d) = self.buf.get_mut(j..j.wrapping_add(8)).and_then(|d| <&mut [u8; 8]>::try_from(d).ok()) {
                    *d = x;
                }
                j = j.wrapping_add(8);
            }
        }
        for &b in words.remainder() {
            j = escape_byte(self.buf, j, b);
        }
        self.len = j;
        Ok(())
    }
}

/// Writes byte `b` escaped at `j` of `buf`, which has room for it; returns where the next byte goes.
#[inline]
fn escape_byte(buf: &mut [u8], j: usize, b: u8) -> usize {
    if let Some(d) = buf.get_mut(j) {
        *d = b;
    }
    if b != 0 {
        return j.wrapping_add(1);
    }
    if let Some(d) = buf.get_mut(j.wrapping_add(1)) {
        *d = 0xFF;
    }
    j.wrapping_add(2)
}
