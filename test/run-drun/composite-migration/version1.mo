import Prim "mo:prim";
import Func "Func";
import Migration1 "Migration1";
import Migration2 "Migration2";
import Migration3 "Migration3";
import Migration4 "Migration4";
import Migration5 "Migration5";
import Migration6 "Migration4";

(
  with migration =
    (Migration1.run).then
    (Migration2.run).then
    (Migration3.run).then
    (Migration4.run).then
    (Migration5.run)
)
actor {

  Prim.debugPrint("Version 1");

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
