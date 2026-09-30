module Top {
 public module Nested {
   public let zero : Nat = 0;
   public let one : Nat = 1;
 };
 public let zero : Nat = 0;
 public let one : Nat = 1;
};

let one : Nat = 1;

func f(zero : (implicit : Nat)) : Nat {
  zero
};

func g(one : (implicit : Nat)) : Nat {
  one
};

ignore g(); // Fine, top-level candidates win over module candidates
ignore f(); // Error, candidates from nested modules conflict with candidates from top-level modules

func render(n : Nat, show : (implicit : Nat -> Text)) : Text { show(n) };

// Each call builds a different module from one definition, so they conflict
func mk(n : Nat) : module { show : Nat -> Text } {
  module { public func show(m : Nat) : Text { debug_show (n + m) } }
};

let A = mk(1);
let B = mk(2);
ignore render(5); // Error, `A.show` and `B.show` differ
