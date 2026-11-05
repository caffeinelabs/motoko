//! `nt_hash64`: the crate's one hash, used for run filters, key spec hashes, plan hashes and
//! load finish hashes. h = 0x9E37_79B9_7F4A_7C15 ^ len; for each 8-byte little-endian word w of the input
//! (the last zero-padded) h = mix(h ^ w), mix being splitmix64's finalizer.

/// splitmix64's finalizer.
#[inline]
pub const fn mix(mut z: u64) -> u64 {
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z
}

/// `nt_hash64` of `b`.
pub fn nt_hash64(b: &[u8]) -> u64 {
    let mut h = 0x9E37_79B9_7F4A_7C15 ^ (b.len() as u64);
    let mut words = b.chunks_exact(8);
    for w in &mut words {
        let mut a = [0u8; 8];
        crate::mem::copy_bytes(&mut a, w);
        h = mix(h ^ u64::from_le_bytes(a));
    }
    let rest = words.remainder();
    if !rest.is_empty() {
        let mut a = [0u8; 8];
        if let Some(d) = a.get_mut(..rest.len()) {
            crate::mem::copy_bytes(d, rest);
        }
        h = mix(h ^ u64::from_le_bytes(a));
    }
    h
}

/// `nt_hash64` over bytes given in pieces, for inputs that are not contiguous. Feeding the pieces of `b`
/// in order, then `finish`, equals `nt_hash64(b)` when the length is given up front.
pub struct Hasher64 {
    h: u64,
    word: [u8; 8],
    fill: usize,
}

impl Hasher64 {
    /// A hasher for an input of `len` bytes in total.
    pub const fn new(len: u64) -> Self {
        Hasher64 { h: 0x9E37_79B9_7F4A_7C15 ^ len, word: [0; 8], fill: 0 }
    }

    /// Feeds `bytes`: a word at a time where the pieces so far end on a word, byte by byte otherwise.
    pub fn update(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        // Complete a word left open by the pieces before.
        while self.fill != 0 {
            let Some((&b, more)) = rest.split_first() else { return };
            self.byte(b);
            rest = more;
        }
        // Four words a step, then one.
        while let Some((q, more)) = rest.split_first_chunk::<32>() {
            let w = |o: usize| q.get(o..).and_then(|t| t.first_chunk::<8>()).map_or(0, |t| u64::from_le_bytes(*t));
            self.h = mix(mix(mix(mix(self.h ^ w(0)) ^ w(8)) ^ w(16)) ^ w(24));
            rest = more;
        }
        while let Some((w, more)) = rest.split_first_chunk::<8>() {
            self.h = mix(self.h ^ u64::from_le_bytes(*w));
            rest = more;
        }
        for &b in rest {
            self.byte(b);
        }
    }

    /// Feeds one byte.
    #[inline(always)]
    fn byte(&mut self, b: u8) {
        if let Some(slot) = self.word.get_mut(self.fill) {
            *slot = b;
        }
        self.fill = self.fill.wrapping_add(1);
        if self.fill == 8 {
            self.h = mix(self.h ^ u64::from_le_bytes(self.word));
            self.word = [0; 8];
            self.fill = 0;
        }
    }

    pub fn finish(mut self) -> u64 {
        if self.fill > 0 {
            self.h = mix(self.h ^ u64::from_le_bytes(self.word));
            self.fill = 0;
        }
        self.h
    }
}
