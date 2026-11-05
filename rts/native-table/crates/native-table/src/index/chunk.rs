//! The value part of a run's row entry or tombstone: a chunk of one key's rids.
//!
//! Byte 0 is the chunk's kind.
//!
//! A **posting chunk** (kind 0) holds up to 256 rids r_0 ≤ … ≤ r_m of one key; the entry's field holds the
//! last, r_m, and its tombstone bit says whether they are rows or tombstones. Byte 1 is m (0–255), byte 2 is
//! b (bits per gap code, 0–62), and from byte 3 the m gap codes g_k = r_k − r_(k−1), b bits each, packed
//! least significant bit first: code k occupies bits [(k − 1)·b, k·b). The rest is zero. A chunk fits a
//! value part of V bytes when 3 + ⌈m·b / 8⌉ ≤ V. A rid may repeat (a gap of 0): a position held twice.
//! Decoding runs in either direction at O(1) per rid with no buffer: forward from r_0 = r_m − Σ g_k, backward
//! from r_m.
//!
//! A **key run** (kind 1) holds n items of n consecutive keys of an index whose keys end in an integer of a
//! fixed length L ≤ 8 bytes: item k's key is the entry's key with its last L bytes, read as a big-endian
//! integer, lowered by n − 1 − k, and its rid is the entry's rid lowered by n − 1 − k. Byte 1 is L (1–8),
//! bytes 2–5 are n − 1 (u32 little-endian), the rest is zero. Key runs hold rows only and need V ≥ 8.
//!
//! **Greedy chunking** is deterministic: a key's items, ascending by rid, open a chunk with the first; each
//! next item joins the open chunk when it has the same sign (row or tombstone), m + 1 ≤ 255, and
//! 3 + ⌈(m + 1)·max(b, bits(g)) / 8⌉ ≤ V, where g is its gap and bits(0) = 0; otherwise the open chunk is
//! closed and the item opens the next. With V = 0 every item is an entry of its own.
//!
//! Every reader checks the header before it decodes: a bad kind, b, size or L is `Corrupt`, never a panic.

use crate::consts::CHUNK_RIDS_MAX;
use crate::fault::{corrupt, Corruption, Fault, ImageId};

/// Kind byte of a posting chunk.
pub const POSTING: u8 = 0;
/// Kind byte of a key run.
pub const KEY_RUN: u8 = 1;
/// The most items one key run holds.
pub const KEY_RUN_MAX: u64 = 1 << 32;
/// The largest gap-code width.
pub const B_MAX: u8 = 62;
/// Every decoded absolute rid is below this.
pub const RID_LIMIT: u64 = 1 << 62;

/// A checked chunk header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chunk {
    /// m codes of b bits.
    Posting { m: u8, b: u8 },
    /// L key bytes stepping; n − 1.
    KeyRun { l: u8, n_minus_1: u32 },
}

impl Chunk {
    /// Items the chunk holds: m + 1, or n.
    pub fn items(self) -> u64 {
        match self {
            Chunk::Posting { m, .. } => u64::from(m).wrapping_add(1),
            Chunk::KeyRun { n_minus_1, .. } => u64::from(n_minus_1).wrapping_add(1),
        }
    }
}

/// Bits of x: 0 for 0, else the position of its highest set bit plus 1.
#[inline]
pub const fn bits(x: u64) -> u8 {
    // leading_zeros ≤ 64.
    #[allow(clippy::cast_possible_truncation)]
    let b = 64u32.wrapping_sub(x.leading_zeros()) as u8;
    b
}

/// Whether m codes of b bits fit a value part of v bytes: 3 + ⌈m·b / 8⌉ ≤ v.
#[inline]
pub const fn posting_fits(m: u64, b: u64, v: u64) -> bool {
    // m ≤ 255 and b ≤ 64 in every caller; checked anyway.
    let Some(bitsum) = m.checked_mul(b) else { return false };
    let Some(bytes) = bitsum.checked_add(7) else { return false };
    let Some(need) = (bytes / 8).checked_add(3) else { return false };
    need <= v
}

/// The header of the value part `value` (exactly V bytes). `image` and `at` name where it was read.
pub fn header(value: &[u8], image: ImageId, at: u64) -> Result<Chunk, Fault> {
    let enc = corrupt(image, Corruption::Encoding, at);
    let bounds = corrupt(image, Corruption::Bounds, at);
    let kind = *value.first().ok_or(bounds)?;
    let v = u64::try_from(value.len()).map_err(|_| bounds)?;
    match kind {
        POSTING => {
            let m = *value.get(1).ok_or(bounds)?;
            let b = *value.get(2).ok_or(bounds)?;
            if b > B_MAX {
                return Err(enc);
            }
            if !posting_fits(u64::from(m), u64::from(b), v) {
                return Err(bounds);
            }
            Ok(Chunk::Posting { m, b })
        }
        KEY_RUN => {
            let l = *value.get(1).ok_or(bounds)?;
            if l == 0 || l > 8 {
                return Err(enc);
            }
            let n = value.get(2..6).and_then(|s| s.first_chunk::<4>()).ok_or(bounds)?;
            Ok(Chunk::KeyRun { l, n_minus_1: u32::from_le_bytes(*n) })
        }
        _ => Err(enc),
    }
}

/// Code k (1 ≤ k ≤ m) of a posting chunk whose header was checked: b bits at bit (k − 1)·b of the bytes from
/// offset 3, least significant bit first.
pub fn code(value: &[u8], m: u8, b: u8, k: u8, image: ImageId, at: u64) -> Result<u64, Fault> {
    if k == 0 || k > m || b > B_MAX {
        return Err(corrupt(image, Corruption::Range, at));
    }
    if b == 0 {
        return Ok(0);
    }
    // (k − 1)·b ≤ 254 · 62.
    let bit = u64::from(k.wrapping_sub(1)).wrapping_mul(u64::from(b));
    let byte = usize::try_from(bit / 8).map_err(|_| corrupt(image, Corruption::Bounds, at))?.wrapping_add(3);
    let shift = u32::try_from(bit % 8).unwrap_or(0);
    // b + 7 ≤ 69 bits span at most 9 bytes; bytes past the value read as zero (the header check keeps every
    // code inside the value).
    let mut a = [0u8; 16];
    for (j, slot) in a.iter_mut().enumerate().take(9) {
        if let Some(&x) = value.get(byte.wrapping_add(j)) {
            *slot = x;
        }
    }
    let word = u128::from_le_bytes(a).wrapping_shr(shift);
    let mask = 1u64.checked_shl(u32::from(b)).map_or(u64::MAX, |x| x.wrapping_sub(1));
    // The low 64 bits of word.
    #[allow(clippy::cast_possible_truncation)]
    let low = word as u64;
    Ok(low & mask)
}

/// The first rid of a posting chunk: last − Σ codes, by checked subtraction.
pub fn first(value: &[u8], m: u8, b: u8, last: u64, image: ImageId, at: u64) -> Result<u64, Fault> {
    let range = corrupt(image, Corruption::Range, at);
    let mut span = 0u64;
    let mut k = 1u8;
    while k <= m {
        span = span.checked_add(code(value, m, b, k, image, at)?).ok_or(range)?;
        if k == u8::MAX {
            break;
        }
        k = k.wrapping_add(1);
    }
    last.checked_sub(span).ok_or(range)
}

/// A stored rid plus the run's rid base, checked: below 2^62.
pub fn abs_rid(stored: u64, base: u64, image: ImageId, at: u64) -> Result<u64, Fault> {
    match stored.checked_add(base) {
        Some(r) if r < RID_LIMIT => Ok(r),
        _ => Err(corrupt(image, Corruption::Range, at)),
    }
}

/// Writes the posting chunk of `rids` (r_0 ≤ … ≤ r_m, 1 to 256 of them) into `out`, the V-byte value part.
pub fn encode_posting(rids: &[u64], out: &mut [u8]) -> Result<(), Fault> {
    encode_posting_of(rids.iter().copied(), out)
}

/// `encode_posting` of the rids an iterator gives (it is gone over twice).
fn encode_posting_of<I: Iterator<Item = u64> + Clone>(rids: I, out: &mut [u8]) -> Result<(), Fault> {
    let bad = corrupt(ImageId::Runtime, Corruption::Encoding, 0);
    let mut count = 0u64;
    let mut b = 0u8;
    let mut prev = 0u64;
    for r in rids.clone() {
        if count > 0 {
            b = b.max(bits(r.checked_sub(prev).ok_or(bad)?));
        }
        prev = r;
        count = count.wrapping_add(1);
    }
    if count == 0 || count > CHUNK_RIDS_MAX {
        return Err(bad);
    }
    let m = count.wrapping_sub(1);
    let v = u64::try_from(out.len()).map_err(|_| bad)?;
    if b > B_MAX || !posting_fits(m, u64::from(b), v) {
        return Err(bad);
    }
    out.fill(0);
    let m8 = u8::try_from(m).map_err(|_| bad)?;
    *out.get_mut(0).ok_or(bad)? = POSTING;
    *out.get_mut(1).ok_or(bad)? = m8;
    *out.get_mut(2).ok_or(bad)? = b;
    // The codes, b bits each, least significant bit first, packed from byte 3 on through a 64-bit accumulator
    // that is written 8 bytes at a time when full (those bytes are codes', so they lie inside the value
    // part), the last partial bytes at the end.
    let mut acc = 0u64;
    let mut pending = 0u32;
    let mut at = 3usize;
    let mut it = rids;
    let mut prev = it.next().unwrap_or(0);
    for r in it {
        // r ≥ prev: checked above; the code has at most b ≤ 62 bits, and pending < 64.
        let code = r.wrapping_sub(prev);
        prev = r;
        acc |= code.wrapping_shl(pending);
        let total = pending.wrapping_add(u32::from(b));
        if total >= 64 {
            let w = out.get_mut(at..).and_then(|x| x.first_chunk_mut::<8>()).ok_or(bad)?;
            *w = acc.to_le_bytes();
            at = at.wrapping_add(8);
            // pending ≥ 2 here (b ≤ 62): the code's bits past the word stay pending.
            acc = if pending == 0 { 0 } else { code.wrapping_shr(64u32.wrapping_sub(pending)) };
            pending = total.wrapping_sub(64);
        } else {
            pending = total;
        }
    }
    let mut k = 0u32;
    while k < pending {
        // The accumulator's next byte.
        #[allow(clippy::cast_possible_truncation)]
        let byte = acc.wrapping_shr(k) as u8;
        *out.get_mut(at).ok_or(bad)? = byte;
        at = at.wrapping_add(1);
        k = k.wrapping_add(8);
    }
    Ok(())
}

/// Writes the key-run header of `n` items with an L-byte key field into `out`, the V-byte value part.
pub fn encode_key_run(l: u8, n: u64, out: &mut [u8]) -> Result<(), Fault> {
    let bad = corrupt(ImageId::Runtime, Corruption::Encoding, 0);
    if l == 0 || l > 8 || n == 0 || n > KEY_RUN_MAX || out.len() < 8 {
        return Err(bad);
    }
    out.fill(0);
    *out.get_mut(0).ok_or(bad)? = KEY_RUN;
    *out.get_mut(1).ok_or(bad)? = l;
    let nm1 = u32::try_from(n.wrapping_sub(1)).map_err(|_| bad)?;
    crate::mem::put_u32(out, 2, nm1)
}

/// Item k (0 ≤ k < n) of a key run whose last item has key field `last_field` and rid `last_rid`: its key
/// field and its rid.
pub fn key_run_item(
    last_field: u64,
    last_rid: u64,
    n_minus_1: u32,
    k: u32,
    image: ImageId,
    at: u64,
) -> Result<(u64, u64), Fault> {
    let range = corrupt(image, Corruption::Range, at);
    let back = u64::from(n_minus_1.checked_sub(k).ok_or(range)?);
    Ok((last_field.checked_sub(back).ok_or(range)?, last_rid.checked_sub(back).ok_or(range)?))
}

/// The key field of a key-run key: its last L bytes as a big-endian integer; `None` when the key is shorter
/// than L.
pub fn key_field(key: &[u8], l: u8) -> Option<u64> {
    let lu = usize::from(l);
    if lu == 0 || lu > 8 {
        return None;
    }
    let start = key.len().checked_sub(lu)?;
    let mut f = 0u64;
    for &x in key.get(start..)? {
        f = f.wrapping_shl(8) | u64::from(x);
    }
    Some(f)
}

/// Whether `next` follows `prev` in a key run of field length L: equal lengths, equal bytes before the
/// field, and the field one above, with no carry out of its L bytes.
pub fn key_run_step(prev: &[u8], next: &[u8], l: u8) -> bool {
    if prev.len() != next.len() {
        return false;
    }
    let (Some(fp), Some(fnx)) = (key_field(prev, l), key_field(next, l)) else { return false };
    let lu = usize::from(l);
    let head = prev.len().wrapping_sub(lu);
    if prev.get(..head) != next.get(..head) {
        return false;
    }
    fp.checked_add(1) == Some(fnx)
}

/// Greedy chunking at one value-part size: decides where a key's chunks end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chunker {
    pub v: u64,
    pub open: bool,
    pub tomb: bool,
    pub m: u64,
    pub b: u8,
    pub last: u64,
}

impl Chunker {
    pub const fn new(v: u64) -> Chunker {
        Chunker { v, open: false, tomb: false, m: 0, b: 0, last: 0 }
    }

    /// Offers the key's next item (rids ascending). True when it joined the open chunk; false when it opened
    /// a new chunk, the open one (if any) being closed before it.
    pub fn offer(&mut self, rid: u64, tomb: bool) -> bool {
        if self.open && self.v > 0 && self.tomb == tomb {
            if let Some(g) = rid.checked_sub(self.last) {
                let nb = self.b.max(bits(g));
                let nm = self.m.wrapping_add(1);
                // nm < 256 and nb ≤ 62 here: 3 + ⌈nm·nb / 8⌉ needs no overflow check.
                if nm < CHUNK_RIDS_MAX
                    && nb <= B_MAX
                    && (nm.wrapping_mul(u64::from(nb)).wrapping_add(7) >> 3).wrapping_add(3) <= self.v
                {
                    self.m = nm;
                    self.b = nb;
                    self.last = rid;
                    return true;
                }
            }
        }
        *self = Chunker { v: self.v, open: true, tomb, m: 0, b: 0, last: rid };
        false
    }

    /// Ends the key: the next item opens a new chunk.
    pub fn close(&mut self) {
        self.open = false;
    }
}
