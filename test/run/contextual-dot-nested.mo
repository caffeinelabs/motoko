import Vec "contextual-dot/Vec";
import Pair "contextual-dot/Pair";
import Facade "contextual-dot/Facade";
import Facade2 "contextual-dot/Facade2";

// Contextual dot and implicit arguments search the scope in the same tiers:
// a local value, then the direct fields of the modules in scope, then the
// fields of their nested modules. Importing a facade is enough to reach its
// modules, and a direct field always wins over one reached through a facade.

func tripled<T>(x : T, triple : (implicit : T -> T)) : T = triple(x);
func swapped(x : Pair.Pair, swap : (implicit : Pair.Pair -> (Text, Nat))) : (Text, Nat) = swap(x);
func total<T>(x : T, sum : (implicit : T -> Nat)) : Nat = sum(x);
func picked<T>(x : T, pick : (implicit : T -> Int)) : Int = pick(x);
func bump<T>(x : T, inc : (implicit : T -> T)) : T = inc(x);

// Reached only through the facade
assert (2 : Nat).triple() == 6; // Facade.Deep.Deeper.triple
assert tripled(2 : Nat) == 6;
assert Facade.nine() == 9;

// Pair.swap is also Facade.Pair.swap and Facade2.Pair.swap; the direct field wins
let p : Pair.Pair = (1, "one");
assert p.swap() == ("one", 1);
assert swapped(p) == ("one", 1);

// Likewise Vec.sum
let v : Vec.Vec = { x = 1; y = 2 };
assert v.sum() == 3;
assert total(v) == 3;
assert Facade2.Vec.sum(v) == 3;

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
  public func three() : Nat = bump(2 : Nat);
};

assert (1 : Nat).pick() == 1; // Direct.pick
assert picked(1 : Nat) == 1;
assert (1 : Nat).inc() == 2; // Outer.Inner.inc
assert bump(1 : Nat) == 2;
assert Outer.two() == 2;
assert Outer.three() == 3;
