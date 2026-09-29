import Prim "mo:prim";

// State-changing primitives still require `system`, which no query provides
actor {
  public query func q() : async () {
    ignore Prim.cyclesAccept(1);
    Prim.cyclesAdd(1);
    ignore Prim.cyclesBurn(1);
    Prim.setCandidLimits({ numerator = 1; denominator = 1; bias = 0 });
    Prim.setCandidTypeLimits({ scalar = 1; bias = 0 });
  };

  public composite query func cq() : async () {
    ignore Prim.cyclesAccept<system>(1);
    Prim.cyclesAdd<system>(1);
    ignore Prim.cyclesBurn<system>(1);
    Prim.setCandidLimits<system>({ numerator = 1; denominator = 1; bias = 0 });
    Prim.setCandidTypeLimits<system>({ scalar = 1; bias = 0 });
  };
};
