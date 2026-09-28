import Prim "mo:prim";

actor {
  var instance = {
    var firstField = 0;
    var secondField = 0;
    var thirdField = 0;
  };

  public func increase() : async () {
    instance.firstField += 1;
    instance.secondField += 1;
    instance.thirdField += 1;
  };

  public func show() : async () {
    Prim.debugPrint("firstField=" # debug_show (instance.firstField));
    Prim.debugPrint("secondField=" # debug_show (instance.secondField));
    Prim.debugPrint("thirdField=" # debug_show (instance.thirdField));
  };
};
