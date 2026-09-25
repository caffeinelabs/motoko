import Prim "mo:prim";

actor class OriginalActor() {
  public func test0() : async () {
    Prim.debugPrint("original test0");
  };

  public func test1() : async () {
    Prim.debugPrint("original test1");
  };

  public func test2() : async () {
    Prim.debugPrint("original test2");
  };

  public func test3() : async () {
    Prim.debugPrint("original test3");
  };
};
