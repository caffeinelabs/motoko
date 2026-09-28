import Prim "mo:⛔";

shared actor class One(a : Text) {
  Prim.debugPrint(a);

  public func test() : async () {
    Prim.debugPrint(debug_show a # " tested");
  };
}
