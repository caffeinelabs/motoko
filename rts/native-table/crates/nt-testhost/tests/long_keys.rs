//! The key bytes of long Nat and Int index keys read through windows: `enc_int_leb_read` and `LebFacts` give
//! `enc_int_leb`'s bytes.

use nt_testhost::{VecSink, XorShift};

// ---------------------------------------------------------------- the walk over the tree

// ---------------------------------------------------------------- huge numbers read in windows

/// A canonical (signed) LEB128 form of `n` bytes, its 7-bit groups drawn from `r` (the low ones zero when
/// `zeros`); a negative one when `neg`.
fn leb(r: &mut XorShift, n: usize, signed: bool, neg: bool, zeros: usize) -> Vec<u8> {
    let mut out: Vec<u8> = (0..n).map(|i| if i < zeros { 0x80 } else { 0x80 | r.below(128) as u8 }).collect();
    out[n - 1] = match (signed, neg) {
        (false, _) => 1 + r.below(127) as u8,
        (true, false) => 1 + r.below(0x3F) as u8,
        (true, true) => 0x40 + r.below(0x3F) as u8,
    };
    out
}

/// `enc_int_leb_read` gives `enc_int_leb`'s bytes, its form read through windows: Nats and Ints, positive and
/// negative, short and of 20,000 bytes, with their low groups zero (where a negative's negation starts) or not.
#[test]
fn leb_read_equals_enc_int_leb() {
    use native_table::key::{enc_int_leb, enc_int_leb_read};
    use native_table::ImageId;
    let mut r = XorShift(0x1EB);
    for n in [1usize, 2, 9, 10, 300, 5_000, 20_000] {
        for (signed, neg) in [(false, false), (true, false), (true, true)] {
            for zeros in [0, n / 2, n - 1] {
                let form = leb(&mut r, n, signed, neg, zeros);
                let mut want = VecSink::default();
                enc_int_leb(&mut want, &form, signed, ImageId::Runtime).unwrap();
                let mut got = VecSink::default();
                let read = |at: u64, out: &mut [u8]| {
                    let at = at as usize;
                    out.copy_from_slice(&form[at..at + out.len()]);
                    Ok(())
                };
                enc_int_leb_read(&mut got, n as u64, signed, ImageId::Runtime, read).unwrap();
                assert!(got.0 == want.0, "{n} bytes, signed {signed}, negative {neg}, {zeros} low groups zero");
            }
        }
    }
}

/// `LebFacts` gives `enc_int_leb`'s bytes from windows of the form alone: a negative's lowest non-zero byte
/// found in windows of any size, the facts from the form's last bytes, and the magnitude's key bytes written
/// in runs of any size, each from the form's bytes its span names. Nats and Ints, positive and negative,
/// short and of 20,000 bytes, with their low groups zero (where a negative's negation starts) or not.
#[test]
fn leb_facts_equal_enc_int_leb() {
    use native_table::key::{enc_int_leb, leb_low, LebFacts, LEB_TOP};
    use native_table::ImageId;
    let mut r = XorShift(0x1EF);
    for n in [1usize, 2, 9, 10, 17, 18, 24, 25, 300, 5_000, 20_000] {
        for (signed, neg) in [(false, false), (true, false), (true, true)] {
            for zeros in [0, n / 2, n - 1] {
                let form = leb(&mut r, n, signed, neg, zeros);
                let mut want = VecSink::default();
                enc_int_leb(&mut want, &form, signed, ImageId::Runtime).unwrap();
                let (mut low, mut at) = (None, 0usize);
                while neg && low.is_none() {
                    let w = (1 + r.below(800) as usize).min(n - at);
                    low = leb_low(&form[at..at + w], at as u64);
                    at += w;
                }
                let top = &form[n - n.min(LEB_TOP)..];
                let f = LebFacts::new(n as u64, signed, low.unwrap_or(0), top, ImageId::Runtime).unwrap();
                let mut got = VecSink::default();
                f.head(&mut got).unwrap();
                let mut hi = f.mag_len;
                while hi > 0 {
                    let lo = hi.saturating_sub(1 + r.below(700));
                    let (a, b) = f.span(lo, hi);
                    let mut out = vec![0u8; (hi - lo) as usize];
                    f.key_bytes(&form[a as usize..b as usize], a, lo, &mut out).unwrap();
                    got.0.extend_from_slice(&out);
                    hi = lo;
                }
                assert!(got.0 == want.0, "{n} bytes, signed {signed}, negative {neg}, {zeros} low groups zero");
            }
        }
    }
}
