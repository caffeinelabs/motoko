import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed 1");
  stable let c = "a";
  stable var i : Nat = c.size();
  var j = 0; // unstable cached state
  public func inc() : () { j += 1; };
  public query func check(n : Int) : async () {
    Prim.debugPrintNat(j);
    Prim.debugPrint(c);
    assert (j == n);
    assert (c.size() == 3);
    assert (c.size() <= j);
  };
  system func preupgrade(){
    Prim.debugPrint("preupgrade 1");
    i := j; // save cache
  };
  system func postupgrade(){
    j := i; // restore cache
    Prim.debugPrint("postupgrade 1");
  };

}

