import Vec "contextual-dot/Vec";
import _Vec2 "contextual-dot/Vec";
import Facade "contextual-dot/Facade";
import Facade2 "contextual-dot/Facade2";

// One library imported twice, or reached through two facades and not
// imported itself, is ambiguous for both features, since the search does not
// tell one function reached through several paths apart. Importing the
// library directly resolves it, as in test/run/contextual-dot-nested.mo
let v : Vec.Vec = { x = 1; y = 2 };
ignore v.sum();
func total<T>(x : T, sum : (implicit : T -> Nat)) : Nat = sum(x);
ignore total(v);

let p : Facade.Pair.Pair = (1, "one");
ignore p.swap();
func swapped(x : Facade.Pair.Pair, swap : (implicit : Facade.Pair.Pair -> (Text, Nat))) : (Text, Nat) = swap(x);
ignore swapped(p);
assert Facade2.Vec.sum(v) == 3;

// Two different functions in nested modules are ambiguous
module Top {
  public module A { public func twice(self : Nat) : Nat = self * 2 };
  public module B { public func twice(self : Nat) : Nat = self + self };
};

ignore (1 : Nat).twice();

// The search for nested modules has a depth limit
module L1 { public module L2 { public module L3 { public module L4 {
  public module L5 { public module L6 { public module L7 { public module L8 {
    public func deepest(self : Nat) : Nat = self;
    public module L9 {
      public func tooDeep(self : Nat) : Nat = self;
    };
  } } } }
} } } };

ignore (1 : Nat).deepest(); // Resolves fine
ignore (1 : Nat).tooDeep();

// Instances of one module declaration are different modules
func adder(n : Nat) : module { plus : (self : Nat) -> Nat } {
  module { public func plus(self : Nat) : Nat = self + n }
};
let One = adder(1);
let Two = adder(2);

ignore (1 : Nat).plus();

