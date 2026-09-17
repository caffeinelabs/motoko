import Prim "mo:prim";

actor {

  type B<T> = [T];
  type A<T, U> = (B<T>, B<U>);
  module X = { public type A<U> = U };
  type T = (A<Nat, Int>, X.A<Bool>);

  var value = (([0], [1]), true) : T;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};
