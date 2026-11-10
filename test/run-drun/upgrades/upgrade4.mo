import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed 4");
  let c : Text = do { assert false; loop {}};
  var i : Nat = do { assert false; loop {}};
  public func inc() : () { i += 1; };
  public query func check(n : Int) : async () {
    assert (c.size() == 3);
    assert (i == n);
  };
  system func timer(set : Nat64 -> ()) : async () {
      Prim.debugPrint ("timer endpoint in 4");
      set 0
  };
}
