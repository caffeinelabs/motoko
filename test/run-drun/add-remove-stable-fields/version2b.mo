import Prim "mo:prim";

// need explicit migration to drop field
(with migration = func ({ firstValue : Nat }) : {} = {})
actor {
  var secondValue = 0;

  public func increase() : async () {
    secondValue += 1;
  };

  public func show() : async () {
    Prim.debugPrint("secondValue=" # debug_show (secondValue));
  };
};
