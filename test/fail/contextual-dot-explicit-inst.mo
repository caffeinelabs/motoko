module Box {
  public type Box<T> = { var v : T };
  public func get<T>(self : Box<T>) : T { self.v };
  public func set<T>(self : Box<T>, x : T) { self.v := x };
};

module Rec {
  public type Rec<T> = { v : T };
  public func get<T>(self : Rec<T>) : T { self.v };
};

module Arr {
  public func store<A>(self : [var A], x : A) { self[0] := x };
};

let b : Box.Box<Text> = { var v = "hello" };
let _ : Nat = b.get<Nat>(); // reject
b.set<Nat>(42); // reject
let _ : Text = b.get<Text>(); // accept

let r : Rec.Rec<Nat> = { v = 1 };
let _ : Int = r.get<Int>(); // accept, `Rec<Nat> <: Rec<Int>`
let _ : Text = r.get<Text>(); // reject

let a : [var Nat] = [var 1];
a.store<Int>(-1); // reject, `[var Nat]` is not a `[var Int]`
a.store<Nat>(2); // accept
