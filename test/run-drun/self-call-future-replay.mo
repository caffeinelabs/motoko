//MOC-FLAG -A=M0194
import P "mo:⛔";

// A canister that forwards arbitrary calls to itself can be used to call
// `__motoko_async_helper` with the continuation index of a live future.
// The helper then runs that future's body a second time.
actor self {

  var bodyRuns = 0;

  // The deputy: forwards any method and raw argument to this canister.
  public func proxy(m : Text, arg : Blob) : async Blob {
    await P.call_raw(P.principalOfActor(self), m, arg)
  };

  public func idle() : async () {};

  public func replay() : async () {
    let f = async {
      bodyRuns += 1;
      P.debugPrint(debug_show ("future body run", bodyRuns));
      // Keep the self-call pending, so its table entry stays live.
      var k = 0;
      while (k < 20) { await idle(); k += 1 };
    };
    var i : Nat32 = 0;
    while (i < 6) {
      try {
        let _ = await proxy("__motoko_async_helper", to_candid (i));
        P.debugPrint(debug_show ("index", i, "replied"));
      } catch _ {
        P.debugPrint(debug_show ("index", i, "rejected"));
      };
      i += 1;
    };
    await f;
    P.debugPrint(debug_show ("body runs", bodyRuns));
  };
};

//CALL ingress replay "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
