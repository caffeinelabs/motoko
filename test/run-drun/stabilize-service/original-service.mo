import Prim "mo:prim";

actor class OriginalActor() {
  public func test1() : async () {
    Prim.debugPrint("original test1");
  };

  public func test2() : async () {
    Prim.debugPrint("original test2");
  };
};
