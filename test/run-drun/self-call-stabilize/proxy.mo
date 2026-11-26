import P "mo:⛔";

// A canister that forwards arbitrary calls to itself.
// Created by the test actor, which is its only controller.
actor class Proxy() = self {

  // The deputy: forwards any method and raw argument to this canister.
  public func proxy(m : Text, arg : Blob) : async Blob {
    await P.call_raw(P.principalOfActor(self), m, arg)
  };

  public func ping() : async () { P.debugPrint("ping ok") };

  // These also accept self-calls, but only continue a (de)stabilization
  // that is already under way, so they must trap without effect.
  public func others() : async () {
    for (m in ["@motoko_async_stabilization", "@motoko_async_destabilization", "__motoko_destabilize_after_upgrade"].values()) {
      try {
        let _ = await proxy(m, to_candid (()));
        P.debugPrint(debug_show (m, "replied"));
      } catch e {
        P.debugPrint(debug_show (m, P.errorMessage e));
      };
    };
  };

  public func freeze() : async () {
    try {
      let _ = await proxy("__motoko_stabilize_before_upgrade", to_candid (()));
      P.debugPrint("stabilize replied");
    } catch e {
      P.debugPrint(debug_show ("stabilize", P.errorMessage e));
    };
  };
};
