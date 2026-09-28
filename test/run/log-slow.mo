// These are rather long-running tests, not advisable for wasm-run!

import Prim "mo:⛔";

class range(x : Nat, y : Nat) {
  var i = x;
  public func next() : ?Nat { if i > y { null } else { let j = i; i += 1; ?j } };
};

// Nat*

for n in range(0, 255) {
  for exp in range(0, 255) {
    if n <= 1 or exp <= 1 or (n <= 16 and exp <= 9) {
      let res = n ** exp;
      if res <= 255 { assert (Prim.natToNat8 n ** Prim.natToNat8 exp == Prim.natToNat8 res) }
    }
  }
};

assert (Prim.natToNat8 2 ** Prim.natToNat8 7 == Prim.natToNat8 128); // highest exponent

for n in range(0, 255) {
  for exp in range(0, 255) {
    if n <= 1 or exp <= 1 or (n <= 16 and exp <= 9) {
      // see #537
      let res = n ** exp;
      if res <= 255 { assert (Prim.natToNat8 n ** Prim.natToNat8 exp == Prim.natToNat8 res) }
    }
  }
};

assert (Prim.natToNat16 2 ** Prim.natToNat16 15 == Prim.natToNat16 32768); // highest exponent

for n in range(0, 255) {
  for exp in range(0, 255) {
    if n <= 1 or exp <= 106 {
      // see #537
      let res = n ** exp;
      if res <= 65535 { assert (Prim.natToNat16 n ** Prim.natToNat16 exp == Prim.natToNat16 res) }
    }
  }
};

assert (Prim.natToNat32 2 ** Prim.natToNat32 31 == Prim.natToNat32 2_147_483_648); // highest exponent

for n in range(0, 255) {
  for exp in range(0, 255) {
    if n <= 1 or exp <= 106 {
      // see #537
      let res = n ** exp;
      if res <= 65535 { assert (Prim.natToNat32 n ** Prim.natToNat32 exp == Prim.natToNat32 res) }
    }
  }
};

assert (Prim.natToNat64 2 ** Prim.natToNat64 63 == Prim.natToNat64 9223372036854775808); // highest exponent
assert (Prim.natToNat64 2642245 ** Prim.natToNat64 3 == Prim.natToNat64 18_446_724_184_312_856_125);

for n in range(0, 255) {
  for exp in range(0, 255) {
    if n <= 1 or exp <= 106 {
      // see #537
      let res = n ** exp;
      if res <= 18446744073709551615
      {
        assert (Prim.natToNat64 n ** Prim.natToNat64 exp == Prim.natToNat64 res)
      }
    }
  }
};

// Int*

assert (Prim.intToInt8 2 ** Prim.intToInt8 6 == Prim.intToInt8 64); // highest exponent

for n in range(0, 127) {
  for exp in range(0, 127) {
    if n <= 1 or exp <= 1 or (n <= 16 and exp <= 9) {
      let res = n ** exp;
      if res <= 127 { assert (Prim.intToInt8 n ** Prim.intToInt8 exp == Prim.intToInt8 res) }
    }
  }
};

assert (Prim.intToInt8 (-2) ** Prim.intToInt8 7 == Prim.intToInt8 (-128)); // highest exponent

do {
  var n = -128;
  while n < -1 {
    for exp in range(0, 127) {
      if n == -1 or exp <= 1 or (n >= -17 and exp <= 9) {
        let res = n ** exp;
        if res >= -128 and res <= 127 {
          assert (Prim.intToInt8 n ** Prim.intToInt8 exp == Prim.intToInt8 res)
        }
      }
    };
    n += 1
  }
};

assert (Prim.intToInt16 2 ** Prim.intToInt16 14 == Prim.intToInt16 16384); // highest exponent

for n in range(0, 127) {
  for exp in range(0, 127) {
    if n <= 1 or exp <= 1 or exp <= 14 {
      let res = n ** exp;
      if res <= 32767 { assert (Prim.intToInt16 n ** Prim.intToInt16 exp == Prim.intToInt16 res) }
    }
  }
};

assert (Prim.intToInt16 (-2) ** Prim.intToInt16 15 == Prim.intToInt16 (-32768)); // highest exponent

do {
  var n = -128;
  while n < -1 {
    for exp in range(0, 127) {
      if n == -1 or exp <= 1 or exp <= 15 {
        let res = n ** exp;
        if res >= -32768 and res <= 32767 {
          assert (Prim.intToInt16 n ** Prim.intToInt16 exp == Prim.intToInt16 res)
        }
      }
    };
    n += 1
  }
};

assert (Prim.intToInt32 3 ** Prim.intToInt32 19 == Prim.intToInt32 1_162_261_467);
assert (Prim.intToInt32 2 ** Prim.intToInt32 30 == Prim.intToInt32 1_073_741_824); // highest exponent
assert (Prim.intToInt32 (-2) ** Prim.intToInt32 31 == Prim.intToInt32 (-2_147_483_648)); // highest exponent
assert (Prim.intToInt32 (-3) ** Prim.intToInt32 19 == Prim.intToInt32 (-1_162_261_467));

assert (Prim.intToInt32 1 ** Prim.intToInt32 19 == Prim.intToInt32 1);
assert (Prim.intToInt32 1 ** Prim.intToInt32 100 == Prim.intToInt32 1);
assert (Prim.intToInt32 1 ** Prim.intToInt32 101 == Prim.intToInt32 1);

assert (Prim.intToInt32 0 ** Prim.intToInt32 19 == Prim.intToInt32 0);
assert (Prim.intToInt32 0 ** Prim.intToInt32 100 == Prim.intToInt32 0);
assert (Prim.intToInt32 0 ** Prim.intToInt32 101 == Prim.intToInt32 0);

assert (Prim.intToInt32 (-1) ** Prim.intToInt32 19 == Prim.intToInt32 (-1));
assert (Prim.intToInt32 (-1) ** Prim.intToInt32 100 == Prim.intToInt32 1);
assert (Prim.intToInt32 (-1) ** Prim.intToInt32 101 == Prim.intToInt32 (-1));

for n in range(0, 127) {
  for exp in range(0, 127) {
    if n <= 1 or exp <= 1 or exp <= 30 {
      let res = n ** exp;
      if res <= 2_147_483_647 { assert (Prim.intToInt32 n ** Prim.intToInt32 exp == Prim.intToInt32 res) }
    }
  }
};

do {
  var n = -128;
  while n < -1 {
    for exp in range(0, 127) {
      if n == -1 or exp <= 1 or exp <= 31 {
        let res = n ** exp;
        if res >= -2_147_483_648 and res <= 2_147_483_647 {
          assert (Prim.intToInt32 n ** Prim.intToInt32 exp == Prim.intToInt32 res)
        }
      }
    };
    n += 1
  }
};

assert (Prim.intToInt64 3 ** Prim.intToInt64 31 == Prim.intToInt64 617_673_396_283_947); // still on fast path

assert (Prim.intToInt64 3 ** Prim.intToInt64 39 == Prim.intToInt64 4_052_555_153_018_976_267);
assert (Prim.intToInt64 2 ** Prim.intToInt64 62 == Prim.intToInt64 4_611_686_018_427_387_904); // highest exponent
assert (Prim.intToInt64 (-2) ** Prim.intToInt64 63 == Prim.intToInt64 (-9_223_372_036_854_775_808)); // highest exponent
assert (Prim.intToInt64 (-3) ** Prim.intToInt64 39 == Prim.intToInt64 (-4_052_555_153_018_976_267));

assert (Prim.intToInt64 (-3) ** Prim.intToInt64 31 == Prim.intToInt64 (-617_673_396_283_947)); // still on fast path

assert (Prim.intToInt64 1 ** Prim.intToInt64 39 == Prim.intToInt64 1);
assert (Prim.intToInt64 1 ** Prim.intToInt64 100 == Prim.intToInt64 1);
assert (Prim.intToInt64 1 ** Prim.intToInt64 101 == Prim.intToInt64 1);

assert (Prim.intToInt64 0 ** Prim.intToInt64 39 == Prim.intToInt64 0);
assert (Prim.intToInt64 0 ** Prim.intToInt64 100 == Prim.intToInt64 0);
assert (Prim.intToInt64 0 ** Prim.intToInt64 101 == Prim.intToInt64 0);

assert (Prim.intToInt64 (-1) ** Prim.intToInt64 39 == Prim.intToInt64 (-1));
assert (Prim.intToInt64 (-1) ** Prim.intToInt64 100 == Prim.intToInt64 1);
assert (Prim.intToInt64 (-1) ** Prim.intToInt64 101 == Prim.intToInt64 (-1));

for n in range(0, 127) {
  for exp in range(0, 127) {
    if n <= 1 or exp <= 1 or exp <= 63 {
      let res = n ** exp;
      if res <= 9_223_372_036_854_775_807 { assert (Prim.intToInt64 n ** Prim.intToInt64 exp == Prim.intToInt64 res) }
    }
  }
};

do {
  var n = -128;
  while n < -1 {
    for exp in range(0, 127) {
      if n == -1 or exp <= 1 or exp <= 63 {
        let res = n ** exp;
        if res >= -9_223_372_036_854_775_808 and res <= 9_223_372_036_854_775_807 {
          assert (Prim.intToInt64 n ** Prim.intToInt64 exp == Prim.intToInt64 res)
        }
      }
    };
    n += 1
  }
};
