import Prim "mo:⛔";

// intAddMod: (7 + 5) mod 10 = 2
assert Prim.intAddMod(7, 5, 10) == 2;

// intSubMod: (3 - 7) mod 10 = 6 (LibTomMath returns positive remainder)
assert Prim.intSubMod(3, 7, 10) == 6;

// intMulMod: (7 * 8) mod 10 = 6
assert Prim.intMulMod(7, 8, 10) == 6;

// intPowMod: 2^10 mod 1000 = 24
assert Prim.intPowMod(2, 10, 1000) == 24;

// intPowMod: 3^13 mod 50 = 23
assert Prim.intPowMod(3, 13, 50) == 23;

// Large modular exponentiation (crypto-relevant)
// Fermat's little theorem: 2^(p-1) mod p = 1 for prime p
assert Prim.intPowMod(2, 12, 13) == 1;

// intInvMod: 3 * 7 = 21 ≡ 1 (mod 10), so 3⁻¹ ≡ 7 (mod 10)
assert Prim.intInvMod(3, 10) == 7;
// intInvMod: 5⁻¹ ≡ 3 (mod 7) since 5*3 = 15 ≡ 1 (mod 7)
assert Prim.intInvMod(5, 7) == 3;
// Round-trip: a * a⁻¹ ≡ 1 (mod m)
assert Prim.intMulMod(11, Prim.intInvMod(11, 100), 100) == 1;

// intSqr: trivial cases
assert Prim.intSqr(0) == 0;
assert Prim.intSqr(1) == 1;
assert Prim.intSqr(7) == 49;
assert Prim.intSqr(-7) == 49;
// Large square — fits Motoko Int but exercises the libtommath path
assert Prim.intSqr(1234567890) == 1524157875019052100;

// Montgomery primitives — round-trips that hold for any consistent digit_bit
// (so the same test passes in both interpreter (digit_bit=28) and compiled
// (digit_bit=28 on wasm32, 60 on wasm64) — exact rho/R values differ across
// targets but cancel in round-trip).
let m = 101;          // odd prime, required for libtommath Montgomery
let R = Prim.intMontgomeryCalcNormalization(m);
let mp = Prim.intMontgomerySetup(m);

// Round-trip: reduce(a * R mod m, m, mp) == a mod m
let a = 42;
let aR = (a * R) % m;
assert Prim.intMontgomeryReduce(aR, m, mp) == a % m;

// Montgomery multiplication: reduce(aR * bR, m, mp) == abR mod m
let b = 17;
let bR = (b * R) % m;
let abR_via_montgomery = Prim.intMontgomeryReduce(aR * bR, m, mp);
let abR_direct = (a * b * R) % m;
assert abR_via_montgomery == abR_direct;
// One more reduce undoes the trailing R factor: reduce(abR, m, mp) == ab mod m
assert Prim.intMontgomeryReduce(abR_via_montgomery, m, mp) == (a * b) % m;
