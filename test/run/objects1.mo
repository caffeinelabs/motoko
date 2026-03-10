//MOC-FLAG -A=M0194
import Prim "mo:⛔";

let p = object {public let x = 3; let y = 2; public func get_y() : Int = y};
assert(p.x == 3);
assert(p.get_y() == 2);

func nonrec(a : {x : Nat}) = ignore ({a = {}; b = a.x});

let o : {a : {}; b : Nat} = object {public let a = {x = 0}; public let b = a.x};

let oo = object {
  type T = Int;
  public let x : T = 3;
  public let (y, z) = (3, "");
  public var v = 0;
  public func f() : T { g() + x };
  public func g() : T { f() + y };
  class C() {};
};

// pattern matching

func get_b () : Int = switch (o) {
  case {b = 11} 22;
  case {b = result} result;
  case {b = b : Int; a} b;
  case {a} 42
};

assert(get_b () == 0);

// subtyping in patterns

let q : {a : Int; b : Nat} = {a = -42; b = 25};

func get_a () : Int = switch (q) {
  case {a = 42; b} b;                  // OK: 42 is Int by subtyping
  case {b = 25 : Nat; a = a : Int} a   // OK: Int can consume all Nats
};

assert (get_a () == -42);

// subtyping and tuple patterns for comparison

let row : (Nat, Int, {c : Char; d : Text}) = (100, -42, {c = 'C'; d = "D"});

func foo () : Int = switch row {
  case (a : Int, -42, {c} : {c : Char}) (Prim.nat32ToNat(Prim.charToNat32 c))  // OK
};

assert (foo () == 67);

// matching on type-annotated fields

func baz () : Int = switch {a = 42} {
  case {a : Int} a  // OK
};

func full_name({first_name : Text; last_name : Text}) : Text =
    first_name # " " # last_name;
