import Prim "mo:prim";

// Read-only primitives need no `system` capability, so queries can call them
actor {
  public func u() : async () {
    Prim.debugPrint(debug_show Prim.callerInfoSigner<system>()); // redundant `<system>` warns
  };

  public query func q() : async () {
    Prim.debugPrint(debug_show Prim.getSelfPrincipal());
    Prim.debugPrint(debug_show Prim.envVarNames());
    Prim.debugPrint(debug_show Prim.envVar("key"));
    Prim.debugPrint(debug_show Prim.callerInfoSigner());
    Prim.debugPrint(debug_show Prim.callerInfoData());
    Prim.debugPrint(debug_show Prim.getCandidLimits());
    Prim.debugPrint(debug_show Prim.getCandidTypeLimits());
  };

  public composite query func cq() : async () {
    Prim.debugPrint(debug_show Prim.getSelfPrincipal());
    Prim.debugPrint(debug_show Prim.envVarNames());
    Prim.debugPrint(debug_show Prim.envVar("key"));
    Prim.debugPrint(debug_show Prim.callerInfoSigner());
    Prim.debugPrint(debug_show Prim.callerInfoData());
    Prim.debugPrint(debug_show Prim.getCandidLimits());
    Prim.debugPrint(debug_show Prim.getCandidTypeLimits());
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress u 0x4449444C0000
//CALL query q 0x4449444C0000
//CALL query cq 0x4449444C0000
