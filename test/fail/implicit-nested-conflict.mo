module Top {
  public module A { public let zero : Nat = 0 };
  public module B { public let zero : Nat = 0 };
  public module Nested {
    public let one : Nat = 1;
    public let two : Nat = 2;
  };
  public let one : Nat = 1;
};

let two : Nat = 2;

func f(zero : (implicit : Nat)) : Nat = zero;
func g(one : (implicit : Nat)) : Nat = one;
func h(two : (implicit : Nat)) : Nat = two;

ignore h(); // Fine, the local value wins over module candidates
ignore g(); // Fine, the direct field Top.one wins over the nested Top.Nested.one
ignore f(); // Error, two candidates from nested modules conflict
