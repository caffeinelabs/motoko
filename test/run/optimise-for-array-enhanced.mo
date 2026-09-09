//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG -fshared-code
import Prim "mo:⛔";

// CHECK: (local $check0 i64)

// CHECK-NOT:  call $@immut_array_size
// DON'TCHECK: i64.load offset=17
// CHECK:      i64.load offset= 
// CHECK:      i64.const 2
// CHECK:      i64.shr_s
// CHECK-NEXT: i64.const 3
// CHECK-NEXT: i64.shl
// CHECK-NEXT: i64.add
// CHECK:      local.tee $check0
// CHECK:      i64.const 4
// CHECK:      i64.add
for (check0 in ["hello", "world"].values()) { Prim.debugPrint check0 };

// CHECK-NOT:  call $@mut_array_size
// DON'TCHECK: i64.load offset=17
// FIX-CHECK:      i64.const 2
// FIX-CHECK:      i64.shr_s
// FIX-CHECK:      i64.const 3
// FIX-CHECK:      i64.shl
// FIX-CHECK:      i64.add
// DON'TCHECK: i64.load offset=25
// CHECK:      i64.load offset=
// CHECK:      local.tee $check1Values
// CHECK:      call $print_ptr
// CHECK:      i64.const 4
// CHECK:      i64.add
for (check1Values in [var "hello", "mutable", "world"].values()) { Prim.debugPrint check1Values };

let array = [var "hello", "mutable", "world"];
array[1] := "remutable";
// FIX-CHECK-NOT:   call $@immut_array_size
// DON'TCHECK:  i64.load offset=17
// FIX-CHECK:       i64.load offset=
// FIX-CHECK:       i64.const 2
// FIX-CHECK:       i64.shr_s
// DON'T-CHECK: i64.lt_u
// DON'T-CHECK: local.get $array
// DON'T-CHECK: local.set $check2
// `arr` being a `VarE` already (but we rebind anyway, otherwise we open a can of worms)
// later when we have path compression for variables in the backend, we can bring this back
for (check2 in array.values()) { Prim.debugPrint check2 };

// FIX-CHECK-NOT:  call $@immut_array_size
// DON'TCHECK: i64.load offset=17
// FIX-CHECK:      i64.load offset=
// FIX-CHECK:      i64.const 2
// FIX-CHECK-NEXT: i64.shr_s
// FIX-CHECK:      i64.lt_u
// FIX-CHECK:      i64.add
// DON'TCHECK: i64.load offset=25
// FIX-CHECK:      local.tee $check3
// interfering parentheses don't disturb us
for (check3 in (((["hello", "immutable", "world"].values())))) { Prim.debugPrint check3 };


// FIX-CHECK:      i64.const 170
// FIX-CHECK:      call $B_add
// FIX-CHECK-NEXT: call $B_eq
// FIX-CHECK-NEXT: i32.wrap_i64
// FIX-CHECK-NEXT: if
// FIX-CHECK-NEXT: loop
// FIX-CHECK-NEXT: br 0
// FIX-CHECK-NEXT: end
// FIX-CHECK-NEXT: unreachable
// FIX-CHECK-NEXT: else
// bottom iteration expression is treated fairly
var c = 42;
if (c == c + 1) {
    for (check4 in (loop {}).values()) { Prim.debugPrint check4 }
};

// FIX-CHECK:      i64.const 170
// FIX-CHECK:      call $B_add
// FIX-CHECK-NEXT: call $B_eq
// FIX-CHECK-NEXT: i32.wrap_i64
// FIX-CHECK-NEXT: if
// FIX-CHECK-NEXT: loop
// FIX-CHECK-NEXT: br 0
// FIX-CHECK-NEXT: end
// FIX-CHECK-NEXT: unreachable
// FIX-CHECK-NEXT: else
// bottom iteration expression is treated fairly
c := 42;
if (c == c + 1) {
    for (check4 in (loop {}).values()) { Prim.debugPrint check4 }
};

// FIX-CHECK:      call $B_add
// FIX-CHECK-NEXT: call $B_eq
// FIX-CHECK-NEXT: i32.wrap_i64
// FIX-CHECK-NEXT: if
// FIX-CHECK-NEXT: loop
// FIX-CHECK-NEXT: br 0
// FIX-CHECK-NEXT: end
// FIX-CHECK-NEXT: unreachable
// FIX-CHECK-NEXT: else
// typed bottom iteration expression is treated fairly
if (c == c + 1) {
    for (check5 in ((loop {}) : [Text]).values()) { Prim.debugPrint check5 }
};

// FIX-CHECK:      call $B_add
// FIX-CHECK-NEXT: call $B_eq
// FIX-CHECK-NEXT: i32.wrap_i64
// FIX-CHECK-NEXT: if
// FIX-CHECK-NEXT: loop
// FIX-CHECK-NEXT: br 0
// FIX-CHECK-NEXT: end
// FIX-CHECK-NEXT: unreachable
// FIX-CHECK-NEXT: else
// typed bottom iteration expression is treated fairly
if (c == c + 1) {
    for (check5 in ((loop {}) : [Text]).values()) { Prim.debugPrint check5 }
};

let check6 = [var "hello", "immutable", "world"];
check6[1] := "mutable";
// `check6` being a `VarE` already and iteration variable is named identically
// this passes the IR type check, which demonstrates that no name capture happens
for (check6 in check6.values()) { ignore check6 };

// DON'TCHECK: i64.load offset=17
// FIX-CHECK:      i64.load offset=
// FIX-CHECK:      i64.const 3
// FIX-CHECK:      i64.shl
// argument to vals can have an effect too, expect it
for (check7 in [].values(Prim.debugPrint "want to see you")) { };

// FIX-CHECK:      local.set $num8
// FIX-CHECK-NOT:  call $@immut_array_size
// CON'TFHECK: i64.load offset=17
// FIX-CHECK:      i64.load offset=
// FIX-CHECK:      i64.const 2
// FIX-CHECK:      i64.shr_s
// FIX-CHECK:      i64.lt_u
// FIX-CHECK-NOT:  i64.add
// FIX-CHECK:      local.tee $check8
// FIX-CHECK-NEXT: local.get $num8
// FIX-CHECK-NEXT: call $B_add
var num8 = 42;
num8 := 25;
// `keys` is even easier to rewrite, as the "indexing expression" is just the
// indexing variable itself
for (check8 in ["hello", "keyed", "world"].keys()) { ignore (check8 + num8) };

// polymorphic arrays should still work
func _f9<A>(array : [A]) {
  for (check9 in array.keys()) { }
};

// make sure that one-byte-sized elements still work
var sum10 : Nat8 = 0;
for (check10 in ([3, 5, 7, 11] : [Nat8]).values()) { sum10 += check10 };
assert sum10 == 26;
