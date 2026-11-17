// Tuple-shaped codecs: a single tuple-typed parameter (the decoder returns
// one tuple, written `((A, B))`), a tuple next to another parameter, and a
// nested tuple result.
import Prim "mo:⛔";
actor {
  // a single tuple-typed parameter
  public (with decoder = func (b : Blob) : ((Nat, Nat)) = ((b.size(), 2 * b.size()))) func tup(p : (Nat, Nat)) : async Nat { p.0 + p.1 };
  // a tuple parameter next to another
  public (with decoder = func (b : Blob) : ((Nat, Nat), Nat) = ((b.size(), 1), 10)) func mixed(p : (Nat, Nat), q : Nat) : async Nat { p.0 + p.1 + q };
  // a nested tuple result
  public (with encoder = func ((p, t) : ((Nat, Nat), Text)) : Blob = Prim.encodeUtf8(debug_show(p.0 + p.1) # t)) func nested() : async ((Nat, Nat), Text) { ((3, 4), "x") };

  // `inspect` sees the decoded tuples too
  system func inspect({ msg : { #tup : () -> ((Nat, Nat)); #mixed : () -> ((Nat, Nat), Nat); #nested : () -> () } }) : Bool {
    switch msg {
      case (#tup f) { let (a, b) = f(); a + b < 20 };
      case (#mixed f) { let ((a, _), q) = f(); a + q < 20 };
      case (#nested _) { true }
    }
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress tup "abc"
//CALL ingress mixed "ab"
//CALL ingress nested "DIDL\x00\x00"
//CALL ingress tup "abcdefg"
