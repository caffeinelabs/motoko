func foo(b : Bool, n : Int) : (Int, Int) {
  if (b) {
    (n,1)
  } else {
    (n,2)
  }
};
ignore(foo(true,5));

func pair(n : Nat64) : (Nat64, Nat64) = (n, n + 1);
let (a, b) = pair(42);
assert (a == 42 and b == 43);
