import Prim "mo:prim";

// Compatible change
actor {

  type B<T0> = [T0];
  type A<T1, T2> = (B<T1>, B<T2>);
  type T = (A<Nat, Int>, Bool);

  var value = (([0], [1]), true) : T;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};
