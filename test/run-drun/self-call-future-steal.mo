//MOC-FLAG -A=M0194
import P "mo:⛔";

// A forwarded call to `__motoko_async_helper` that is queued before the
// genuine self-send of a future starts that future itself: the body runs in
// the forwarded message, its result goes to the forwarder, and the genuine
// awaiter is rejected although the body ran. Forwarding a call to an
// internal method of this canister must fail at the send.
actor self {

  var paid = 0;

  func pay() : async Nat {
    paid += 1;
    paid
  };

  // The deputy: calls a caller-supplied function, then pays out.
  public func execute(f : shared Nat32 -> async Nat, n : Nat32) : async () {
    var forwarded : ?(async Nat) = null;
    try {
      forwarded := ?f(n); // index 1 (`steal` holds 0), sent right away
    } catch e {
      P.debugPrint(debug_show ("forward refused", P.errorMessage(e)));
    };
    let payment = pay(); // index 2, its self-send is queued behind `forwarded`
    try {
      P.debugPrint(debug_show ("payment", await payment));
    } catch e {
      P.debugPrint(debug_show ("payment rejected", P.errorMessage(e)));
    };
    switch forwarded {
      case (?forwarded) {
        try {
          P.debugPrint(debug_show ("forwarded replied", await forwarded));
        } catch e {
          P.debugPrint(debug_show ("forwarded rejected", P.errorMessage(e)));
        };
      };
      case null {};
    };
    P.debugPrint(debug_show ("paid", paid));
  };

  public func steal() : async () {
    let target = actor (debug_show (P.principalOfActor(self))) : actor {
      __motoko_async_helper : shared Nat32 -> async Nat
    };
    await execute(target.__motoko_async_helper, 2);
  };
};

//CALL ingress steal "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
