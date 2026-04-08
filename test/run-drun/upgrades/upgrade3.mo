import { debugPrint; setTimer } "mo:⛔";
actor {
  debugPrint ("init'ed 3");
  stable let c : Text = do { assert false; loop {}};
  stable var i : Nat = do { assert false; loop {}};
  public func inc() : () {
      let i0 = i;
      ignore setTimer(0, false, func () : async () {i += 1});
      while (i0 == i) { await async () }
  };
  public query func check(n : Int) : async () {
    assert (c.size() == 3);
    assert (i == n);
  };
}
