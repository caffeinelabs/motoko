// Small-word `**` near the 2^64 boundary of the Nat64 intermediate: these must
// all stay representable, so the overflow guard must not fire on them. The
// trapping counterparts live in test/trap/pow{Nat,Int}{8,16,32}-wrap64.mo.

// widest operands, exponent 1
assert ((255 : Nat8) ** (1 : Nat8) == 255);
assert ((65535 : Nat16) ** (1 : Nat16) == 65535);
assert ((4294967295 : Nat32) ** (1 : Nat32) == 4294967295);
assert ((127 : Int8) ** (1 : Int8) == 127);
assert ((32767 : Int16) ** (1 : Int16) == 32767);
assert ((2147483647 : Int32) ** (1 : Int32) == 2147483647);

// largest exponents that still fit the target width
assert ((2 : Nat8) ** (7 : Nat8) == 128);
assert ((3 : Nat8) ** (5 : Nat8) == 243);
assert ((15 : Nat8) ** (2 : Nat8) == 225);
assert ((2 : Nat16) ** (15 : Nat16) == 32768);
assert ((3 : Nat16) ** (10 : Nat16) == 59049);
assert ((255 : Nat16) ** (2 : Nat16) == 65025);
assert ((2 : Nat32) ** (31 : Nat32) == 2147483648);
assert ((3 : Nat32) ** (20 : Nat32) == 3486784401);
assert ((65535 : Nat32) ** (2 : Nat32) == 4294836225);

assert ((2 : Int8) ** (6 : Int8) == 64);
assert ((-2 : Int8) ** (7 : Int8) == -128);
assert ((-5 : Int8) ** (3 : Int8) == -125);
assert ((2 : Int16) ** (14 : Int16) == 16384);
assert ((-2 : Int16) ** (15 : Int16) == -32768);
assert ((-3 : Int16) ** (9 : Int16) == -19683);
assert ((2 : Int32) ** (30 : Int32) == 1073741824);
assert ((-2 : Int32) ** (31 : Int32) == -2147483648);
assert ((3 : Int32) ** (19 : Int32) == 1162261467);
assert ((46340 : Int32) ** (2 : Int32) == 2147395600);

// degenerate bases and exponents short-circuit ahead of the guard
assert ((0 : Nat32) ** (40 : Nat32) == 0);
assert ((1 : Nat32) ** (99 : Nat32) == 1);
assert ((5 : Nat8) ** (0 : Nat8) == 1);
assert ((-1 : Int32) ** (99 : Int32) == -1);
assert ((-1 : Int32) ** (98 : Int32) == 1);
assert ((0 : Int8) ** (40 : Int8) == 0);
