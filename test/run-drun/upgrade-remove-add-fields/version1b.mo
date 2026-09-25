import Prim "mo:prim";

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
