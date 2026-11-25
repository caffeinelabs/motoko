//MOC-FLAG -W=M0210
import { debugPrint; replyDeadline } = "mo:⛔";

// A parenthetical on a call that does not send a message must not leave
// `timeout` (or `cycles`) behind for a later, unrelated send. Each `after`
// is a fresh message, so its call to `probe` must be guaranteed-response.

actor A {
  func inner() : async Nat { 42 };

  public func probe() : async Bool { replyDeadline() != 0 };

  public func before() : async () {
    debugPrint ("before: best-effort = " # debug_show (await probe()));
  };

  public func arm() : async () {
    // a plain function with a parenthesised future result: it cannot send
    func pass(f : async Nat) : (async Nat) = f;
    let fut = inner();
    ignore (with timeout = 1) pass(fut);
    debugPrint "armed";
  };

  public func arm_generic() : async () {
    // a generic function instantiated at a future type: it cannot send
    func pass<X>(x : X) : X = x;
    let fut = inner();
    ignore (with timeout = 1) pass<async Nat>(fut);
    debugPrint "armed (generic)";
  };

  public func after() : async () {
    debugPrint ("after: best-effort = " # debug_show (await probe()));
  };
};

//CALL ingress before "DIDL\x00\x00"
//CALL ingress arm "DIDL\x00\x00"
//CALL ingress after "DIDL\x00\x00"
//CALL ingress after "DIDL\x00\x00"
//CALL ingress arm_generic "DIDL\x00\x00"
//CALL ingress after "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
