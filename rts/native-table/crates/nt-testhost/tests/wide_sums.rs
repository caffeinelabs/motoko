//! Wide sums: the i192 sums of the u128 and i128 leaves' totals.

use native_table::stats::I192;
use nt_testhost::XorShift;

#[test]
fn i192_sums() {
    let mut r = XorShift(8);
    let mut s = I192::default();
    let mut vals = Vec::new();
    for _ in 0..1_000 {
        let v = ((r.next_u64() as i128) << 64) | r.next_u64() as i128;
        vals.push(v);
        s.apply_i128(v, false);
    }
    // Sign checks against a 2-limb reference of the low 128 bits.
    let low: i128 = vals.iter().fold(0i128, |a, v| a.wrapping_add(*v));
    assert_eq!(((s.0[1] as u128) << 64 | s.0[0] as u128) as i128, low);
    for v in &vals {
        s.apply_i128(*v, true);
    }
    assert_eq!(s, I192::default());
    s.apply_u128(u128::MAX, false);
    s.apply_u128(u128::MAX, false);
    assert_eq!(s.0, [u64::MAX - 1, u64::MAX, 1]);
    assert!(!s.is_negative());
    s.apply_u128(u128::MAX, true);
    s.apply_u128(u128::MAX, true);
    s.apply_i128(-5, false);
    assert!(s.is_negative());
}
