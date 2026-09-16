import { debugPrint } = "mo:⛔";

// v1: the SAME stable var, now typed `Principal`. The v0 `actor {}` value must
// migrate in — permitted because `actor {} <: Principal`. On EOP the heap value
// carries over untouched (stays `A`-tagged); on the classical backend the var is
// re-serialised through Candid, which canonicalises it to a `principal`.
actor {
  stable var p : Principal = (actor "aaaaa-aa" : actor {});

  public func show() : async () {
    debugPrint (debug_show p)
  }
}
