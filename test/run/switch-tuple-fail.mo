//MOC-FLAG -W=M0145
// A switch on an unboxed tuple scrutinee still traps when no case matches
func no_match(a : Nat, b : Nat) : Nat {
  switch (a, b) {
    case (0, x) { x }
    case (x, 0) { x }
  }
};
ignore no_match(1, 1)
