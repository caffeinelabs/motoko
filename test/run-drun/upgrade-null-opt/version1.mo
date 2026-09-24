import Prim "mo:prim";

actor {
  let value: ?{} = ?{};

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};
