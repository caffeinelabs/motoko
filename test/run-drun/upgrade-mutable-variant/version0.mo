import Prim "mo:prim";

actor {
  type OriginalVariant = {
    #Option1;
    #Option2;
  };

  let instance = [var #Option1: OriginalVariant];
  let alias = instance;

  public func test() : async () {
    Prim.debugPrint("instance=" # debug_show (instance));
    Prim.debugPrint("alias=" # debug_show (alias));
  };
};
