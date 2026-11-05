//! Keys: the golden vectors exactly, memcmp order equal to value order for every kind,
//! Nat and Int of any magnitude, E⁻¹ round trips, and malformed keys giving `Corrupt`, never a panic.

use std::cmp::Ordering;

use native_table::hash::{nt_hash64, Hasher64};
use native_table::key::*;
use native_table::{Corruption, Fault, ImageId};
use nt_testhost::rows::idl_hash;
use nt_testhost::{hex, unhex, VecSink, XorShift};

const IMG: ImageId = ImageId::Runtime;

// E⁻¹ of the kinds the table never decodes, for the round trips.

fn dec_i64(key: &[u8], pos: u64, width: u8, image: ImageId) -> Result<(i64, u64), Fault> {
    let (u, next) = dec_u64(key, pos, width, image)?;
    let shift = 64 - u32::from(width) * 8;
    Ok(((((u ^ (1u64 << (63 - shift))) << shift) as i64) >> shift, next))
}

fn dec_i128(key: &[u8], pos: u64) -> i128 {
    let p = pos as usize;
    (u128::from_be_bytes(key[p..p + 16].try_into().unwrap()) ^ (1u128 << 127)) as i128
}

/// The f64 bits a float key decodes to.
fn dec_f64(key: &[u8], pos: u64) -> u64 {
    let p = pos as usize;
    let k = u64::from_be_bytes(key[p..p + 8].try_into().unwrap());
    if k >> 63 != 0 {
        k & !(1 << 63)
    } else {
        !k
    }
}

/// A Nat or Int key's sign and big-endian magnitude, and the position after it.
fn dec_int(key: &[u8], pos: u64, out: &mut VecSink) -> (bool, u64) {
    let (negative, n, at) = int_header(key, pos, IMG).unwrap();
    let m = &key[at as usize..(at + n) as usize];
    out.0.extend(m.iter().map(|&b| if negative { !b } else { b }));
    (negative, at + n)
}

fn key<F: FnOnce(&mut VecSink) -> Result<(), Fault>>(f: F) -> Vec<u8> {
    let mut s = VecSink::default();
    f(&mut s).unwrap();
    s.0
}

fn eq_hex(got: &[u8], want: &str) {
    assert_eq!(hex(got), hex(&unhex(want)), "want {want}");
}

#[test]
fn golden_vectors_of_6_5() {
    // Int
    for (v, h) in [(-65_536i64, "7C FE FF FF"), (-256, "7D FE FF"), (-255, "7E 00"), (-1, "7E FE")] {
        eq_hex(&key(|s| enc_int_i64(s, v)), h);
    }
    // Nat/Int
    for (v, h) in [(0u64, "80"), (1, "81 01"), (255, "81 FF"), (256, "82 01 00")] {
        eq_hex(&key(|s| enc_nat_u64(s, v)), h);
        eq_hex(&key(|s| enc_int_i64(s, v as i64)), h);
    }
    eq_hex(&key(|s| enc_nat_u64(s, (1 << 56) - 1)), "87 FF FF FF FF FF FF FF");
    eq_hex(&key(|s| enc_nat_u64(s, 1 << 56)), "88 01 00 00 00 00 00 00 00");
    let two64 = [1u8, 0, 0, 0, 0, 0, 0, 0, 0];
    eq_hex(&key(|s| enc_int_be(s, false, &two64)), "89 01 00 00 00 00 00 00 00 00");
    // A 127-byte magnitude and its negation.
    let mut r = XorShift(7);
    let mut m: Vec<u8> = (0..127).map(|_| r.next_u64() as u8).collect();
    m[0] |= 1;
    let k = key(|s| enc_int_be(s, false, &m));
    assert_eq!(&k[..3], &[0xFF, 0x81, 0x7F]);
    assert_eq!(&k[3..], &m[..]);
    let k = key(|s| enc_int_be(s, true, &m));
    assert_eq!(&k[..3], &[0x00, 0x7E, 0x80]);
    assert!(k[3..].iter().zip(&m).all(|(a, b)| *a == !*b));
    // Float
    let f = |b: u64| key(|s| enc_f64(s, b));
    eq_hex(&f(0xFFF8_0000_0000_0001), "00 00 00 00 00 00 00 00");
    eq_hex(&f(f64::NEG_INFINITY.to_bits()), "00 0F FF FF FF FF FF FF");
    eq_hex(&f((-1.0f64).to_bits()), "40 0F FF FF FF FF FF FF");
    eq_hex(&f(0x8000_0000_0000_0001), "7F FF FF FF FF FF FF FE");
    eq_hex(&f((-0.0f64).to_bits()), "80 00 00 00 00 00 00 00");
    eq_hex(&f(0.0f64.to_bits()), "80 00 00 00 00 00 00 00");
    eq_hex(&f(1), "80 00 00 00 00 00 00 01");
    eq_hex(&f(1.0f64.to_bits()), "BF F0 00 00 00 00 00 00");
    eq_hex(&f(f64::INFINITY.to_bits()), "FF F0 00 00 00 00 00 00");
    eq_hex(&f(0x7FF0_0000_0000_0123), "FF FF FF FF FF FF FF FF");
    // Float32
    eq_hex(&key(|s| enc_f32(s, 1.0f32.to_bits())), "BF 80 00 00");
    eq_hex(&key(|s| enc_f32(s, (-1.0f32).to_bits())), "40 7F FF FF");
    // Text
    for (t, h) in [
        (&b""[..], "00 00"),
        (b"a", "61 00 00"),
        (b"a\0", "61 00 FF 00 00"),
        (b"a\x01", "61 01 00 00"),
        (b"\0", "00 FF 00 00"),
    ] {
        eq_hex(&key(|s| enc_bytes(s, t)), h);
    }
    // Principal
    let mut want = vec![0u8; 30];
    eq_hex(&key(|s| enc_principal(s, &[])), &hex(&want));
    want[0] = 4;
    want[29] = 1;
    eq_hex(&key(|s| enc_principal(s, &[4])), &hex(&want));
    let cid = [0u8, 0, 0, 0, 0, 0, 0, 1, 1, 1];
    let k = key(|s| enc_principal(s, &cid));
    assert_eq!(&k[..10], &cid);
    assert!(k[10..29].iter().all(|&b| b == 0));
    assert_eq!(k[29], 10);
    assert!(k < key(|s| enc_principal(s, &[4])), "a canister id sorts before anonymous");
    // Candid idl hashes of names.
    for (name, h) in [("", 0u32), ("a", 0x61), ("foo", 0x004D_C706), ("Ok", 0x453C), ("Err", 0x0034_BF45)] {
        assert_eq!(idl_hash(name.as_bytes()), h);
    }
    assert_eq!(idl_hash(b"pending"), 736_925_783);
    assert_eq!(idl_hash(b"approved"), 2_464_173_687);
}

/// The numeric order of (negative, big-endian magnitude without leading zeros).
fn cmp_int(a: &(bool, Vec<u8>), b: &(bool, Vec<u8>)) -> Ordering {
    let norm = |x: &(bool, Vec<u8>)| -> (bool, Vec<u8>) {
        let m: Vec<u8> = x.1.iter().copied().skip_while(|&b| b == 0).collect();
        (x.0 && !m.is_empty(), m)
    };
    let (na, ma) = norm(a);
    let (nb, mb) = norm(b);
    let mag = ma.len().cmp(&mb.len()).then_with(|| ma.cmp(&mb));
    match (na, nb) {
        (false, false) => mag,
        (true, true) => mag.reverse(),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

#[test]
fn big_keys_sort_and_round_trip() {
    let mut r = XorShift(0x1234_5678_9ABC_DEF1);
    let mut vals: Vec<(bool, Vec<u8>)> = Vec::new();
    for n in [0usize, 1, 2, 7, 8, 9, 125, 126, 127, 128, 129, 255, 256, 10_000, 70_000] {
        for _ in 0..3 {
            let mut m: Vec<u8> = (0..n).map(|_| r.next_u64() as u8).collect();
            if let Some(b) = m.first_mut() {
                *b |= 1;
            }
            vals.push((false, m.clone()));
            vals.push((true, m));
        }
    }
    for _ in 0..300 {
        let n = r.below(20) as usize;
        vals.push((r.below(2) == 1, (0..n).map(|_| r.next_u64() as u8).collect()));
    }
    let keys: Vec<Vec<u8>> = vals.iter().map(|(neg, m)| key(|s| enc_int_be(s, *neg, m))).collect();
    for i in 0..vals.len() {
        // Prefix-free and order-preserving.
        for j in 0..vals.len() {
            if i != j && keys[i] != keys[j] {
                assert!(!keys[j].starts_with(&keys[i]), "prefix: {} vs {}", i, j);
            }
        }
        // E⁻¹ gives the value back.
        let mut out = VecSink::default();
        let (neg, end) = dec_int(&keys[i], 0, &mut out);
        assert_eq!(end, keys[i].len() as u64);
        assert_eq!(cmp_int(&(neg, out.0.clone()), &vals[i]), Ordering::Equal);
        assert_eq!(KeyKind::Int.skip(&keys[i], 0, IMG).unwrap(), keys[i].len() as u64);
    }
    let mut idx: Vec<usize> = (0..vals.len()).collect();
    idx.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
    for w in idx.windows(2) {
        assert_ne!(cmp_int(&vals[w[0]], &vals[w[1]]), Ordering::Greater);
    }
    for i in 0..vals.len() {
        for j in 0..vals.len() {
            if cmp_int(&vals[i], &vals[j]) == Ordering::Equal {
                assert_eq!(keys[i], keys[j]);
            }
        }
    }
}

/// LEB128 of a big-endian magnitude, unsigned.
fn uleb(m: &[u8]) -> Vec<u8> {
    if m.iter().all(|&b| b == 0) {
        return vec![0];
    }
    let mut bits: Vec<u8> = Vec::new();
    for &b in m.iter().rev() {
        for i in 0..8 {
            bits.push((b >> i) & 1);
        }
    }
    while bits.len() > 1 && *bits.last().unwrap() == 0 {
        bits.pop();
    }
    let mut out = Vec::new();
    for chunk in bits.chunks(7) {
        let mut g = 0u8;
        for (i, b) in chunk.iter().enumerate() {
            g |= b << i;
        }
        out.push(g | 0x80);
    }
    *out.last_mut().unwrap() &= 0x7F;
    out
}

/// Signed LEB128 of (negative, magnitude), canonical.
fn sleb(neg: bool, m: &[u8]) -> Vec<u8> {
    // Two's complement over enough bytes, little-endian.
    let n = m.len() + 1;
    let mut le: Vec<u8> = m.iter().rev().copied().collect();
    le.resize(n, 0);
    let mag_zero = le.iter().all(|&b| b == 0);
    if neg && !mag_zero {
        let mut carry = 1u16;
        for b in le.iter_mut() {
            let v = (!*b) as u16 + carry;
            *b = v as u8;
            carry = v >> 8;
        }
    }
    let mut bits: Vec<u8> = Vec::new();
    for &b in &le {
        for i in 0..8 {
            bits.push((b >> i) & 1);
        }
    }
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let mut g = 0u8;
        for i in 0..7 {
            let bit = bits.get(pos + i).copied().unwrap_or(*bits.last().unwrap());
            g |= bit << i;
        }
        pos += 7;
        let rest_all_sign = (pos..bits.len()).all(|k| bits[k] == ((g >> 6) & 1));
        let sign = (g >> 6) & 1;
        if rest_all_sign && sign == *bits.last().unwrap() {
            out.push(g);
            return out;
        }
        out.push(g | 0x80);
    }
}

#[test]
fn leb_sources_equal_magnitude_sources() {
    let mut r = XorShift(99);
    let mut cases: Vec<(bool, Vec<u8>)> = vec![
        (false, vec![]),
        (false, vec![1]),
        (true, vec![1]),
        (true, vec![0x80]),
        (true, vec![1, 0]),
        (false, vec![0x7F]),
        (true, vec![0x40]),
    ];
    for n in [1usize, 2, 7, 8, 9, 16, 17, 126, 127, 128, 300] {
        for _ in 0..4 {
            let mut m: Vec<u8> = (0..n).map(|_| r.next_u64() as u8).collect();
            m[0] |= 1;
            cases.push((r.below(2) == 1, m));
        }
    }
    for (neg, m) in cases {
        let want = key(|s| enc_int_be(s, neg, &m));
        if !neg {
            let got = key(|s| enc_int_leb(s, &uleb(&m), false, IMG));
            assert_eq!(got, want, "uleb {}", hex(&m));
        }
        let got = key(|s| enc_int_leb(s, &sleb(neg, &m), true, IMG));
        assert_eq!(got, want, "sleb neg={neg} {}", hex(&m));
    }
    // An unterminated LEB is an encoding fault.
    let mut s = VecSink::default();
    assert!(matches!(
        enc_int_leb(&mut s, &[0x80, 0x80], false, ImageId::Group(3)),
        Err(Fault::Corrupt(c)) if c.kind == Corruption::Encoding
    ));
}

/// A float's class order: −NaN < … < +NaN, −0.0 = +0.0.
fn float_rank(b: u64) -> (i32, f64) {
    let x = f64::from_bits(b);
    if x.is_nan() {
        return if b >> 63 == 1 { (-1, 0.0) } else { (1, 0.0) };
    }
    (0, if x == 0.0 { 0.0 } else { x })
}

#[test]
fn fixed_kinds_sort_by_memcmp() {
    let mut r = XorShift(0xDEAD_BEEF);
    // Signed and unsigned integers of every width.
    for w in [1u8, 2, 4, 8] {
        let bits = u32::from(w) * 8;
        let mut us: Vec<u64> = (0..500).map(|_| r.next_u64() >> (64 - bits)).collect();
        us.extend([0, (u128::from(u64::MAX) >> (64 - bits)) as u64]);
        let mut ks: Vec<(Vec<u8>, u64)> = us.iter().map(|&v| (key(|s| enc_u64(s, v, w)), v)).collect();
        ks.sort();
        assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
        for (k, v) in &ks {
            assert_eq!(dec_u64(k, 0, w, IMG).unwrap(), (*v, u64::from(w)));
        }
        let mut is: Vec<i64> = us.iter().map(|&v| ((v << (64 - bits)) as i64) >> (64 - bits)).collect();
        is.extend([i64::MIN >> (64 - bits), i64::MAX >> (64 - bits), 0, -1]);
        let mut ks: Vec<(Vec<u8>, i64)> = is.iter().map(|&v| (key(|s| enc_i64(s, v, w)), v)).collect();
        ks.sort();
        assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
        for (k, v) in &ks {
            assert_eq!(dec_i64(k, 0, w, IMG).unwrap(), (*v, u64::from(w)));
        }
        if w < 8 {
            let mut s = VecSink::default();
            assert!(enc_u64(&mut s, 1 << bits, w).is_err());
            assert!(enc_i64(&mut s, 1 << (bits - 1), w).is_err());
        }
    }
    // u128 and i128.
    let mut vs: Vec<i128> = (0..300).map(|_| ((r.next_u64() as i128) << 64 | r.next_u64() as i128)).collect();
    vs.extend([i128::MIN, i128::MAX, 0, -1]);
    let mut ks: Vec<(Vec<u8>, i128)> = vs.iter().map(|&v| (key(|s| enc_i128(s, v)), v)).collect();
    ks.sort();
    assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
    for (k, v) in &ks {
        assert_eq!(dec_i128(k, 0), *v);
    }
    let mut ks: Vec<(Vec<u8>, u128)> = vs.iter().map(|&v| (key(|s| enc_u128(s, v as u128)), v as u128)).collect();
    ks.sort();
    assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
    // Floats, with special values and random bits.
    let mut fs: Vec<u64> = (0..2_000).map(|_| r.next_u64()).collect();
    fs.extend([
        0,
        1 << 63,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        f64::NAN.to_bits(),
        (-f64::NAN).to_bits(),
        1,
        (1 << 63) | 1,
        f64::MIN_POSITIVE.to_bits(),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
    ]);
    let mut ks: Vec<(Vec<u8>, u64)> = fs.iter().map(|&b| (key(|s| enc_f64(s, b)), b)).collect();
    ks.sort();
    for p in ks.windows(2) {
        let (a, b) = (float_rank(p[0].1), float_rank(p[1].1));
        assert!(a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1), "{:x} {:x}", p[0].1, p[1].1);
        if p[0].0 == p[1].0 {
            assert_eq!(a.0, b.0);
            assert!(a.0 != 0 || a.1 == b.1);
        }
    }
    for (k, b) in &ks {
        let back = dec_f64(k, 0);
        let x = f64::from_bits(*b);
        if x.is_nan() {
            assert!(f64::from_bits(back).is_nan());
            assert_eq!(back >> 63, b >> 63);
            assert_eq!(back & 0x000F_FFFF_FFFF_FFFF, 0x000F_FFFF_FFFF_FFFF);
        } else if x == 0.0 {
            assert_eq!(back, 0);
        } else {
            assert_eq!(back, *b);
        }
    }
    let mut f32s: Vec<u32> = (0..1_000).map(|_| r.next_u64() as u32).collect();
    f32s.extend([0, 1 << 31, f32::INFINITY.to_bits(), f32::NAN.to_bits(), (-f32::NAN).to_bits()]);
    let mut ks: Vec<(Vec<u8>, u32)> = f32s.iter().map(|&b| (key(|s| enc_f32(s, b)), b)).collect();
    ks.sort();
    for p in ks.windows(2) {
        let wide = |b: u32| {
            float_rank(
                f64::from(f32::from_bits(b)).to_bits()
                    | if f32::from_bits(b).is_nan() { u64::from(b >> 31) << 63 } else { 0 },
            )
        };
        let (a, b) = (wide(p[0].1), wide(p[1].1));
        assert!(a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1));
    }
    // Chars and Bool.
    let mut cs: Vec<char> = (0..2_000).filter_map(|_| char::from_u32(r.below(0x11_0000) as u32)).collect();
    cs.extend(['\0', '\u{D7FF}', '\u{E000}', '\u{10FFFF}']);
    let mut ks: Vec<(Vec<u8>, char)> = cs.iter().map(|&c| (key(|s| enc_char(s, c)), c)).collect();
    ks.sort();
    assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
    for (k, c) in &ks {
        assert_eq!(dec_char(k, 0, IMG).unwrap().0, *c);
    }
    assert!(key(|s| enc_bool(s, false)) < key(|s| enc_bool(s, true)));
}

#[test]
fn text_and_principals_sort_by_bytes() {
    let mut r = XorShift(5);
    let mut ts: Vec<Vec<u8>> =
        (0..1_500).map(|_| (0..r.below(12)).map(|_| [0u8, 0xFF, 1, b'a'][r.below(4) as usize]).collect()).collect();
    ts.extend([vec![], vec![0], vec![0, 0], vec![0, 0xFF], vec![0xFF, 0], vec![1u8; 100_000]]);
    let mut ks: Vec<(Vec<u8>, Vec<u8>)> = ts.iter().map(|t| (key(|s| enc_bytes(s, t)), t.clone())).collect();
    ks.sort();
    assert!(ks.windows(2).all(|p| p[0].1 <= p[1].1));
    for (k, t) in &ks {
        let mut out = VecSink::default();
        assert_eq!(dec_bytes(k, 0, &mut out, IMG).unwrap(), k.len() as u64);
        assert_eq!(&out.0, t);
    }
    // Pieces cut anywhere give the same key.
    let t = b"ab\0\0cd\0";
    for cut in 0..=t.len() {
        let k = key(|s| {
            enc_bytes_piece(s, &t[..cut])?;
            enc_bytes_piece(s, &t[cut..])?;
            enc_bytes_end(s)
        });
        assert_eq!(k, key(|s| enc_bytes(s, t)));
    }
    // Principals: shorter prefix first, by bytes.
    let mut ps: Vec<Vec<u8>> = (0..500).map(|_| (0..r.below(30)).map(|_| r.below(3) as u8).collect()).collect();
    ps.push(vec![]);
    let mut ks: Vec<(Vec<u8>, Vec<u8>)> = ps.iter().map(|p| (key(|s| enc_principal(s, p)), p.clone())).collect();
    ks.sort();
    for p in ks.windows(2) {
        // Byte order with zero padding: equal to comparing the padded bytes, then the length.
        let pad = |v: &Vec<u8>| {
            let mut x = v.clone();
            x.resize(29, 0);
            (x, v.len())
        };
        assert!(pad(&p[0].1) <= pad(&p[1].1));
    }
    for (k, p) in &ks {
        let (b, n, end) = dec_principal(k, 0, IMG).unwrap();
        assert_eq!((&b[..n as usize], end), (&p[..], 30));
    }
    let mut s = VecSink::default();
    assert!(enc_principal(&mut s, &[0u8; 30]).is_err());
}

#[test]
fn key_specs() {
    use native_table::table::cell::{enc_key, Val};
    use native_table::table::layout::{LeafInfo, Ty};
    let leaf = |id, ty, nullable| LeafInfo { ordinal: 0, id, ty, nullable, key: true, path: 0, added: 0, dropped: 0 };
    let p = |kind, nullable| KeyPart { leaf: 0, kind, nullable };
    // Composite keys: ("a", 2) against ("ab", 1), and a null second part first.
    let parts = [p(KeyKind::Text, false), p(KeyKind::Nat, true)];
    let spec = KeySpec { parts: &parts };
    let (text, nat) = (leaf(1, Ty::Text, false), leaf(2, Ty::Nat, true));
    let enc = |a: &[u8], b: Val<'_>| {
        key(|s| {
            enc_key(s, &text, &Val::Bytes(a))?;
            enc_key(s, &nat, &b)
        })
    };
    let (k1, k2, k3) = (enc(b"a", Val::U(2)), enc(b"ab", Val::U(1)), enc(b"a", Val::Null));
    assert!(k3 < k1 && k1 < k2);
    assert_eq!(spec.end(&k1, IMG).unwrap(), k1.len() as u64);
    assert_eq!(spec.end(&k3, IMG).unwrap(), k3.len() as u64);
    // A null on a non-nullable part, or a value out of the part's range, names the part's leaf.
    let part = leaf(9, Ty::U(1), false);
    let mut s = VecSink::default();
    assert_eq!(enc_key(&mut s, &part, &Val::Null), Err(Fault::BadValue { leaf: 9 }));
    assert_eq!(enc_key(&mut s, &part, &Val::U(256)), Err(Fault::BadValue { leaf: 9 }));
    // The hash changes with the parts and flags.
    let h = spec.hash(1);
    assert_ne!(h, spec.hash(2));
    assert_ne!(h, KeySpec { parts: &[parts[1], parts[0]] }.hash(1));
}

#[test]
fn hash_in_pieces_equals_whole() {
    let mut r = XorShift(3);
    for n in [0usize, 1, 7, 8, 9, 15, 16, 17, 100, 1_000] {
        let b: Vec<u8> = (0..n).map(|_| r.next_u64() as u8).collect();
        for cut in [0, n / 3, n / 2, n] {
            let mut h = Hasher64::new(n as u64);
            h.update(&b[..cut]);
            h.update(&b[cut..]);
            assert_eq!(h.finish(), nt_hash64(&b));
        }
        // Three pieces, cut on and off word boundaries.
        for (c1, c2) in [(1, 9), (3, 5), (8, 16), (7, 8), (5, 21), (13, 14)] {
            let (c1, c2) = (c1.min(n), c2.min(n));
            let mut h = Hasher64::new(n as u64);
            h.update(&b[..c1]);
            h.update(&b[c1..c2]);
            h.update(&b[c2..]);
            assert_eq!(h.finish(), nt_hash64(&b), "{n} bytes cut at {c1} and {c2}");
        }
    }
    assert_ne!(nt_hash64(b""), nt_hash64(b"\0"));
}

#[test]
fn malformed_keys_fault_and_never_panic() {
    let img = ImageId::Run { index: 2, start_seq: 5 };
    let is_corrupt = |r: Result<(), Fault>| matches!(r, Err(Fault::Corrupt(c)) if c.image == img);
    assert!(is_corrupt(dec_bool(&[2], 0, img).map(|_| ())));
    assert!(is_corrupt(dec_char(&[0, 0, 0xD8, 0], 0, img).map(|_| ())));
    assert!(is_corrupt(dec_char(&[0, 0x11, 0, 0], 0, img).map(|_| ())));
    assert!(is_corrupt(dec_bytes(b"a\0\x01", 0, &mut VecSink::default(), img).map(|_| ())));
    assert!(is_corrupt(dec_bytes(b"abc", 0, &mut VecSink::default(), img).map(|_| ())));
    assert!(is_corrupt(dec_bytes(b"a\0", 0, &mut VecSink::default(), img).map(|_| ())));
    let mut p = [0u8; 30];
    p[29] = 30;
    assert!(is_corrupt(dec_principal(&p, 0, img).map(|_| ())));
    assert!(is_corrupt(KeyKind::Int.skip(&[0x7F], 0, img).map(|_| ())));
    assert!(is_corrupt(KeyKind::Int.skip(&[0x83, 1], 0, img).map(|_| ())));
    assert!(is_corrupt(KeyKind::Int.skip(&[0xFF, 0x81, 0x10], 0, img).map(|_| ())));
    assert!(is_corrupt(KeyKind::Int.skip(&[0xFF, 0x89], 0, img).map(|_| ())));
    assert!(is_corrupt(dec_u64(&[1, 2], 0, 4, img).map(|_| ())));
    // Random bytes through every decoder: a value or a Corrupt, never a panic.
    let mut r = XorShift(42);
    let kinds = [
        KeyKind::Bool,
        KeyKind::U8,
        KeyKind::U64,
        KeyKind::I32,
        KeyKind::U128,
        KeyKind::Char,
        KeyKind::Nat,
        KeyKind::Int,
        KeyKind::F64,
        KeyKind::F32,
        KeyKind::Text,
        KeyKind::Principal,
        KeyKind::Fixed(3),
        KeyKind::Unit,
    ];
    for _ in 0..20_000 {
        let n = r.below(40) as usize;
        let b: Vec<u8> =
            (0..n).map(|_| [0u8, 0xFF, 0x80, 0x7F, 1, 0x81][r.below(6) as usize] ^ (r.below(4) as u8)).collect();
        let pos = r.below(n as u64 + 2);
        for k in kinds {
            match k.skip(&b, pos, img) {
                Ok(end) => assert!(end <= b.len() as u64),
                Err(Fault::Corrupt(c)) => assert_eq!(c.image, img),
                Err(f) => panic!("{f:?}"),
            }
        }
        let _ = dec_bytes(&b, pos, &mut VecSink::default(), img);
        let _ = dec_principal(&b, pos, img);
        let _ = int_header(&b, pos, img);
    }
}

/// The escape of a Text or Blob, looked for a word at a time, equals the byte-at-a-time escape: `00` bytes
/// at every place of values up to 40 bytes, next to bytes whose word arithmetic borrows or carries (`01`,
/// `80`, `FF`), whole and cut into pieces anywhere, and random values with few and many `00` bytes; into a
/// growing sink (a run at a time) and into a slice (straight in while twice the piece fits what is left of it,
/// else a run at a time: slices of the key's length and of twice the value's), and a slice too short for the
/// key is `Scratch`.
#[test]
fn escape_equals_byte_at_a_time() {
    fn want(t: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for &b in t {
            if b == 0 {
                out.extend_from_slice(&[0x00, 0xFF]);
            } else {
                out.push(b);
            }
        }
        out.extend_from_slice(&[0x00, 0x00]);
        out
    }
    let check = |t: &[u8]| {
        assert_eq!(key(|s| enc_bytes(s, t)), want(t), "{}", hex(t));
        for cut in [1usize, 3, 7, 8, 9] {
            let got = key(|s| {
                for p in t.chunks(cut) {
                    enc_bytes_piece(s, p)?;
                }
                enc_bytes_end(s)
            });
            assert_eq!(got, want(t), "cut {cut}: {}", hex(t));
        }
        let w = want(t);
        for (size, cut) in
            [(w.len(), t.len().max(1)), (w.len(), 5), (2 * t.len() + 2, t.len().max(1)), (2 * t.len() + 2, 9)]
        {
            let mut buf = vec![0xEEu8; size];
            let mut s = SliceSink::new(&mut buf);
            for p in t.chunks(cut) {
                enc_bytes_piece(&mut s, p).unwrap();
            }
            enc_bytes_end(&mut s).unwrap();
            assert_eq!(s.written(), &w[..], "slice {size}, cut {cut}: {}", hex(t));
        }
        if let Some(short) = w.len().checked_sub(1) {
            let mut buf = vec![0u8; short];
            let mut s = SliceSink::new(&mut buf);
            let r = enc_bytes_piece(&mut s, t).and_then(|()| enc_bytes_end(&mut s));
            assert!(matches!(r, Err(Fault::Scratch { .. })), "short slice: {}", hex(t));
        }
    };
    for len in 0..=40usize {
        for fill in [0x01u8, 0x80, 0xFF, 0x61] {
            check(&vec![fill; len]);
            for z in 0..len {
                let mut t = vec![fill; len];
                t[z] = 0;
                check(&t);
                if z + 1 < len {
                    t[z + 1] = 0x01;
                    check(&t);
                    t[z + 1] = 0;
                    check(&t);
                }
            }
        }
    }
    let mut r = XorShift(0x5EED_0E5C);
    for _ in 0..2_000 {
        let len = r.below(300) as usize;
        let zeros = r.below(4);
        let t: Vec<u8> = (0..len).map(|_| if r.below(8) < zeros { 0 } else { r.next_u64() as u8 }).collect();
        check(&t);
    }
}

/// `leb_low`, which finds a form's first byte with a set group bit 8 bytes at a time, gives the byte-at-a-time
/// answer: windows of up to 40 bytes whose zero groups are `00` or `80` bytes (a continuation bit alone), a set
/// group at every place, any lowest set bit, any base; and 2,000 random windows.
#[test]
fn leb_low_equals_byte_at_a_time() {
    fn want(w: &[u8], base: u64) -> Option<u64> {
        let (j, b) = w.iter().enumerate().find(|&(_, &b)| b & 0x7F != 0)?;
        Some(((base + j as u64) * 7 + u64::from((b & 0x7F).trailing_zeros())) / 8)
    }
    for len in 0..=40usize {
        for fill in [0x00u8, 0x80] {
            for base in [0u64, 1, 7, 1_000_003] {
                let w = vec![fill; len];
                assert_eq!(leb_low(&w, base), want(&w, base), "{}", hex(&w));
                for at in 0..len {
                    for set in [0x01u8, 0x40, 0x7F, 0x81, 0xC0] {
                        let mut w = vec![fill; len];
                        w[at] = set;
                        assert_eq!(leb_low(&w, base), want(&w, base), "{base} {}", hex(&w));
                        if at + 1 < len {
                            w[at + 1] = 0x7F;
                            assert_eq!(leb_low(&w, base), want(&w, base), "{base} {}", hex(&w));
                        }
                    }
                }
            }
        }
    }
    let mut r = XorShift(0x1EB_10E5);
    for _ in 0..2_000 {
        let len = r.below(300) as usize;
        let set = r.below(len as u64 + 1) as usize;
        let w: Vec<u8> = (0..len).map(|i| if i < set { (r.below(2) as u8) << 7 } else { r.next_u64() as u8 }).collect();
        let base = r.next_u64() >> 20;
        assert_eq!(leb_low(&w, base), want(&w, base), "{base} {}", hex(&w));
    }
}
