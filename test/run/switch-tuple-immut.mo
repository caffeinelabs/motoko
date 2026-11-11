// A tuple-literal scrutinee whose components are pure reads (variables,
// constants) is matched against those reads directly; no `switch_in` local
// is introduced for them. Components that are not pure reads, or that are
// followed by a component that could write their source, still get one.

func pure(a : Nat, b : Nat) : Nat {
  switch (a, b) {
    case (0, y) { y }
    case (x, 0) { x }
    case _ { a + b }
  }
};

func mixed(a : Nat, b : Nat) : Nat {
  switch (a + 1, b) {
    case (1, y) { y }
    case (x, _) { x }
  }
};

// The read of `a` must happen before the second component assigns it
func write_after(n : Nat) : Nat {
  var a = n;
  switch (a, do { a := 5; 1 }) {
    case (0, _) { 100 }
    case (x, y) { x + y }
  }
};

// The read of `a` must happen after the first component assigns it
func write_before(n : Nat) : Nat {
  var a = n;
  switch (do { a := 5; 1 }, a) {
    case (_, 0) { 100 }
    case (x, y) { x + y }
  }
};

assert pure(0, 7) == 7;
assert pure(7, 0) == 7;
assert pure(3, 4) == 7;
assert mixed(0, 7) == 7;
assert mixed(3, 7) == 4;
assert write_after(0) == 100;
assert write_after(3) == 4;
assert write_before(0) == 6;
assert write_before(3) == 6;

// (functions appear in reverse order)
// CHECK-LABEL: (func $write_before
// CHECK: local.set $switch_in
// CHECK-NOT: local.set $switch_in
// CHECK-LABEL: (func $write_after
// CHECK: local.set $switch_in
// CHECK: local.set $switch_in.1
// CHECK-LABEL: (func $mixed
// CHECK: local.set $switch_in
// CHECK-NOT: local.set $switch_in
// CHECK-LABEL: (func $pure
// CHECK-NOT: switch_in
// CHECK: local.get $a
// CHECK-NOT: switch_in
// CHECK: local.get $b
// CHECK-NOT: switch_in
// CHECK-LABEL: (func $B_add
