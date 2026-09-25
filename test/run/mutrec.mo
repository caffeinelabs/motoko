func even(n : Nat) : Bool {
  if n == 0 {
    return true;
  } else {
    return odd(n - 1);
  }
};

func odd(n : Nat) : Bool {
  if n == 0 {
    return false;
  } else {
    return even(n - 1);
  }
};

assert(even(0));
assert(even(2));
assert(even(4));
assert(even(6));
assert(odd(5));
assert(not odd(6));
