import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed 1");
  Prim.debugPrint ("current version: " # debug_show Prim.canisterVersion());
  stable let c = "a";
  stable var i : Nat = c.size();
  public func inc() : () { i += 1; };
  public query func check(n : Int) : async () {
    Prim.debugPrintNat(i);
    Prim.debugPrint(c);
    assert (i == n);
    assert (c.size() == 3);
    assert (c.size() <= i);
  };
}
