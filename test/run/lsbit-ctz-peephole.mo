//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY

// Regression test for the LSB-and-1 → ctz peephole in EOP.
// `if_both_tagged_scalar` emits `or; const 1; and; wrap_i64; if(slow,fast)`,
// which the peephole at `instrList.ml` rewrites to `or; ctz; wrap_i64; if(fast,slow)`.
// Before the fix the i64 case was suppressed because `i32.wrap_i64` (inserted by
// `E.if_` in compile_enhanced.ml) interposed between `and` and `if`, breaking the
// 3-instruction adjacency the original peephole required.

let x : Int = 1;
let y : Int = 2;
func eqInt() : Bool = x == y;
ignore eqInt();

//CHECK-LABEL: func $B_eq
//CHECK: i64.or
//CHECK-NEXT: i64.ctz
//CHECK-NEXT: i32.wrap_i64
//CHECK-NEXT: if

//SKIP run
//SKIP run-ir
//SKIP run-low
