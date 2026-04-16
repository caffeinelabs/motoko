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
