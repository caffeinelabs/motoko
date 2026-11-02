// CHECK-LABEL: (func $init

func matchNat(n : Nat) : Bool =
         switch n { case 1073741823 true
                  ; case _ false };
// CHECK-LABEL: (func $matchNat
// CHECK:        local.get $switch_in
// CHECK-NEXT:   i64.const 4294967294
// CHECK-NEXT:   call $B_eq

func matchInt(n : Int) : Bool =
         switch n { case (-1073741824) true
                  ; case 1073741823 true
                  ; case _ false };
// CHECK-LABEL: (func $matchInt
// CHECK:        local.get $switch_in
// CHECK-NEXT:   i64.const -4294967294
// CHECK-NEXT:   $B_eq
// CHECK:        local.get $switch_in
// CHECK-NEXT:   i64.const 4294967294
// CHECK-NEXT:   $B_eq

func match8(n : Nat8) : Bool = switch n { case 42 true; case _ false };
// CHECK-LABEL: (func $match8
// CHECK:        i64.const 3044433348102455296
// CHECK-NEXT:   i64.eq

func match16(n : Nat16) : Bool = switch n { case 42 true; case _ false };
// CHECK-LABEL: (func $match16
// CHECK:        i64.const 11892317766025216
// CHECK-NEXT:   i64.eq

// NB: reverse order, so that things appear in order
assert (match16(42));
assert (match8(42));
assert (matchInt(1073741823));
assert (matchInt(-1073741824));
assert (matchNat(1073741823));
