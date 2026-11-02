//MOC-FLAG -A=M0194
let x = 3 else { assert false; loop () };

let (y, z) = (4, 5) else { assert false; loop () };

// the second alternative is never matched
func oneOrZero(n : Nat) : Nat {
  let (1 or 1) = n else { return 0 };
  1
};
assert oneOrZero(2) == 0;
