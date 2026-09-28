import Prim "mo:prim";

(with migration =
  func (x: {
    var instance : {
      var firstField : Nat;
      var secondField : Nat;
      var thirdField : Nat
    }
  }) : {
    var instance : {
      var secondField : Nat;
    }
  } = { var instance = x.instance }
)
actor {
  var instance = {
    var secondField = 0;
  };

  public func increase() : async () {
    instance.secondField += 1;
  };

  public func show() : async () {
    Prim.debugPrint("secondField=" # debug_show (instance.secondField));
  };
};
