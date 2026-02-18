//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration/migrations1

import Prim "mo:prim";

actor {
  // Changing type and mutability also works.
  let zero : [Nat];

  var three : [var (Nat, Text)];

  var four : Text;

  var five : Text;

  var six : [Text];

  public func check() : async () {
    assert zero[0] == 0; // Checks here if the Init.mo in migrations1 dir was skipped because of its name.
    Prim.debugPrint(debug_show { zero; three; four; five; six });
  };
};
