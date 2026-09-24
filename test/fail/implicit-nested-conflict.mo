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
