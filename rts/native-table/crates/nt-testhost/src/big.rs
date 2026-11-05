//! Integers of any size for checking sums: two's complement, little-endian, without redundant sign bytes
//! (zero is no byte), the form `Table::sum` hands out past i128.

use native_table::table::totals::Total;

/// An integer of any size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Big(pub Vec<u8>);

impl Big {
    fn trim(mut b: Vec<u8>) -> Big {
        while let [.., below, hi] = b[..] {
            if (hi == 0 && below & 0x80 == 0) || (hi == 0xFF && below & 0x80 != 0) {
                b.pop();
            } else {
                break;
            }
        }
        if b == [0] {
            b.clear();
        }
        Big(b)
    }

    fn ext(&self) -> u8 {
        if self.0.last().is_some_and(|&x| x & 0x80 != 0) {
            0xFF
        } else {
            0
        }
    }

    pub fn from_i128(x: i128) -> Big {
        Big::trim(x.to_le_bytes().to_vec())
    }

    pub fn from_u128(x: u128) -> Big {
        let mut b = x.to_le_bytes().to_vec();
        b.push(0);
        Big::trim(b)
    }

    /// The value of a canonical LEB128 form, signed or not.
    pub fn from_leb(leb: &[u8], signed: bool) -> Big {
        let neg = signed && leb.last().is_some_and(|&x| x & 0x40 != 0);
        let mut b = Vec::with_capacity(leb.len() * 7 / 8 + 2);
        let (mut acc, mut bits) = (0u64, 0u32);
        for &x in leb {
            acc |= u64::from(x & 0x7F) << bits;
            bits += 7;
            while bits >= 8 {
                b.push(acc as u8);
                acc >>= 8;
                bits -= 8;
            }
        }
        if neg {
            acc |= u64::MAX << bits;
        }
        b.push(acc as u8);
        b.push(if neg { 0xFF } else { 0 });
        Big::trim(b)
    }

    pub fn add(&self, o: &Big) -> Big {
        let n = self.0.len().max(o.0.len()) + 1;
        let (ea, eb) = (self.ext(), o.ext());
        let mut out = Vec::with_capacity(n);
        let mut carry = 0u16;
        for i in 0..n {
            let s = u16::from(*self.0.get(i).unwrap_or(&ea)) + u16::from(*o.0.get(i).unwrap_or(&eb)) + carry;
            out.push(s as u8);
            carry = s >> 8;
        }
        Big::trim(out)
    }

    pub fn neg(&self) -> Big {
        let mut b: Vec<u8> = self.0.iter().map(|x| !x).collect();
        b.push(!self.ext());
        Big::trim(b).add(&Big::from_i128(1))
    }

    pub fn sub(&self, o: &Big) -> Big {
        self.add(&o.neg())
    }

    /// The order of two integers.
    pub fn order(&self, o: &Big) -> std::cmp::Ordering {
        let d = self.sub(o);
        if d.0.is_empty() {
            std::cmp::Ordering::Equal
        } else if d.ext() == 0xFF {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }

    /// The value of a `Table::sum` result whose big bytes lie in `buf`.
    pub fn of_total(t: Total, buf: &[u8]) -> Big {
        match t {
            Total::Small(x) => Big::from_i128(x),
            Total::Big(n) => {
                let b = Big::trim(buf[..n].to_vec());
                assert_eq!(b.0.len(), n, "a big sum is handed out without redundant sign bytes");
                assert!(n > 16, "a sum that fits i128 is Small");
                b
            }
        }
    }

    /// The nearest f64 (ties to even), through the decimal digits Rust parses exactly; a magnitude of more
    /// than 129 bytes is past 2^1024, an infinity.
    pub fn to_f64(&self) -> f64 {
        let neg = self.ext() == 0xFF;
        let mut mag = if neg { self.neg().0 } else { self.0.clone() };
        if Big::trim(mag.clone()).0.len() > 129 {
            return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
        }
        // Decimal digits by repeated division by 10.
        let mut digits = Vec::new();
        while mag.iter().any(|&x| x != 0) {
            let mut rem = 0u32;
            for x in mag.iter_mut().rev() {
                let cur = (rem << 8) | u32::from(*x);
                *x = (cur / 10) as u8;
                rem = cur % 10;
            }
            digits.push(b'0' + rem as u8);
        }
        if digits.is_empty() {
            return 0.0;
        }
        digits.reverse();
        let s = String::from_utf8(digits).unwrap();
        let f: f64 = s.parse().unwrap();
        if neg {
            -f
        } else {
            f
        }
    }
}
