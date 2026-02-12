//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration/migrations1

import Prim "mo:prim";

actor {
  // Changing mutability also works.
  let zero : Nat;

  var three : [var (Nat, Text)];

  var four : Text;

  var five : Text;

  var six : Text;

  public func check() : async () {
    Prim.debugPrint(debug_show { zero; three; four; five; six });
  };
};
