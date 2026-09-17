import Prim "mo:prim";

actor {
  let value: Null = null;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};
