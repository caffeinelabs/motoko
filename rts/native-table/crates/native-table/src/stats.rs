//! Exact sums for the integer leaves' totals: the i192 sums of u128 and i128 leaves; the running records that
//! hold the totals (`running`); and a rewrite's work space (`space`).

pub mod running;
pub mod space;

/// The exact sum of u128 and i128 values: a 192-bit two's-complement integer, little-endian limbs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct I192(pub [u64; 3]);

impl I192 {
    fn add_limbs(&mut self, v: [u64; 3]) {
        let mut carry = 0u64;
        for (l, a) in self.0.iter_mut().zip(v) {
            let (s1, c1) = l.overflowing_add(a);
            let (s2, c2) = s1.overflowing_add(carry);
            *l = s2;
            carry = u64::from(c1) | u64::from(c2);
        }
    }

    fn neg_limbs(v: [u64; 3]) -> [u64; 3] {
        let mut out = [0u64; 3];
        let mut carry = 1u64;
        for (o, x) in out.iter_mut().zip(v) {
            let (s, c) = (!x).overflowing_add(carry);
            *o = s;
            carry = u64::from(c);
        }
        out
    }

    pub fn apply_u128(&mut self, v: u128, subtract: bool) {
        // The low and high halves.
        #[allow(clippy::cast_possible_truncation)]
        let limbs = [v as u64, (v >> 64) as u64, 0];
        self.add_limbs(if subtract { Self::neg_limbs(limbs) } else { limbs });
    }

    pub fn apply_i128(&mut self, v: i128, subtract: bool) {
        let ext = if v < 0 { u64::MAX } else { 0 };
        let u = v as u128;
        #[allow(clippy::cast_possible_truncation)]
        let limbs = [u as u64, (u >> 64) as u64, ext];
        self.add_limbs(if subtract { Self::neg_limbs(limbs) } else { limbs });
    }
}
