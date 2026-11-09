//! Unsigned LEB128 over slices. Every function here returns `Corrupt(Runtime, Bounds)` for a range outside
//! the slice and never panics.

use crate::fault::{bounds, corrupt, Corruption, Fault, ImageId};
use crate::mem::put;

/// Bytes of the unsigned LEB128 form of `v`: 1 to 10.
#[inline(always)]
pub const fn uleb_len(v: u64) -> u64 {
    // leading_zeros ≤ 64: the value's bits, at least one.
    let bits = 64u64.wrapping_sub(v.leading_zeros() as u64);
    (if bits == 0 { 1 } else { bits }).wrapping_add(6) / 7
}

/// Writes `v` as unsigned LEB128 at `off`; returns the bytes written.
pub fn put_uleb(b: &mut [u8], off: u64, v: u64) -> Result<u64, Fault> {
    let mut buf = [0u8; 10];
    let mut x = v;
    let mut n = 0usize;
    for slot in buf.iter_mut() {
        // Low 7 bits of x.
        #[allow(clippy::cast_possible_truncation)]
        let low = (x & 0x7F) as u8;
        x >>= 7;
        n = n.saturating_add(1);
        if x == 0 {
            *slot = low;
            break;
        }
        *slot = low | 0x80;
    }
    let s = buf.get(..n).ok_or(bounds(off))?;
    put(b, off, s)?;
    Ok(n as u64)
}

/// Reads an unsigned LEB128 at `off`; returns (value, bytes read). A form longer than 10 bytes, or a 10th
/// byte above 1, is `Corrupt(image, Encoding)`; a form that runs past the slice is `Bounds`.
pub fn rd_uleb(b: &[u8], off: u64, image: ImageId) -> Result<(u64, u64), Fault> {
    let start = usize::try_from(off).map_err(|_| bounds(off))?;
    let tail = b.get(start..).ok_or(bounds(off))?;
    let mut v: u64 = 0;
    for (i, &byte) in tail.iter().take(10).enumerate() {
        let low = u64::from(byte & 0x7F);
        // i ≤ 9, so the shift is at most 63.
        let shift = u32::try_from(i).unwrap_or(0).wrapping_mul(7);
        if i == 9 && byte > 1 {
            return Err(corrupt(image, Corruption::Encoding, off));
        }
        v |= low << shift;
        if byte & 0x80 == 0 {
            return Ok((v, (i as u64).wrapping_add(1)));
        }
    }
    if tail.len() < 10 {
        Err(bounds(off))
    } else {
        Err(corrupt(image, Corruption::Encoding, off))
    }
}
