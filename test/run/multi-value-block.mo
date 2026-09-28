import Prim "mo:⛔";

func returns_tuple(n : Nat) : (Nat, Nat) {
  return (n, n + 1);
};

func multi_value_if(n : Nat) : (Nat, Nat) {
  if n == 0 {
    returns_tuple(0)
  } else {
    if n == 1 {
      returns_tuple(1)
    } else {
      (n, n + 1);
    }
  }
};

func multi_value_if_mixed(n : Nat) : (Nat, Nat) {
  if n == 0 {
    returns_tuple(0)
  } else {
    if n == 1 {
      let first_class = (n, n + 1);
      first_class;
    } else {
      (n, n + 1);
    }
  }
};

func multi_value_switch(n : Nat) : (Nat, Nat) {
  switch n {
    case 0 { returns_tuple(n) }
    case 1 { returns_tuple(n) }
    case _ { (n, n + 1) }
  };
};

func multi_value_switch_mixed(n : Nat) : (Nat, Nat) {
  switch n {
    case 0 { returns_tuple(n) }
    case 1 { let first_class = (n, n + 1); first_class; }
    case _ { (n, n + 1) }
  };
};

func test(fun : Nat -> (Nat, Nat), arg : Nat) : Bool {
  let a0 = Prim.rts_total_allocation();
  let (x, y) = fun(arg);
  let a1 = Prim.rts_total_allocation();
  assert(x == arg and y == arg + 1);
  if a1 != a0 {
    Prim.debugPrint("Unexpected allocation: " # debug_show (a1 - a0 : Nat));
    return false;
  };
  return true;
};

// assert outside of test to get good location info
assert(test(returns_tuple, 0));
assert(test(returns_tuple, 1));
assert(test(returns_tuple, 2));
assert(test(multi_value_if, 0));
assert(test(multi_value_if, 1));
assert(test(multi_value_if, 2));
assert(test(multi_value_if_mixed, 0));
// assert(test(multi_value_if_mixed, 1));  // We expect this to allocate
assert(test(multi_value_if_mixed, 2));
assert(test(multi_value_switch, 0));
assert(test(multi_value_switch, 1));
assert(test(multi_value_switch, 2));
assert(test(multi_value_switch_mixed, 0));
// assert(test(multi_value_switch_mixed, 1));  // We expect this to allocate
assert(test(multi_value_switch_mixed, 2));
