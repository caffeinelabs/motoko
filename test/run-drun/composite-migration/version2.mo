import Prim "mo:prim";
import Func "Func";

import Migration "Migration6";
import Migration2 "Migration5";

(
  with migration =
    ((Migration.run).then
     (Migration2.run))
)
actor {

  Prim.debugPrint("The wrong version 2");

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
