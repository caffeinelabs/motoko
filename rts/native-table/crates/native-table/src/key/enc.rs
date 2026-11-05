//! E(x) per key kind and its checked inverse E⁻¹.
//!
//! Decoders take the key bytes, a position and the image the bytes came from; a malformed key is
//! `Corrupt(image, Range)` or `Corrupt(image, Encoding)` at the offending position, and a
//! key that ends early is `Corrupt(image, Encoding)` too, since every E states its own length.

use super::Sink;
use crate::fault::{corrupt, Corruption, Fault, ImageId};

/// The leaf a host-value refusal names when the encoder does not know it; callers that know the leaf
/// replace it.
pub const NO_LEAF: u32 = u32::MAX;

// ---------------------------------------------------------------- encoders

/// Bool: `00` false, `01` true.
#[inline]
pub fn enc_bool<S: Sink>(s: &mut S, v: bool) -> Result<(), Fault> {
    s.put_byte(u8::from(v))
}

/// An unsigned integer of `width` bytes (1, 2, 4 or 8), big-endian. A value that does not fit is refused.
#[inline]
pub fn enc_u64<S: Sink>(s: &mut S, v: u64, width: u8) -> Result<(), Fault> {
    let b = v.to_be_bytes();
    let w = usize::from(width);
    let skip = 8usize.checked_sub(w).ok_or(Fault::BadValue { leaf: NO_LEAF })?;
    if w < 8 && v >> (w.wrapping_mul(8)) != 0 {
        return Err(Fault::BadValue { leaf: NO_LEAF });
    }
    s.put(b.get(skip..).ok_or(Fault::BadValue { leaf: NO_LEAF })?)
}

/// A signed integer of `width` bytes (1, 2, 4 or 8): two's complement with the top bit flipped, big-endian.
#[inline]
pub fn enc_i64<S: Sink>(s: &mut S, v: i64, width: u8) -> Result<(), Fault> {
    let w = u32::from(width);
    if !matches!(width, 1 | 2 | 4 | 8) {
        return Err(Fault::BadValue { leaf: NO_LEAF });
    }
    let bits = w.wrapping_mul(8);
    // The value must be representable in `bits`: sign-extending its low bits gives it back.
    let shift = 64u32.wrapping_sub(bits);
    if bits < 64 && (v.wrapping_shl(shift)).wrapping_shr(shift) != v {
        return Err(Fault::BadValue { leaf: NO_LEAF });
    }
    let flipped = (v as u64) ^ (1u64 << bits.wrapping_sub(1));
    let masked = if bits < 64 { flipped & ((1u64 << bits).wrapping_sub(1)) } else { flipped };
    enc_u64(s, masked, width)
}

/// u128: 16 bytes big-endian.
#[inline]
pub fn enc_u128<S: Sink>(s: &mut S, v: u128) -> Result<(), Fault> {
    s.put(&v.to_be_bytes())
}

/// i128: 16 bytes, two's complement with the top bit flipped, big-endian.
#[inline]
pub fn enc_i128<S: Sink>(s: &mut S, v: i128) -> Result<(), Fault> {
    s.put(&((v as u128) ^ (1u128 << 127)).to_be_bytes())
}

/// Char: the scalar value as a big-endian u32.
#[inline]
pub fn enc_char<S: Sink>(s: &mut S, c: char) -> Result<(), Fault> {
    s.put(&u32::from(c).to_be_bytes())
}

/// Float (f64), from its bits: a NaN with sign 0 is `FF`×8, a NaN with sign 1 `00`×8, −0.0 is +0.0;
/// otherwise the bits with the sign bit set when the sign is 0, else their complement.
#[inline]
pub fn enc_f64<S: Sink>(s: &mut S, bits: u64) -> Result<(), Fault> {
    s.put(&f64_key(bits).to_be_bytes())
}

/// The 8-byte key of an f64, as a u64.
#[inline]
pub const fn f64_key(bits: u64) -> u64 {
    const SIGN: u64 = 1 << 63;
    const EXP: u64 = 0x7FF0_0000_0000_0000;
    const FRAC: u64 = 0x000F_FFFF_FFFF_FFFF;
    if bits & EXP == EXP && bits & FRAC != 0 {
        return if bits & SIGN == 0 { u64::MAX } else { 0 };
    }
    let b = if bits == SIGN { 0 } else { bits };
    if b & SIGN == 0 {
        b | SIGN
    } else {
        !b
    }
}

/// Float32, by the same rule on 32 bits.
#[inline]
pub fn enc_f32<S: Sink>(s: &mut S, bits: u32) -> Result<(), Fault> {
    s.put(&f32_key(bits).to_be_bytes())
}

/// The 4-byte key of an f32, as a u32.
#[inline]
pub const fn f32_key(bits: u32) -> u32 {
    const SIGN: u32 = 1 << 31;
    const EXP: u32 = 0x7F80_0000;
    const FRAC: u32 = 0x007F_FFFF;
    if bits & EXP == EXP && bits & FRAC != 0 {
        return if bits & SIGN == 0 { u32::MAX } else { 0 };
    }
    let b = if bits == SIGN { 0 } else { bits };
    if b & SIGN == 0 {
        b | SIGN
    } else {
        !b
    }
}

/// Text, Blob, `PString`, `PList<u8>`: every `00` byte written `00 FF`, other bytes as they are, then
/// `00 00`.
#[inline]
pub fn enc_bytes<S: Sink>(s: &mut S, bytes: &[u8]) -> Result<(), Fault> {
    enc_bytes_piece(s, bytes)?;
    enc_bytes_end(s)
}

/// One piece of an escaped Text or Blob part; pieces of one value may be cut anywhere (`Sink::put_escaped`).
pub fn enc_bytes_piece<S: Sink>(s: &mut S, bytes: &[u8]) -> Result<(), Fault> {
    s.put_escaped(bytes)
}

/// `bytes` escaped into `s` a run at a time: each run of bytes without `00` and each `00 FF` its own put.
pub fn escape_runs<S: Sink + ?Sized>(s: &mut S, bytes: &[u8]) -> Result<(), Fault> {
    let mut rest = bytes;
    while let Some(z) = first_zero(rest) {
        let (run, tail) = rest.split_at_checked(z).unwrap_or((rest, &[]));
        s.put(run)?;
        s.put(&[0x00, 0xFF])?;
        rest = tail.get(1..).unwrap_or(&[]);
    }
    s.put(rest)
}

/// Where the first `00` byte of `bytes` lies, looked for 8 bytes at a time: a word's bytes taken little-endian,
/// `(x - 0x01..01) & !x & 0x80..80` marks its `00` bytes, and none below its lowest `00` byte (a borrow only
/// carries upwards from one), so its lowest mark is the first.
#[inline]
fn first_zero(bytes: &[u8]) -> Option<usize> {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGHS: u64 = 0x8080_8080_8080_8080;
    let mut words = bytes.chunks_exact(8);
    let mut at = 0usize;
    for w in &mut words {
        let x = u64::from_le_bytes(<[u8; 8]>::try_from(w).unwrap_or([1; 8]));
        let z = x.wrapping_sub(ONES) & !x & HIGHS;
        if z != 0 {
            return Some(at.wrapping_add((z.trailing_zeros() / 8) as usize));
        }
        at = at.wrapping_add(8);
    }
    words.remainder().iter().position(|&b| b == 0).map(|i| at.wrapping_add(i))
}

/// Whether `bytes` hold no `00`, looked for 8 bytes at a time (`first_zero`).
#[inline]
pub fn zero_free(bytes: &[u8]) -> bool {
    first_zero(bytes).is_none()
}

/// Whether one of the 8 bytes of `x` is `00` (by `first_zero`'s test).
#[inline]
pub(crate) const fn has_zero(x: u64) -> bool {
    x.wrapping_sub(0x0101_0101_0101_0101) & !x & 0x8080_8080_8080_8080 != 0
}

/// The terminator of an escaped part.
#[inline]
pub fn enc_bytes_end<S: Sink>(s: &mut S) -> Result<(), Fault> {
    s.put(&[0x00, 0x00])
}

/// Principal: its 0–29 bytes zero-padded to 29, then its length as one byte (30 bytes, the `Fixed` 30 cell).
pub fn enc_principal<S: Sink>(s: &mut S, bytes: &[u8]) -> Result<(), Fault> {
    let mut b = [0u8; 30];
    let n = bytes.len();
    let dst = b.get_mut(..n).ok_or(Fault::BadValue { leaf: NO_LEAF })?;
    if n > 29 {
        return Err(Fault::BadValue { leaf: NO_LEAF });
    }
    crate::mem::copy_bytes(dst, bytes);
    // n ≤ 29.
    #[allow(clippy::cast_possible_truncation)]
    let len = n as u8;
    if let Some(last) = b.last_mut() {
        *last = len;
    }
    s.put(&b)
}

/// `[u8; N]`: the N bytes.
#[inline]
pub fn enc_fixed<S: Sink>(s: &mut S, bytes: &[u8]) -> Result<(), Fault> {
    s.put(bytes)
}

/// The null tag of an option or nullable part: `00` for null, `01` before a value.
#[inline]
pub fn enc_opt_tag<S: Sink>(s: &mut S, some: bool) -> Result<(), Fault> {
    s.put_byte(u8::from(some))
}

/// Nat or Int of any magnitude, from a big-endian magnitude (leading zeros allowed and dropped) and a sign.
/// Zero is `80` whatever the sign.
pub fn enc_int_be<S: Sink>(s: &mut S, negative: bool, magnitude: &[u8]) -> Result<(), Fault> {
    let first = magnitude.iter().position(|&b| b != 0).unwrap_or(magnitude.len());
    let m = magnitude.get(first..).unwrap_or(&[]);
    let n = m.len() as u64;
    let neg = negative && n > 0;
    int_head(s, neg, n)?;
    if neg {
        complement_out(s, m)
    } else {
        s.put(m)
    }
}

/// Nat of at most 64 bits.
#[inline]
pub fn enc_nat_u64<S: Sink>(s: &mut S, v: u64) -> Result<(), Fault> {
    enc_int_be(s, false, &v.to_be_bytes())
}

/// Int of at most 64 bits.
#[inline]
pub fn enc_int_i64<S: Sink>(s: &mut S, v: i64) -> Result<(), Fault> {
    enc_int_be(s, v < 0, &v.unsigned_abs().to_be_bytes())
}

/// Nat or Int of any magnitude from (signed) LEB128 bytes, as VE and the `Leb` form store them: unsigned
/// LEB128 for a Nat (`signed` false), signed LEB128 for an Int. The LEB form must end inside `leb` and be
/// canonical, as table writes store it; its key is `LebFacts`' (a bit accumulator, a few operations a byte).
pub fn enc_int_leb<S: Sink>(s: &mut S, leb: &[u8], signed: bool, image: ImageId) -> Result<(), Fault> {
    let end = leb.iter().position(|&b| b & 0x80 == 0).ok_or(corrupt(image, Corruption::Encoding, 0))?;
    let leb = leb.get(..=end).ok_or(corrupt(image, Corruption::Encoding, 0))?;
    enc_leb(s, leb.len() as u64, signed, image, &mut &*leb)
}

/// `enc_int_leb` of a canonical form of `len` bytes that does not lie in one piece, read through `read(at, out)`,
/// which fills `out` with the form's bytes from `at`.
pub fn enc_int_leb_read<S: Sink, R: FnMut(u64, &mut [u8]) -> Result<(), Fault>>(
    s: &mut S,
    len: u64,
    signed: bool,
    image: ImageId,
    read: R,
) -> Result<(), Fault> {
    enc_leb(s, len, signed, image, &mut ReadWindow { read, buf: [0; LEB_WINDOW] })
}

/// Key bytes `enc_leb` makes at a time: 36 runs of 7, which `LebFacts::key_bytes` makes from 8 groups each.
const LEB_RUN: u64 = 252;
/// The LEB bytes the groups of `LEB_RUN` key bytes lie in, and a few more.
const LEB_WINDOW: usize = 296;
const _: () = assert!(LEB_RUN % 7 == 0 && LEB_WINDOW as u64 >= LEB_RUN / 7 * 8 + 2);

/// Where `enc_leb` reads a LEB form's bytes [a, b), at most `LEB_WINDOW` of them.
trait LebWindow {
    fn window(&mut self, a: u64, b: u64) -> Result<&[u8], Fault>;
}

/// A form in one slice: its bytes as they lie.
impl LebWindow for &[u8] {
    fn window(&mut self, a: u64, b: u64) -> Result<&[u8], Fault> {
        crate::mem::sub(self, a, b.saturating_sub(a))
    }
}

/// A form read into a buffer of its own.
struct ReadWindow<R> {
    read: R,
    buf: [u8; LEB_WINDOW],
}

impl<R: FnMut(u64, &mut [u8]) -> Result<(), Fault>> LebWindow for ReadWindow<R> {
    fn window(&mut self, a: u64, b: u64) -> Result<&[u8], Fault> {
        let n = usize::try_from(b.saturating_sub(a)).unwrap_or(usize::MAX);
        let w = self.buf.get_mut(..n).ok_or(corrupt(ImageId::Runtime, Corruption::Bounds, a))?;
        (self.read)(a, w)?;
        Ok(w)
    }
}

/// The key of the canonical form of `len` bytes that `w` reads: its facts from its last bytes and, for a
/// negative, its lowest non-zero byte from its first ones (`leb_low`); its head; then its magnitude from the
/// most significant byte down, `LEB_RUN` bytes at a time from a multiple of `LEB_RUN`, each run from the form's
/// bytes its span names (`LebFacts`).
fn enc_leb<S: Sink, W: LebWindow>(s: &mut S, len: u64, signed: bool, image: ImageId, w: &mut W) -> Result<(), Fault> {
    let top = len.saturating_sub(LEB_TOP as u64);
    let low = if leb_negative(w.window(top, len)?, signed, image)? {
        let mut at = 0u64;
        loop {
            let b = len.min(at.saturating_add(LEB_WINDOW as u64));
            if at >= b {
                return Err(corrupt(image, Corruption::Encoding, len));
            }
            if let Some(x) = leb_low(w.window(at, b)?, at) {
                break x;
            }
            at = b;
        }
    } else {
        0
    };
    let f = LebFacts::new(len, signed, low, w.window(top, len)?, image)?;
    f.head(s)?;
    // LEB_RUN = 252.
    #[allow(clippy::cast_possible_truncation)]
    let mut out = [0u8; LEB_RUN as usize];
    let mut hi = f.mag_len;
    while hi > 0 {
        let wants = s.wants();
        if wants == 0 {
            // A sink that keeps a prefix counts the rest: the magnitude's lower bytes are not computed.
            return s.skip(hi);
        }
        let run = match hi % LEB_RUN {
            0 => LEB_RUN,
            r => r,
        };
        let lo = hi.wrapping_sub(run.min(wants));
        let (a, b) = f.span(lo, hi);
        let kb = out.get_mut(..usize::try_from(hi.wrapping_sub(lo)).unwrap_or(0)).unwrap_or(&mut []);
        f.key_bytes(w.window(a, b)?, a, lo, kb)?;
        s.put(kb)?;
        hi = lo;
    }
    Ok(())
}

/// LEB bytes at a canonical form's end that hold its magnitude's most significant byte (`LebFacts::new`).
pub const LEB_TOP: usize = 24;

/// Whether the (signed, when `signed`) LEB128 form whose last bytes are `top` is a negative's;
/// `Corrupt(Encoding)` when its last byte does not end it.
pub fn leb_negative(top: &[u8], signed: bool, image: ImageId) -> Result<bool, Fault> {
    match top.last() {
        Some(&b) if b & 0x80 == 0 => Ok(signed && b & 0x40 != 0),
        _ => Err(corrupt(image, Corruption::Encoding, 0)),
    }
}

/// The lowest non-zero byte of a LEB128 form's two's-complement value, when the form's bytes `win`, from its
/// byte `base`, hold a set bit: the byte its lowest set bit lies in. The first byte with a set group bit is
/// found 8 bytes at a time (a little-endian word's lowest byte with a bit of `0x7F` set), the last 0-7 bytes
/// one at a time.
pub fn leb_low(win: &[u8], base: u64) -> Option<u64> {
    const GROUPS: u64 = 0x7F7F_7F7F_7F7F_7F7F;
    let mut words = win.chunks_exact(8);
    let mut at = 0usize;
    let mut found = None;
    for w in &mut words {
        let x = u64::from_le_bytes(<[u8; 8]>::try_from(w).unwrap_or([0; 8])) & GROUPS;
        if x != 0 {
            let i = (x.trailing_zeros() / 8) as usize;
            found = Some((at.wrapping_add(i), x.to_le_bytes().get(i).copied().unwrap_or(0)));
            break;
        }
        at = at.wrapping_add(8);
    }
    let (j, b) = match found {
        Some(f) => f,
        None => words
            .remainder()
            .iter()
            .enumerate()
            .find(|&(_, &b)| b & 0x7F != 0)
            .map(|(i, &b)| (at.wrapping_add(i), b))?,
    };
    let bit = base.saturating_add(j as u64).saturating_mul(7).saturating_add(u64::from((b & 0x7F).trailing_zeros()));
    Some(bit / 8)
}

/// A Nat or Int beyond i128 by its canonical (signed) LEB128 form of `len` bytes, read a window at a time with
/// nothing held between windows but these facts, so its key can be written over several calls: its sign, the
/// lowest non-zero byte of its two's-complement value (a negative's, `leb_low`; 0 otherwise) and its
/// magnitude's length. Its key is `enc_int_leb`'s: its head (`head`), then its magnitude's bytes from the most
/// significant down (`key_bytes`, each run of them from the form's bytes `span` gives).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LebFacts {
    pub len: u64,
    pub negative: bool,
    pub low: u64,
    pub mag_len: u64,
}

impl LebFacts {
    /// The facts of a form of `len` bytes whose last min(len, `LEB_TOP`) bytes are `top`, a negative's lowest
    /// non-zero byte being `low`: its magnitude's length found among the bytes `top` holds, as a canonical
    /// form's most significant byte is. `Corrupt(Encoding)` when its last byte does not end it or no byte
    /// there is the magnitude's highest.
    pub fn new(len: u64, signed: bool, low: u64, top: &[u8], image: ImageId) -> Result<Self, Fault> {
        let negative = leb_negative(top, signed, image)?;
        let base = len.checked_sub(top.len() as u64).ok_or(corrupt(image, Corruption::Encoding, len))?;
        let mut f = LebFacts { len, negative, low: if negative { low } else { 0 }, mag_len: 0 };
        // The two's-complement bytes whose groups lie in `top` or past the form's end, sign-extended, and the
        // one past them; their key bytes give their magnitude bytes back.
        let (from, to) = (base.saturating_mul(7).div_ceil(8), len.saturating_mul(7).div_ceil(8).saturating_add(1));
        let mut kb = [0u8; LEB_TOP];
        let n = usize::try_from(to.saturating_sub(from)).unwrap_or(usize::MAX);
        let kb = kb.get_mut(..n).ok_or(corrupt(image, Corruption::Encoding, len))?;
        f.key_bytes(top, base, from, kb)?;
        let flip = if negative { 0xFF } else { 0 };
        match kb.iter().position(|&b| b ^ flip != 0) {
            Some(j) => f.mag_len = to.wrapping_sub(j as u64),
            None if base == 0 => {}
            None => return Err(corrupt(image, Corruption::Encoding, len)),
        }
        Ok(f)
    }

    /// Writes the key's head: its sign and the magnitude's length.
    pub fn head<S: Sink>(&self, s: &mut S) -> Result<(), Fault> {
        int_head(s, self.negative && self.mag_len > 0, self.mag_len)
    }

    /// The form's bytes [from, to) that `key_bytes` reads for the magnitude's bytes [lo, hi).
    pub fn span(&self, lo: u64, hi: u64) -> (u64, u64) {
        if hi <= lo {
            return (0, 0);
        }
        let to = hi.saturating_mul(8).wrapping_sub(1) / 7;
        ((lo.saturating_mul(8) / 7).min(self.len), to.saturating_add(1).min(self.len))
    }

    /// Writes the key's bytes of the magnitude's bytes [lo, lo + out.len()) into `out`, the most significant
    /// first (complemented for a negative), from `win`, the form's bytes from `base` (`span`): the
    /// two's-complement value's bits taken from its 7-bit groups. The groups it reads inside the form are
    /// checked to lie in `win` once; then each run of 7 bytes from a multiple of 7 inside the form (and above
    /// a negative's lowest non-zero byte) is made from its 8 groups read as one word, and any other byte from
    /// the two groups holding it.
    pub fn key_bytes(&self, win: &[u8], base: u64, lo: u64, out: &mut [u8]) -> Result<(), Fault> {
        let hi = lo.saturating_add(out.len() as u64);
        if hi <= lo {
            return Ok(());
        }
        // The groups read: those holding the magnitude's bits [lo · 8, hi · 8).
        let first = lo.saturating_mul(8) / 7;
        let last_in = (hi.saturating_mul(8).wrapping_sub(1) / 7).min(self.len.saturating_sub(1));
        if first < self.len && (first < base || last_in.wrapping_sub(base) >= win.len() as u64) {
            return Err(corrupt(ImageId::Runtime, Corruption::Bounds, lo));
        }
        let fill = if self.negative { 0x7F } else { 0 };
        // Group g (7 bits) of the form, sign-extended past its end.
        let group = |g: u64| -> u64 {
            if g >= self.len {
                fill
            } else {
                let i = usize::try_from(g.wrapping_sub(base)).unwrap_or(usize::MAX);
                win.get(i).map_or(0, |&b| u64::from(b & 0x7F))
            }
        };
        // The key byte of the two's-complement byte `tc` at index i: the magnitude is the complement plus one,
        // so a negative's key byte (complemented) is the value's above its lowest non-zero byte, that byte less
        // one, and 0xFF below.
        let key = |i: u64, tc: u8| -> u8 {
            if !self.negative {
                tc
            } else {
                match i.cmp(&self.low) {
                    core::cmp::Ordering::Greater => tc,
                    core::cmp::Ordering::Equal => tc.wrapping_sub(1),
                    core::cmp::Ordering::Less => 0xFF,
                }
            }
        };
        // Byte i is out[hi − 1 − i], from the two groups holding it (none below a negative's lowest non-zero
        // byte).
        let one = |i: u64, out: &mut [u8]| {
            let b = if self.negative && i < self.low {
                0xFF
            } else {
                let p = i.wrapping_mul(8);
                let g = p / 7;
                let v = group(g) | (group(g.wrapping_add(1)) << 7);
                key(i, v.wrapping_shr(u32::try_from(p % 7).unwrap_or(0)).to_le_bytes()[0])
            };
            if let Some(o) = out.get_mut(usize::try_from(hi.wrapping_sub(1).wrapping_sub(i)).unwrap_or(usize::MAX)) {
                *o = b;
            }
        };
        let mut i = lo;
        let lead = lo.div_ceil(7).saturating_mul(7).min(hi);
        while i < lead {
            one(i, out);
            i = i.wrapping_add(1);
        }
        // Bytes i to i + 6, i a multiple of 7: groups g to g + 7, 56 bits, out[hi − 7 − i ..= hi − 1 − i].
        let mut g = (i / 7).wrapping_mul(8);
        while hi.wrapping_sub(i) >= 7 {
            let at = usize::try_from(hi.wrapping_sub(7).wrapping_sub(i)).unwrap_or(usize::MAX);
            let j = usize::try_from(g.wrapping_sub(base)).unwrap_or(usize::MAX);
            let whole = (!self.negative || i > self.low) && g.saturating_add(8) <= self.len;
            let below = self.negative && i.saturating_add(7) <= self.low;
            let dst = out.get_mut(at..at.saturating_add(7)).and_then(|d| <&mut [u8; 7]>::try_from(d).ok());
            match (whole, below, win.get(j..j.saturating_add(8)), dst) {
                (true, _, Some(w), Some(d)) => {
                    // Eight groups read at once and packed: their 7-bit fields side by side.
                    let mut x = u64::from_le_bytes(w.try_into().unwrap_or([0; 8])) & 0x7F7F_7F7F_7F7F_7F7F;
                    x = (x & 0x007F_007F_007F_007F) | ((x & 0x7F00_7F00_7F00_7F00) >> 1);
                    x = (x & 0x0000_3FFF_0000_3FFF) | ((x & 0x3FFF_0000_3FFF_0000) >> 2);
                    x = (x & 0x0000_0000_0FFF_FFFF) | ((x & 0x0FFF_FFFF_0000_0000) >> 4);
                    let b = x.to_be_bytes();
                    (d[0], d[1], d[2], d[3], d[4], d[5], d[6]) = (b[1], b[2], b[3], b[4], b[5], b[6], b[7]);
                }
                // Below a negative's lowest non-zero byte: all ones.
                (_, true, _, Some(d)) => *d = [0xFF; 7],
                _ => {
                    for t in 0..7u64 {
                        one(i.wrapping_add(t), out);
                    }
                }
            }
            i = i.wrapping_add(7);
            g = g.wrapping_add(8);
        }
        while i < hi {
            one(i, out);
            i = i.wrapping_add(1);
        }
        Ok(())
    }
}

/// The head of a Nat or Int key, and E_N(n) for long magnitudes.
fn int_head<S: Sink>(s: &mut S, negative: bool, n: u64) -> Result<(), Fault> {
    if n <= 126 {
        // n ≤ 126.
        #[allow(clippy::cast_possible_truncation)]
        let n8 = n as u8;
        return s.put_byte(if negative { 0x7Fu8.wrapping_sub(n8) } else { 0x80u8.wrapping_add(n8) });
    }
    // E_N(n): n ≥ 127 as a non-negative value of at most 8 magnitude bytes.
    let nb = n.to_be_bytes();
    let first = nb.iter().position(|&b| b != 0).unwrap_or(nb.len());
    let nm = nb.get(first..).unwrap_or(&[]);
    // At most 8 bytes.
    #[allow(clippy::cast_possible_truncation)]
    let len = nm.len() as u8;
    if negative {
        s.put_byte(0x00)?;
        s.put_byte(!0x80u8.wrapping_add(len))?;
        complement_out(s, nm)
    } else {
        s.put_byte(0xFF)?;
        s.put_byte(0x80u8.wrapping_add(len))?;
        s.put(nm)
    }
}

fn complement_out<S: Sink>(s: &mut S, m: &[u8]) -> Result<(), Fault> {
    let mut buf = [0u8; 64];
    for chunk in m.chunks(buf.len()) {
        let out = buf.get_mut(..chunk.len()).unwrap_or(&mut []);
        for (o, &b) in out.iter_mut().zip(chunk) {
            *o = !b;
        }
        s.put(out)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- decoders (E⁻¹)

fn short(image: ImageId, at: u64) -> Fault {
    corrupt(image, Corruption::Encoding, at)
}

fn take(key: &[u8], pos: u64, n: u64, image: ImageId) -> Result<&[u8], Fault> {
    let s = usize::try_from(pos).map_err(|_| short(image, pos))?;
    let e = pos.checked_add(n).and_then(|e| usize::try_from(e).ok()).ok_or(short(image, pos))?;
    key.get(s..e).ok_or(short(image, pos))
}

fn take_array<const N: usize>(key: &[u8], pos: u64, image: ImageId) -> Result<[u8; N], Fault> {
    let s = take(key, pos, N as u64, image)?;
    s.try_into().map_err(|_| short(image, pos))
}

/// A Bool or a null tag: `00` or `01`, else `Corrupt(Range)`.
pub fn dec_bool(key: &[u8], pos: u64, image: ImageId) -> Result<(bool, u64), Fault> {
    let [b] = take_array::<1>(key, pos, image)?;
    match b {
        0 => Ok((false, pos.wrapping_add(1))),
        1 => Ok((true, pos.wrapping_add(1))),
        _ => Err(corrupt(image, Corruption::Range, pos)),
    }
}

/// An unsigned integer of `width` bytes (1, 2, 4 or 8).
pub fn dec_u64(key: &[u8], pos: u64, width: u8, image: ImageId) -> Result<(u64, u64), Fault> {
    let s = take(key, pos, u64::from(width), image)?;
    if width > 8 {
        return Err(short(image, pos));
    }
    let v = s.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
    Ok((v, pos.wrapping_add(u64::from(width))))
}

/// Char: a big-endian u32 that must be a Unicode scalar value, else `Corrupt(Range)`.
pub fn dec_char(key: &[u8], pos: u64, image: ImageId) -> Result<(char, u64), Fault> {
    let b = take_array::<4>(key, pos, image)?;
    let c = char::from_u32(u32::from_be_bytes(b)).ok_or(corrupt(image, Corruption::Range, pos))?;
    Ok((c, pos.wrapping_add(4)))
}

/// Text or Blob part: unescapes into `out` up to the `00 00` terminator. Each `00` must be
/// followed by `FF` or `00`, and the terminator must lie within the key, else `Corrupt(Encoding)`. UTF-8
/// is the host's to check.
pub fn dec_bytes<S: Sink>(key: &[u8], pos: u64, out: &mut S, image: ImageId) -> Result<u64, Fault> {
    let start = usize::try_from(pos).map_err(|_| short(image, pos))?;
    let mut rest = key.get(start..).ok_or(short(image, pos))?;
    let mut at = pos;
    loop {
        let z = rest.iter().position(|&b| b == 0).ok_or(short(image, at))?;
        let (run, tail) = rest.split_at_checked(z).unwrap_or((rest, &[]));
        out.put(run)?;
        at = at.wrapping_add(z as u64);
        match tail.get(1) {
            Some(0xFF) => {
                out.put_byte(0)?;
                rest = tail.get(2..).unwrap_or(&[]);
                at = at.wrapping_add(2);
            }
            Some(0x00) => return Ok(at.wrapping_add(2)),
            _ => return Err(short(image, at)),
        }
    }
}

/// Principal: the 30-byte form; the length byte must be at most 29, else `Corrupt(Range)`. Returns the
/// padded bytes and the length.
pub fn dec_principal(key: &[u8], pos: u64, image: ImageId) -> Result<([u8; 29], u8, u64), Fault> {
    let b = take_array::<30>(key, pos, image)?;
    let (body, len) = b.split_last_chunk::<1>().ok_or(short(image, pos))?;
    let n = len.first().copied().unwrap_or(0);
    if n > 29 {
        return Err(corrupt(image, Corruption::Range, pos.wrapping_add(29)));
    }
    let mut out = [0u8; 29];
    crate::mem::copy_bytes(&mut out, body.get(..29).ok_or(short(image, pos))?);
    Ok((out, n, pos.wrapping_add(30)))
}

/// The class and magnitude length of a Nat or Int key: (negative, n, position of the magnitude).
pub fn int_header(key: &[u8], pos: u64, image: ImageId) -> Result<(bool, u64, u64), Fault> {
    let [h] = take_array::<1>(key, pos, image)?;
    let at = pos.wrapping_add(1);
    match h {
        0x80..=0xFE => Ok((false, u64::from(h.wrapping_sub(0x80)), at)),
        0x01..=0x7E => Ok((true, u64::from(0x7Fu8.wrapping_sub(h)), at)),
        0xFF | 0x00 => {
            let neg = h == 0x00;
            let [hn] = take_array::<1>(key, at, image)?;
            let hn = if neg { !hn } else { hn };
            // E_N(n) is a short non-negative form of 1 to 8 magnitude bytes.
            let len = hn.wrapping_sub(0x80);
            if !(0x81..=0x88).contains(&hn) {
                return Err(short(image, at));
            }
            let nb = take(key, at.wrapping_add(1), u64::from(len), image)?;
            let n = nb.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(if neg { !b } else { b }));
            if n < 127 {
                return Err(short(image, at));
            }
            Ok((neg, n, at.wrapping_add(1).wrapping_add(u64::from(len))))
        }
        // 0x7F would be a negative zero.
        _ => Err(short(image, pos)),
    }
}
