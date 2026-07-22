// desugar.ml `neutral`: identity operands (0 for add-like, 1 for mul-like) are
// elided at lowering for every fixed-width literal type (Nat8..Int64).

do { let x : Nat8 = 42; assert (x + (0 : Nat8) == x); assert ((0 : Nat8) + x == x) };
do { let x : Nat8 = 7; assert (x * (1 : Nat8) == x); assert ((1 : Nat8) * x == x) };
do { let x : Int8 = -13; assert (x + (0 : Int8) == x); assert ((0 : Int8) + x == x); assert (x - (0 : Int8) == x) };
do { let x : Int8 = 3; assert (x * (1 : Int8) == x); assert ((1 : Int8) * x == x) };
do { let x : Nat16 = 1000; assert (x + (0 : Nat16) == x); assert ((0 : Nat16) + x == x) };
do { let x : Nat16 = 9; assert (x * (1 : Nat16) == x); assert ((1 : Nat16) * x == x) };
do { let x : Int16 = -500; assert (x + (0 : Int16) == x); assert ((0 : Int16) + x == x); assert (x - (0 : Int16) == x) };
do { let x : Int16 = 11; assert (x * (1 : Int16) == x); assert ((1 : Int16) * x == x) };
do { let x : Nat32 = 100_000; assert (x + (0 : Nat32) == x); assert ((0 : Nat32) + x == x) };
do { let x : Nat32 = 31; assert (x * (1 : Nat32) == x); assert ((1 : Nat32) * x == x) };
do { let x : Int32 = -99_999; assert (x + (0 : Int32) == x); assert ((0 : Int32) + x == x); assert (x - (0 : Int32) == x) };
do { let x : Int32 = 17; assert (x * (1 : Int32) == x); assert ((1 : Int32) * x == x) };
do { let x : Nat64 = 1_000_000; assert (x + (0 : Nat64) == x); assert ((0 : Nat64) + x == x) };
do { let x : Nat64 = 2; assert (x * (1 : Nat64) == x); assert ((1 : Nat64) * x == x) };
do { let x : Int64 = -1_234_567; assert (x + (0 : Int64) == x); assert ((0 : Int64) + x == x); assert (x - (0 : Int64) == x) };
do { let x : Int64 = 13; assert (x * (1 : Int64) == x); assert ((1 : Int64) * x == x) };
