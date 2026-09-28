import Prim "mo:prim";

actor class ReducedActor() {
  public func test1() : async () {
    Prim.debugPrint("reduced test1");
  };

  public func test3() : async () {
    Prim.debugPrint("reduced test3");
  };
};
