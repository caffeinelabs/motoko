let a : actor {f : () -> (); g : () -> (); type A = Int} = actor {
  public func f() : () {};
  public func g() : () {};
  public type A = Int
};

func foo() = switch a {
  case {f; g} { () }
};

assert (switch (foo()) { case () 0 }) == 0;

let { type A } = a; // okay to import types from actor
