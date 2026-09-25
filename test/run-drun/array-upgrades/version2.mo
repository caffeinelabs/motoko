import Prim "mo:prim";

// Incompatible upgrade
actor {
  type RecursiveArray = ?[RecursiveArray];

  var simpleArray: [Float] = [];
  var nestedArray: [Nat] = [];
  var recursiveArray = ?[] : RecursiveArray;

  public func print() : async () {
    Prim.debugPrint(debug_show (simpleArray));
    Prim.debugPrint(debug_show (nestedArray));
    Prim.debugPrint(debug_show (recursiveArray));
  };
};
