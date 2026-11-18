import Prim "mo:⛔";

// `system func inspect` sees a method with a `decoder` through its
// `decoder`: the `msg` thunk yields the decoded arguments (not Candid).
// For a query, this applies to replicated (ingress) calls only; a plain
// query call doesn't go through `inspect`.
// (drun stops at the first refused message, so that one comes last.)
actor {
  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  (with decoder = text)
  public func shout(t : Text) : async () {
    Prim.debugPrint(t # "!")
  };

  (with decoder = text)
  public query func look(t : Text) : async Nat { t.size() };

  public func plain(n : Nat) : async () {
    Prim.debugPrint(debug_show(n))
  };

  system func inspect({ msg : { #shout : () -> Text; #look : () -> Text; #plain : () -> Nat } }) : Bool {
    switch msg {
      case (#shout f) { f().size() <= 5 };
      case (#look f) { f().size() <= 5 };
      case (#plain f) { f() < 100 }
    }
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress shout "hey"
//CALL query look "fartoolong"
//CALL ingress look "hey"
//CALL ingress plain "DIDL\x00\x01\x7d\x2a"
//CALL ingress shout "fartoolong"
