//! Values: the LEB128 writers, skipping random type expressions, and the incremental UTF-8 validator.

use native_table::bytes::put_uleb;
use native_table::table::cell::put_sleb128;
use native_table::value::{skip_ty, ty, Utf8Stream, FRAME};
use nt_testhost::XorShift;

/// A type pool built in a Vec; each builder returns the expression's offset.
#[derive(Default)]
struct Pool(Vec<u8>);

#[derive(Clone)]
enum T {
    P(u8),
    Bytes(u16),
    Opt(Box<T>),
    Rec(Vec<(u32, u8, T)>),
    Tup(Vec<T>),
}

impl Pool {
    fn add(&mut self, t: &T) -> u32 {
        let at = self.0.len() as u32;
        self.emit(t);
        at
    }
    fn emit(&mut self, t: &T) {
        let b = &mut self.0;
        match t {
            T::P(c) => b.push(*c),
            T::Bytes(n) => {
                b.push(ty::BYTES);
                b.extend(n.to_le_bytes());
            }
            T::Opt(x) => {
                b.push(ty::OPTION);
                self.emit(x);
            }
            T::Rec(fields) => {
                b.push(ty::RECORD);
                b.extend((fields.len() as u16).to_le_bytes());
                b.extend(0u16.to_le_bytes());
                for (h, flags, x) in fields {
                    self.0.extend(h.to_le_bytes());
                    self.0.extend(0u32.to_le_bytes());
                    self.0.push(*flags);
                    self.0.extend([0u8; 8]);
                    self.emit(x);
                }
            }
            T::Tup(xs) => {
                b.push(ty::TUPLE);
                b.extend((xs.len() as u16).to_le_bytes());
                for x in xs {
                    self.emit(x);
                }
            }
        }
    }
}

fn scratch() -> Vec<u8> {
    vec![0u8; 512 * FRAME]
}

#[test]
fn leb_writers() {
    let mut b = [0u8; 19];
    assert_eq!(put_uleb(&mut b, 0, 300).unwrap(), 2);
    assert_eq!(&b[..2], &[0xAC, 0x02]);
    assert_eq!(put_sleb128(&mut b, -129), 2);
    assert_eq!(&b[..2], &[0xFF, 0x7E]);
    // Signed LEB128 round-trips, in its shortest form: no last byte only repeats the sign of the one before.
    for v in [0i64, 1, -1, 63, 64, -64, -65, i64::MAX, i64::MIN] {
        let n = put_sleb128(&mut b, i128::from(v));
        let (mut x, mut shift) = (0i128, 0);
        for &byte in &b[..n] {
            x |= i128::from(byte & 0x7F) << shift;
            shift += 7;
        }
        if b[n - 1] & 0x40 != 0 {
            x -= 1i128 << shift;
        }
        assert_eq!(x, i128::from(v), "{v}");
        assert!(b[..n - 1].iter().all(|&c| c & 0x80 != 0) && b[n - 1] & 0x80 == 0, "{v}");
        if n > 1 {
            let (prev, last) = (b[n - 2], b[n - 1]);
            assert!(!((last == 0 && prev & 0x40 == 0) || (last == 0x7F && prev & 0x40 != 0)), "{v}");
        }
    }
}

#[test]
fn random_types_skip_to_their_end() {
    let mut r = XorShift(77);
    let prims = [
        ty::BOOL,
        ty::U8,
        ty::I16,
        ty::U32,
        ty::F64,
        ty::NAT,
        ty::INT,
        ty::CHAR,
        ty::TEXT,
        ty::BLOB,
        ty::PRINCIPAL,
        ty::U128,
        ty::UNIT,
    ];
    fn gen(r: &mut XorShift, prims: &[u8], depth: u32) -> T {
        if depth == 0 || r.below(3) == 0 {
            return T::P(prims[r.below(prims.len() as u64) as usize]);
        }
        match r.below(4) {
            0 => T::Opt(Box::new(gen(r, prims, depth - 1))),
            1 => T::Tup((0..r.below(4)).map(|_| gen(r, prims, depth - 1)).collect()),
            2 => {
                T::Rec((0..r.below(4)).map(|i| (i as u32, (r.below(4) == 0) as u8, gen(r, prims, depth - 1))).collect())
            }
            _ => T::Bytes(r.below(5) as u16),
        }
    }
    for _ in 0..3_000 {
        let mut p = Pool::default();
        let t = p.add(&gen(&mut r, &prims, 4));
        assert_eq!(skip_ty(&p.0, t, &mut scratch()).unwrap(), p.0.len() as u32);
    }
}

#[test]
fn utf8_across_pieces() {
    // Every split of strings with 1- to 4-byte characters is accepted.
    let s = "aé€😀z".as_bytes();
    for i in 0..=s.len() {
        for j in i..=s.len() {
            let mut v = Utf8Stream::new();
            assert!(v.feed(&s[..i]) && v.feed(&s[i..j]) && v.feed(&s[j..]) && v.finish(), "{i} {j}");
        }
    }
    // A truncated character at the value's end is refused; so are invalid sequences split anywhere.
    for cut in 1..4 {
        let mut v = Utf8Stream::new();
        assert!(v.feed(&"😀".as_bytes()[..cut]));
        assert!(!v.finish());
    }
    for bad in [&[0xC3u8, 0x28][..], &[0xED, 0xA0, 0x80], &[0xF8, 0x80], &[0xC0, 0x80], &[0xF4, 0x90, 0x80, 0x80]] {
        for i in 0..=bad.len() {
            let mut v = Utf8Stream::new();
            let ok = v.feed(&bad[..i]) && v.feed(&bad[i..]) && v.finish();
            assert!(!ok, "{bad:x?} split at {i}");
        }
    }
    // Random bytes in random pieces agree with from_utf8 on the whole.
    let mut r = XorShift(11);
    let alphabet: Vec<u8> = "aé€😀".bytes().chain([0x80, 0xFF, 0xC3]).collect();
    for _ in 0..20_000 {
        let n = r.below(16) as usize;
        let b: Vec<u8> = (0..n).map(|_| alphabet[r.below(alphabet.len() as u64) as usize]).collect();
        let mut v = Utf8Stream::new();
        let mut ok = true;
        let mut at = 0;
        while at < n && ok {
            let k = 1 + r.below(4) as usize;
            let end = (at + k).min(n);
            ok = v.feed(&b[at..end]);
            at = end;
        }
        assert_eq!(ok && v.finish(), std::str::from_utf8(&b).is_ok(), "{b:x?}");
    }
}
