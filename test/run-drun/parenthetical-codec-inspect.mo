import Prim "mo:⛔";

// `system func inspect` sees a method with a `decoder` through its
// `decoder`: the `msg` thunk yields the decoded arguments (not Candid).
// (drun stops at the first refused message, so that one comes last.)
actor {
  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  public (with decoder = text) func shout(t : Text) : async () {
    Prim.debugPrint(t # "!")
  };

  public func plain(n : Nat) : async () {
    Prim.debugPrint(debug_show(n))
  };

  system func inspect({ msg : { #shout : () -> Text; #plain : () -> Nat } }) : Bool {
    switch msg {
      case (#shout f) { f().size() <= 5 };
      case (#plain f) { f() < 100 }
    }
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress shout "hey"
//CALL ingress plain "DIDL\x00\x01\x7d\x2a"
//CALL ingress shout "fartoolong"
