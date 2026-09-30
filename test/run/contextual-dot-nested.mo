import Vec "contextual-dot/Vec";
import _Vec2 "contextual-dot/Vec";
import Facade "contextual-dot/Facade";
import Facade2 "contextual-dot/Facade2";

// Contextual dot also finds functions in the nested modules of the modules in
// scope, so importing a facade is enough. The same function reached through
// several paths, like Facade.Pair.swap and Facade2.Pair.swap, is no ambiguity

let p : Facade.Pair.Pair = (1, "one");
assert p.swap() == ("one", 1); // Facade.Pair.swap
assert (2 : Nat).triple() == 6; // Facade.Deep.Deeper.triple
assert Facade.nine() == 9;

// Vec.sum is also _Vec2.sum, and Facade.Vec.sum; the direct field wins
let v : Vec.Vec = { x = 1; y = 2 };
assert v.sum() == 3;

// A direct field wins over a nested one, even when the nested one is closer
module Direct {
  public func pick(self : Int) : Int = self;
};

module Outer {
  public module Inner {
    public func pick(self : Nat) : Int = self + 100;
    public func inc(self : Nat) : Nat = self + 1;
  };

  // Inside Outer, Inner is in scope by itself
  public func two() : Nat = (1 : Nat).inc();
};

assert (1 : Nat).pick() == 1;
assert (1 : Nat).inc() == 2;
assert Outer.two() == 2;

assert Facade2.Vec.sum(v) == 3;
