//MOC-FLAG -A=M0194
import Prim "mo:⛔";
actor {
  Prim.debugPrint ("init'ed");

/*
  stable let always10 = 10;
  stable let () = ();
  stable let (fst,snd) = ("hello","world");
*/
  stable var c = 1;
  stable var b = true;
  public func inc() : () { c += 1; };
  public query func check(n : Int) : async () {
    assert (c == n);
//    assert (always10 == 10);
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
