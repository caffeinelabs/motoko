// enable -E M0254 to reject non-empty initial actor (pre)
//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration-bad-chain/bad-chain -E M0254

import Prim "mo:prim";

actor {
  var zero : Nat;

  var three : [var (Nat, Text)];

  var four : Text;

  var five : Text;

  var six : Text;

  public func check() : async () {
    Prim.debugPrint(debug_show { zero; three; four; five; six });
  };
};
