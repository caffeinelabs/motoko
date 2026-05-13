//CLASSICAL-PERSISTENCE-ONLY
import Prim "mo:⛔";

// i32 Nat32: `shr_u 31` / `and 0x80000000` before `if`  →  `clz; if e t`.
let x : Nat32 = 4294967295;
assert (x >> 31 != 0);
assert (x & 2147483648 != 0);

// i32 Int32: same shape with `shr_s 31` (or-pattern with `shr_u`).
// `i & -2147483648` = `i & 0x80000000` (Int32.min_int form).
let i : Int32 = -1;
assert (i >> 31 != 0);
assert (i & -2147483648 != 0);

// br_if shape via let-else: pattern `0` matches when value is 0;
// the else branch fires when value != 0. moc emits
//   `value ; eqz ; br_if else_label`
// and the new rules collapse the eqz so the final shape is
//   `clz ; br_if else_label`. Use an MSB-clear value so the else
// branch is NOT taken at runtime.
let j : Int32 = 1;
do {
  let 0 = j >> 31 else { Prim.trap "msb set (shr_s)" };
};
do {
  let 0 = j & -2147483648 else { Prim.trap "msb set (and)" };
};

// CHECK-LABEL: (func $init
// CHECK: i32.clz
// CHECK-NEXT: if  ;;
// CHECK: i32.clz
// CHECK-NEXT: if  ;;
// CHECK: i32.clz
// CHECK-NEXT: if  ;;
// CHECK: i32.clz
// CHECK-NEXT: if  ;;
// CHECK: i32.clz
// CHECK-NEXT: br_if
// CHECK: i32.clz
// CHECK-NEXT: br_if
// CHECK: end)
