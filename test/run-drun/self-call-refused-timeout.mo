//MOC-FLAG -A=M0194
import P "mo:⛔";

// A refused call to an internal method of this canister must not leave its
// `timeout` armed for the next call.
actor self {

  public func probe() : async Bool { P.replyDeadline() != 0 };

  public func go() : async () {
    let target = actor (debug_show (P.principalOfActor(self))) : actor {
      __motoko_async_helper : shared Nat32 -> async ()
    };
    try {
      ignore (with timeout = 5) target.__motoko_async_helper(0);
    } catch e {
      P.debugPrint(debug_show ("refused", P.errorMessage(e)));
    };
    P.debugPrint(debug_show ("best-effort", await probe()));
  };
};

//CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
