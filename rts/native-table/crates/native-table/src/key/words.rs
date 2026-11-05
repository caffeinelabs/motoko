//! The comparator of key bytes, eight bytes at a time.
//!
//! Every index orders its keys by memcmp over their bytes. Here both sides are loaded eight bytes at a time
//! as little-endian words, read where they lie (only a key's last bytes short of a word are read into one,
//! zero-padded): two words differ first at the byte of their lowest differing bit, which the trailing zeros
//! of their XOR give, and that byte orders them. No `Result` is made per comparison.
//!
//! - **A key of at most 64 bytes** becomes its words once (`KeyWords`), so each comparison loads only the
//!   stored side. A node search for a key that fits the key area W (`search_fits`) is monomorphized by the
//!   area's words N = W / 8, 1 to 8. The entry's field orders entries whose key areas equal the key, through
//!   the caller's rank.
//! - **Resumable prefixes.** A descent's lower and upper bounds share some first bytes with the key, and so
//!   does every entry between them. A comparison may start at the fewer of the two (`area_cmp`'s `from`,
//!   `area_key_cmp`'s `from`) and tells how many bytes the two sides share, so the next comparison resumes
//!   there.
//! - **Byte strings of any length** are compared where they lie: over their common length with the bytes
//!   they share (`bytes_cmp`), or in memcmp order with a proper prefix first (`cmp_bytes`; two 8-byte keys
//!   are one word each).

use core::cmp::Ordering;

use crate::mem::copy_bytes;

/// Words of a key held ready: its first 64 bytes.
pub const KEY_WORDS: usize = 8;
pub const KEY_BYTES: usize = 64;
const _: () = assert!(KEY_BYTES == 8 * KEY_WORDS);

/// The little-endian word of bytes [o, o + 8) of `s`; `None` past its end.
#[inline(always)]
pub fn le64(s: &[u8], o: usize) -> Option<u64> {
    s.get(o..)?.first_chunk::<8>().map(|x| u64::from_le_bytes(*x))
}

/// The little-endian word of bytes [at, at + 8) of `b`, zero past its end.
#[inline]
pub fn word_padded(b: &[u8], at: usize) -> u64 {
    if let Some(w) = le64(b, at) {
        return w;
    }
    let n = b.len();
    if at >= n {
        return 0;
    }
    // 1 to 7 bytes are left: the word ending at b's end, shifted down past the bytes before `at`.
    if let Some(t) = n.checked_sub(8).and_then(|e| le64(b, e)) {
        // n − at is 1 to 7 here, so the shift is 8 to 56.
        return t.wrapping_shr(u32::try_from(8usize.wrapping_sub(n.wrapping_sub(at))).unwrap_or(0).wrapping_mul(8));
    }
    let mut x = 0u64;
    for (j, &y) in b.get(at..).unwrap_or(&[]).iter().enumerate() {
        // j < 8 here: fewer than 8 bytes are left.
        x |= u64::from(y).wrapping_shl(u32::try_from(j).unwrap_or(0).wrapping_mul(8));
    }
    x
}

/// Whether the first differing byte of two little-endian words of key bytes is smaller in `a`; a ≠ b.
#[inline(always)]
pub fn word_below(a: u64, b: u64) -> bool {
    // a ≠ b, so the lowest differing bit is below 64: its byte is the first that differs.
    let sh = (a ^ b).trailing_zeros() & !7;
    a.wrapping_shr(sh) & 0xFF < b.wrapping_shr(sh) & 0xFF
}

/// The first differing byte of two different little-endian words, as an index into the word.
#[inline(always)]
pub fn diff_byte(a: u64, b: u64) -> usize {
    // a ≠ b: trailing_zeros < 64, so the index is below 8.
    usize::try_from((a ^ b).trailing_zeros() / 8).unwrap_or(0)
}

/// A key's first 64 bytes, zero-padded, as little-endian words. Words past the 8th are zero.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyWords {
    pub w: [u64; KEY_WORDS],
}

impl KeyWords {
    /// All eight words of `key`. A key of 8 bytes is its one word, the rest zero; a key of 64 bytes or more
    /// is eight whole words.
    #[inline(always)]
    pub fn of(key: &[u8]) -> KeyWords {
        let mut w = [0u64; KEY_WORDS];
        if let Ok(x) = <&[u8; 8]>::try_from(key) {
            if let Some(w0) = w.first_mut() {
                *w0 = u64::from_le_bytes(*x);
            }
            return KeyWords { w };
        }
        if let Some(all) = key.first_chunk::<KEY_BYTES>() {
            for (slot, c) in w.iter_mut().zip(all.chunks_exact(8)) {
                if let Some(x) = c.first_chunk::<8>() {
                    *slot = u64::from_le_bytes(*x);
                }
            }
            return KeyWords { w };
        }
        KeyWords::first(key, KEY_WORDS)
    }

    /// The first `n` words of `key` (at most eight); the rest zero.
    #[inline(always)]
    pub fn first(key: &[u8], n: usize) -> KeyWords {
        let mut w = [0u64; KEY_WORDS];
        for (i, slot) in w.iter_mut().enumerate().take(n) {
            *slot = word_padded(key, i.wrapping_mul(8));
        }
        KeyWords { w }
    }

    /// Word i; zero past the eighth.
    #[inline(always)]
    pub fn get(&self, i: usize) -> u64 {
        self.w.get(i).copied().unwrap_or(0)
    }
}

/// Orders the key area of `n` words at `koff` of `s` against the key's words `k` (the key zero-padded to
/// the area), from word `from` on, the words before known equal: memcmp order, 8 bytes at a time. `None`
/// when the area does not lie inside `s`.
#[inline(always)]
pub fn area_cmp(s: &[u8], koff: usize, from: usize, n: usize, k: &KeyWords) -> Option<Ordering> {
    let mut i = from;
    while i < n {
        let a = le64(s, koff.wrapping_add(i.wrapping_mul(8)))?;
        let b = k.get(i);
        if a != b {
            return Some(if word_below(a, b) { Ordering::Less } else { Ordering::Greater });
        }
        i = i.wrapping_add(1);
    }
    Some(Ordering::Equal)
}

/// The little-endian word of bytes [at, at + 8) of `b`, zero past its end, its last bytes copied into a
/// zeroed word: the form a comparison loop over a key area keeps tightest.
#[inline(always)]
fn word_copied(b: &[u8], at: usize) -> u64 {
    match b.get(at..).and_then(|t| t.first_chunk::<8>()) {
        Some(w) => u64::from_le_bytes(*w),
        None => {
            let mut a = [0u8; 8];
            if let Some(t) = b.get(at..) {
                copy_bytes(&mut a, t);
            }
            u64::from_le_bytes(a)
        }
    }
}

/// Orders the W-byte key area `area` (W = `w`, a multiple of 8) against `key` zero-padded to W, from byte
/// `from` (a multiple of 8) on, the bytes before known equal: `Some` with the order and the first differing
/// position when the two differ, `None` when they agree up to W.
#[inline(always)]
pub fn area_key_cmp(area: &[u8], key: &[u8], from: usize, w: usize) -> Option<(Ordering, usize)> {
    let mut k = from;
    while k < w {
        let a = word_copied(area, k);
        let b = word_copied(key, k);
        if a != b {
            let o = if word_below(a, b) { Ordering::Less } else { Ordering::Greater };
            return Some((o, k.wrapping_add(diff_byte(a, b))));
        }
        k = k.wrapping_add(8);
    }
    None
}

/// The order of byte strings `a` and `b` over the length they have in common, compared 8 bytes at a time,
/// and the bytes they share.
#[inline(always)]
pub fn bytes_cmp(a: &[u8], b: &[u8]) -> (Ordering, usize) {
    let n = a.len().min(b.len());
    let (mut x, mut y) = (a.get(..n).unwrap_or(&[]), b.get(..n).unwrap_or(&[]));
    let mut i = 0usize;
    while let (Some((p, xr)), Some((q, yr))) = (x.split_first_chunk::<8>(), y.split_first_chunk::<8>()) {
        let (p, q) = (u64::from_le_bytes(*p), u64::from_le_bytes(*q));
        if p != q {
            return (
                if word_below(p, q) { Ordering::Less } else { Ordering::Greater },
                i.wrapping_add(diff_byte(p, q)),
            );
        }
        (x, y) = (xr, yr);
        i = i.wrapping_add(8);
    }
    // The last bytes, fewer than 8 and as many in each, zero-padded alike.
    let (p, q) = (word_padded(x, 0), word_padded(y, 0));
    if p != q {
        return (if word_below(p, q) { Ordering::Less } else { Ordering::Greater }, i.wrapping_add(diff_byte(p, q)));
    }
    (Ordering::Equal, n)
}

/// Strings of at most this many common bytes compare through `bytes_cmp`; `bytes_cmp_long` pays off past it.
pub const LONG_BYTES: usize = 64;

/// `bytes_cmp` for long strings (past `LONG_BYTES`): the same answer, both sides cut into exact 8-byte
/// chunks first, so the loop over them carries no bounds test per word. On short strings its set-up costs
/// more than it saves. Out of line: inlined, its loop is compiled looser.
#[inline(never)]
pub fn bytes_cmp_long(a: &[u8], b: &[u8]) -> (Ordering, usize) {
    let n = a.len().min(b.len());
    let (a, b) = (a.get(..n).unwrap_or(&[]), b.get(..n).unwrap_or(&[]));
    let (ca, cb) = (a.chunks_exact(8), b.chunks_exact(8));
    let (ra, rb) = (ca.remainder(), cb.remainder());
    let mut i = 0usize;
    for (x, y) in ca.zip(cb) {
        let p = x.first_chunk::<8>().map_or(0, |w| u64::from_le_bytes(*w));
        let q = y.first_chunk::<8>().map_or(0, |w| u64::from_le_bytes(*w));
        if p != q {
            return (
                if word_below(p, q) { Ordering::Less } else { Ordering::Greater },
                i.wrapping_add(diff_byte(p, q)),
            );
        }
        i = i.wrapping_add(8);
    }
    let (p, q) = (word_padded(ra, 0), word_padded(rb, 0));
    if p != q {
        return (if word_below(p, q) { Ordering::Less } else { Ordering::Greater }, i.wrapping_add(diff_byte(p, q)));
    }
    (Ordering::Equal, n)
}

/// `bytes_cmp`, or for strings whose common length passes `LONG_BYTES` their first two words compared here
/// (where most comparisons of keys that share a known prefix end) and the rest by `bytes_cmp_long`.
#[inline(always)]
pub fn bytes_cmp_any(a: &[u8], b: &[u8]) -> (Ordering, usize) {
    if a.len().min(b.len()) <= LONG_BYTES {
        return bytes_cmp(a, b);
    }
    let mut i = 0usize;
    while i < 16 {
        // Both strings are longer than 64 bytes: both words lie inside them.
        let (p, q) = (le64(a, i).unwrap_or(0), le64(b, i).unwrap_or(0));
        if p != q {
            return (
                if word_below(p, q) { Ordering::Less } else { Ordering::Greater },
                i.wrapping_add(diff_byte(p, q)),
            );
        }
        i = i.wrapping_add(8);
    }
    let (o, n) = bytes_cmp_long(a.get(16..).unwrap_or(&[]), b.get(16..).unwrap_or(&[]));
    (o, n.wrapping_add(16))
}

/// The memcmp order of byte strings, a proper prefix first: the order of `<[u8]>::cmp`. Two keys of 8
/// bytes compare as one word each.
#[inline(always)]
pub fn cmp_bytes(a: &[u8], b: &[u8]) -> Ordering {
    if let (Some(x), Some(y)) = (a.first_chunk::<8>(), b.first_chunk::<8>()) {
        let (p, q) = (u64::from_le_bytes(*x), u64::from_le_bytes(*y));
        if p != q {
            return if word_below(p, q) { Ordering::Less } else { Ordering::Greater };
        }
        if a.len() == 8 && b.len() == 8 {
            return Ordering::Equal;
        }
    }
    cmp_bytes_rest(a, b)
}

/// `cmp_bytes` past its first word.
#[inline(never)]
fn cmp_bytes_rest(a: &[u8], b: &[u8]) -> Ordering {
    bytes_cmp(a, b).0.then(a.len().cmp(&b.len()))
}

/// The position of a key that fits the key area W = 8·N among the c entries of a node `node` whose key parts
/// (the W-byte key area, then the 8-byte big-endian field) start at byte 8 and follow each other every
/// KS = W + 8 bytes: `Ok(i)` when entry i equals the key, else `Err(j)`, j the entries below it. The lower
/// midpoint, the stored entry on the left, as the tree's search. Entries whose key areas equal the key are
/// ordered by `rank`, given the field's 8 bytes as a little-endian word (its flag byte is the word's low
/// byte). `None` when an entry read does not lie inside `node`.
#[inline(always)]
pub fn search_fits<const N: usize, const KS: usize, F: Fn(u64) -> Ordering>(
    node: &[u8],
    c: usize,
    k: &KeyWords,
    rank: F,
) -> Option<Result<usize, usize>> {
    const { assert!(KS == N * 8 + 8 && N >= 1 && N <= KEY_WORDS) };
    let (mut lo, mut hi) = (0usize, c);
    while lo < hi {
        // lo and hi are at most a node's capacity, below 4,096.
        let mid = lo.wrapping_add(hi).wrapping_shr(1);
        let e = node.get(mid.wrapping_mul(KS).wrapping_add(8)..)?.first_chunk::<KS>()?;
        let mut o = Ordering::Equal;
        let mut i = 0usize;
        while i < N {
            let a = le64(e, i.wrapping_mul(8)).unwrap_or(0);
            let b = k.get(i);
            if a != b {
                o = if word_below(a, b) { Ordering::Less } else { Ordering::Greater };
                break;
            }
            i = i.wrapping_add(1);
        }
        if o == Ordering::Equal {
            o = rank(le64(e, N.wrapping_mul(8)).unwrap_or(0));
        }
        match o {
            Ordering::Less => lo = mid.wrapping_add(1),
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Some(Ok(mid)),
        }
    }
    Some(Err(lo))
}
