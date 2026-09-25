func foo() : (Int, Int) {
  return (1, 2)
};

func bar1(b : Bool) : Int {
  let x = if b { foo() } else { (3, 4) };
  return x.0;
};
func bar2(b : Bool) : Int {
  let x = if b { (3, 4) } else { foo() };
  return x.0;
};

assert(bar1(true) == 1);
assert(bar1(false) == 3);
assert(bar2(true) == 3);
assert(bar2(false) == 1);
