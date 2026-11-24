//MOC-FLAG -A=M0194
import P "mo:⛔";

// A canister that forwards arbitrary calls to itself can be used to call
// `__motoko_async_helper` with the index of an ordinary (non-future)
// continuation. Those are 3-element tuples; `peek_future_continuation` used to
// read element 3 regardless, calling the word past the end as a closure.
// It must trap instead.
actor self {

  // The deputy: forwards any method and raw argument to this canister.
  public func proxy(m : Text, arg : Blob) : async Blob {
    await P.call_raw(P.principalOfActor(self), m, arg)
  };

  public func ping() : async () { P.debugPrint("ping ok") };

  public func scan() : async () {
    var i : Nat32 = 0;
    while (i < 6) {
      try {
        let _ = await proxy("__motoko_async_helper", to_candid (i));
        P.debugPrint(debug_show ("index", i, "replied"));
      } catch e {
        P.debugPrint(debug_show ("index", i, P.errorMessage e));
      };
      i += 1;
    }
  };
};

//CALL ingress scan "DIDL\x00\x00"
//CALL ingress ping "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
