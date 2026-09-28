import Prim = "mo:⛔";
import A = "issue-4323/A";
import B = "issue-4323/B";
import C = "issue-4323/C";
// let-like annotation
actor This : actor { type U = Nat; beep : () -> async () } {

  public type T = Nat;
  public type U = T;

  // we see the actual, not constrained type (Nat not Int);
  ignore A.f() : Nat;
  ignore B.f() : Nat;
  ignore C.f() : Nat;

  // we see the actual, not specified members;
  A.g();
  B.g();
  C.g();

  public func beep() : async () {};

  public func ok() : async () { Prim.debugPrint "ok" }; // still exported, despite not in declared interface!

  public func check() : async ()  { await This.ok() }; // check no curtailment this

}
//CALL ingress ok "DIDL\x00\x00"
