//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// Candid record subtyping (`idl_sub`) must merge the field lists by tag:
// an optional field missing from the wire type may sort before the wire
// type's remaining fields, and differently named fields must never be
// compared with each other.
actor this {

  // expected result has an extra `opt` field sorting before `b`
  public func send_res(f : shared () -> async { a : ?Nat; b : Bool }) : async () {
    Prim.debugPrint("ok res")
  };

  // contravariant: expected argument lacks the `opt` field the wire one has
  public func send_arg(f : shared { b : Int } -> async ()) : async () {
    Prim.debugPrint("ok arg")
  };

  // must reject: wire `{a : Nat}` is not a subtype of `{b : Nat}`
  public func send_neg_0(f : shared () -> async { b : Nat }) : async () {
    Prim.debugPrint("wrong neg 0")
  };

  // must reject: required field `c` is missing on the wire
  public func send_neg_1(f : shared () -> async { a : ?Nat; c : Nat; d : Nat }) : async () {
    Prim.debugPrint("wrong neg 1")
  };

  public func res_b() : async { b : Bool } { { b = true } };
  public func arg_ab(_ : { a : ?Nat; b : Int }) : async () {};
  public func res_a() : async { a : Nat } { { a = 0 } };
  public func res_bd() : async { b : Nat; d : Nat } { { b = 0; d = 0 } };

  public func go() : async () {
    let t = debug_show (Prim.principalOfActor(this));

    do {
      let this = actor (t) : actor {
        send_res : (shared () -> async { b : Bool }) -> async ()
      };
      try { await this.send_res(res_b) } catch _ { Prim.debugPrint("wrong res") }
    };

    do {
      let this = actor (t) : actor {
        send_arg : (shared { a : ?Nat; b : Int } -> async ()) -> async ()
      };
      try { await this.send_arg(arg_ab) } catch _ { Prim.debugPrint("wrong arg") }
    };

    do {
      let this = actor (t) : actor {
        send_neg_0 : (shared () -> async { a : Nat }) -> async ()
      };
      try { await this.send_neg_0(res_a) } catch _ { Prim.debugPrint("ok neg 0") }
    };

    do {
      let this = actor (t) : actor {
        send_neg_1 : (shared () -> async { b : Nat; d : Nat }) -> async ()
      };
      try { await this.send_neg_1(res_bd) } catch _ { Prim.debugPrint("ok neg 1") }
    }
  }
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
