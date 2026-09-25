func even(n : Nat) : Bool {
  if n == 0 {
    return true;
  } else
    { return odd(n - 1) };
};

func odd(n : Nat) : Bool {
  if n == 0 {
    return false;
  } else
    { return even(n - 1) };
};

// There should be a bunch of calls to known functions here, but
// no indirect calls
// CHECK: func $init
// CHECK: call $even
// CHECK: call $even
// CHECK: call $even
// CHECK: call $even
// CHECK: call $odd
// CHECK: call $odd

assert(even(0));
assert(even(2));
assert(even(4));
assert(even(6));
assert(odd(5));
assert(not odd(6));
