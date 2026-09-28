import Prim "mo:prim";

actor {
  var firstValue = 0;

  public func increase() : async () {
    firstValue += 0;
  };

  public func show() : async () {
    Prim.debugPrint("firstValue " # debug_show (firstValue));
  };
};
