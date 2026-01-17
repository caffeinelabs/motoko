//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors

import Prim "mo:prim";

import Migration "Migration6";

(
  with multi_migration = (
    Migration.run
  )
)
actor {
  var zero : Nat = 0;

  var three : [var (Nat, Text)] = [var];

  var four : Text = "";

  var five : Text = "";

  var six : Text = "";

  //six #= "hello";

  public func check() : async () {
    Prim.debugPrint(debug_show { zero; three; four; five; six });
  };
};
