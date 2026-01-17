import Prim "mo:prim";

import Migration "Migration6";

(
  with migration = Migration.run
)
actor {

  Prim.debugPrint("The correct version 2");

  stable var zero : Nat = 0;
  assert zero == 0;

  stable var three : [var (Nat, Text)] = [var];

  stable var four : Text = "";

  stable var five : Text = "";

  stable var six : Text = "";

  public func check() : async () {
    Prim.debugPrint(debug_show { zero; three; four; five; six });
  };
};
