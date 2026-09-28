import Prim "mo:prim";

// Compatible upgrade
actor {
  type ArrayStructure = ?[AliasName];
  type AliasName = ArrayStructure;

  var simpleArray: [var Float] = [var];
  var nestedArray: [[Nat]] = [];
  var recursiveArray = ?[] : ArrayStructure;

  public func print() : async () {
    Prim.debugPrint(debug_show (simpleArray));
    Prim.debugPrint(debug_show (recursiveArray));
  };
};
