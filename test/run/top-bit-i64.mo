//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
import Prim "mo:⛔";

// i64 (EOP): MSB tests via `i64.shr_u 63` / `i64.shr_s 63` / `and (1 << 63)`.
// Boolean reaches the i32-typed `if` cond via either `i32.wrap_i64`
// (swap arms) or `i64.eqz` (inverts, no swap).
let n : Nat64 = 0xFFFF_FFFF_FFFF_FFFF;
assert (n >> 63 != 0);
assert (n & 0x8000_0000_0000_0000 != 0);

let i : Int64 = -1;
assert (i >> 63 != 0);
assert (i & -9223372036854775808 != 0);  // Int64.min_int form of 1 << 63

// Runtime sanity for let-else with MSB-clear `j`; the else trap is
// unreachable, the scrutinee path still goes through the MSB peephole.
let j : Int64 = 1;
do {
  let 0 = j >> 63 else { Prim.trap "msb set (i64 shr_s)" };
};
do {
  let 0 = j & -9223372036854775808 else { Prim.trap "msb set (i64 and)" };
};

// CHECK-LABEL: (func $init
// CHECK: i64.clz
// CHECK: i32.wrap_i64
// CHECK-NEXT: if  ;;
// CHECK: i64.clz
// CHECK: i32.wrap_i64
// CHECK-NEXT: if  ;;
// CHECK: end)
