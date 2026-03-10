import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed");
  stable var c = "a";
  public func inc() : () { c #= "a"; };
  public query func check(n : Int) : async () {
    Prim.debugPrint(c);
    assert (c.size() == n);
  };
}

//CALL query check "DIDL\x00\x01\x7d\x01"
//CALL ingress inc "DIDL\x00\x00"
//CALL query check "DIDL\x00\x01\x7d\x02"
//CALL ingress inc "DIDL\x00\x00"
//CALL query check "DIDL\x00\x01\x7d\x03"
//CALL upgrade
//CALL query check "DIDL\x00\x01\x7d\x03"
//CALL ingress inc "DIDL\x00\x00"
//CALL query check "DIDL\x00\x01\x7d\x04"

//SKIP run
//SKIP run-ir
//SKIP run-low
