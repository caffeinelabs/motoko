import Prim "mo:⛔";

func hashInt(x : Int) : Nat32 {
  var n = x;
  var hash : Nat32 = 0;
  if n < 0 {
    hash := ^hash;
    n := Prim.abs n;
  };
  let base = 2**32;
  while n > 0 {
    hash ^= Prim.intToNat32Wrap(n % base);
    n /= base;
  };
  return hash;
};

assert (hashInt (10**7) == (10000000 : Nat32));
assert (hashInt 0 == (0 : Nat32));
assert (hashInt (10**18) == (2_860_824_243 : Nat32));

assert (hashInt (-1) == (4_294_967_294 : Nat32));
assert (hashInt (-387) == (4_294_966_908 : Nat32));
assert (hashInt (-3876548352991) == (1807116198 : Nat32));
