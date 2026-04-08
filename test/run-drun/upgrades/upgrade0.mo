import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed 0");
  Prim.debugPrint ("root key: " # debug_show Prim.rootKey());
  Prim.debugPrint ("initial subnet: " # debug_show Prim.canisterSubnet());
  Prim.debugPrint ("initial version: " # debug_show Prim.canisterVersion());
  stable var c = "a";
  public func inc() : () { c #= "a"; };
  public query func check(n : Int) : async () {
    Prim.debugPrint(c);
    assert (c.size() == n);
  };
}
