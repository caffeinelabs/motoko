//MOC-FLAG -A=M0194
let x = 3 else { assert false; loop () };

let (y, z) = (4, 5) else { assert false; loop () };

// a pattern no value of the type matches always takes the `else`
func negOrZero(n : Nat) : Int {
  let -1 = n else { return 0 };
  -1
};
assert negOrZero(1) == 0;
