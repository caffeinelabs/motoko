//CLASSICAL-PERSISTENCE-ONLY
//MOC-FLAG -fshared-code
import Prim "mo:⛔";

// Differences between incremental and non-incremental compilation (additional forwarding header field).

// FHECK: (local $check0 i32)

// FHECK-NOT:  call $@immut_array_size
// DON'TFHECK: i32.load offset=(5 or 9) 
// FHECK:      i32.load offset= 
// FHECK:      i32.const 2
// FHECK:      i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      local.tee $check0
// FHECK-NEXT: call $print_text
// FHECK:      i32.const 4
// FHECK-NEXT: i32.add
for (check0 in ["hello", "world"].vals()) { Prim.debugPrint check0 };

// FHECK-NOT:  call $@immut_array_size
// DON'TFHECK: i32.load offset=(5 or 9) 
// FHECK:      i32.load offset= 
// FHECK:      i32.const 2
// FHECK:      i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      local.tee $check0
// FHECK-NEXT: call $print_text
// FHECK:      i32.const 4
// FHECK-NEXT: i32.add
for (check0 in ["hello", "world"].values()) { Prim.debugPrint check0 };


// FHECK-NOT:  call $@mut_array_size
// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK-NEXT: i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      i32.load offset=
// FHECK-NEXT: local.tee $check1
// FHECK-NEXT: call $print_text
for (check1 in [var "hello", "mutable", "world"].vals()) { Prim.debugPrint check1 };


// FHECK-NOT:  call $@mut_array_size
// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK-NEXT: i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      i32.load offset=
// FHECK-NEXT: local.tee $check1
// FHECK-NEXT: call $print_text
for (check1 in [var "hello", "mutable", "world"].values()) { Prim.debugPrint check1 };

let array = [var "hello", "unexpected", "world"];
array[1] := "remutable";
// FHECK-NOT:   call $@immut_array_size
// DON'TFHECK:  i32.load offset=(5 or 9)
// FHECK:       i32.load offset=
// FHECK:       i32.const 2
// FHECK:       i32.shl
// DON'T-FHECK: i32.lt_u
// DON'T-FHECK: local.get $array
// DON'T-FHECK: local.set $check2
// `arr` being a `VarE` already (but we rebind anyway, otherwise we open a can of worms)
// later when we have path compression for variables in the backend, we can bring this back
for (check2 in array.vals()) { Prim.debugPrint check2 };

let arrayValues = [var "hello", "unexpected", "world"];
arrayValues[1] := "remutable";
// FHECK-NOT:   call $@immut_array_size
// DON'TFHECK:  i32.load offset=(5 or 9)
// FHECK:       i32.load offset=
// FHECK:       i32.const 2
// FHECK:       i32.shl
// DON'T-FHECK: i32.lt_u
// DON'T-FHECK: local.get $array
// DON'T-FHECK: local.set $check2
// `arr` being a `VarE` already (but we rebind anyway, otherwise we open a can of worms)
// later when we have path compression for variables in the backend, we can bring this back
for (check2 in arrayValues.values()) { Prim.debugPrint check2 };

// FHECK-NOT:  call $@immut_array_size
// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK:      i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      i32.load offset=
// FHECK-NEXT: local.tee $check3
// interfering parentheses don't disturb us
for (check3 in (((["hello", "immutable", "world"].vals())))) { Prim.debugPrint check3 };

// FHECK-NOT:  call $@immut_array_size
// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK:      i32.shl
// FHECK:      i32.lt_u
// FHECK:      i32.add
// DON'TFHECK: i32.load offset=(9 or 13)
// FHECK:      i32.load offset=
// FHECK-NEXT: local.tee $check3
// interfering parentheses don't disturb us
for (check3 in (((["hello", "immutable", "world"].values())))) { Prim.debugPrint check3 };


// FHECK:      i32.const 84
// FHECK:      call $B_add
// FHECK-NEXT: call $B_eq
// FHECK-NEXT: if
// FHECK-NEXT: loop
// FHECK-NEXT: br 0
// FHECK-NEXT: end
// FHECK-NEXT: unreachable
// FHECK-NEXT: else
// bottom iteration expression is treated fairly
var c1 = 42;
if (c1 == c1 + 1) {
    for (check4 in (loop {}).vals()) { Prim.debugPrint check4 }
};

// FHECK:      i32.const 84
// FHECK:      call $B_add
// FHECK-NEXT: call $B_eq
// FHECK-NEXT: if
// FHECK-NEXT: loop
// FHECK-NEXT: br 0
// FHECK-NEXT: end
// FHECK-NEXT: unreachable
// FHECK-NEXT: else
// bottom iteration expression is treated fairly
var c2 = 42;
if (c2 == c2 + 1) {
    for (check4 in (loop {}).values()) { Prim.debugPrint check4 }
};

// FHECK:      call $B_add
// FHECK-NEXT: call $B_eq
// FHECK-NEXT: if
// FHECK-NEXT: loop
// FHECK-NEXT: br 0
// FHECK-NEXT: end
// FHECK-NEXT: unreachable
// FHECK-NEXT: else
// typed bottom iteration expression is treated fairly
if (c1 == c1 + 1) {
    for (check5 in ((loop {}) : [Text]).vals()) { Prim.debugPrint check5 }
};

// FHECK:      call $B_add
// FHECK-NEXT: call $B_eq
// FHECK-NEXT: if
// FHECK-NEXT: loop
// FHECK-NEXT: br 0
// FHECK-NEXT: end
// FHECK-NEXT: unreachable
// FHECK-NEXT: else
// typed bottom iteration expression is treated fairly
if (c2 == c2 + 1) {
    for (check5 in ((loop {}) : [Text]).values()) { Prim.debugPrint check5 }
};

let check6 = [var "hello", "immutable", "world"];
check6[1] := "mutable";
// `check6` being a `VarE` already and iteration variable is named identically
// this passes the IR type check, which demonstrates that no name capture happens
for (check6 in check6.vals()) { ignore check6 };

let check6Values = [var "hello", "immutable", "world"];
check6Values[1] := "mutable";
// `check6` being a `VarE` already and iteration variable is named identically
// this passes the IR type check, which demonstrates that no name capture happens
for (check6 in check6Values.values()) { ignore check6 };

// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK:      i32.shl
// argument to vals can have an effect too, expect it
for (check7 in [].vals(Prim.debugPrint "want to see you")) { };

// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 2
// FHECK:      i32.shl
// argument to vals can have an effect too, expect it
for (check7 in [].values(Prim.debugPrint "want to see you")) { };

// FHECK:      local.set $num8
// FHECK-NOT:  call $@immut_array_size
// DON'TFHECK: i32.load offset=(5 or 9)
// FHECK:      i32.load offset=
// FHECK:      i32.const 1
// FHECK:      i32.shl
// FHECK:      i32.lt_u
// FHECK-NOT:  i32.add
// FHECK:      local.tee $check8
// FHECK-NEXT: local.get $num8
// FHECK-NEXT: call $B_add
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
for (check10 in ([3, 5, 7, 11] : [Nat8]).vals()) { sum10 += check10 };
assert sum10 == 26;

// make sure that one-byte-sized elements still work
sum10 := 0;
for (check10 in ([3, 5, 7, 11] : [Nat8]).values()) { sum10 += check10 };
assert sum10 == 26;

// make sure that one-byte-sized elements still work
sum10 := 0;
for (check10 in ([3, 5, 7, 11] : [Nat8]).values()) { sum10 += check10 };
assert sum10 == 26
//MOC-FLAG -A=M0269
