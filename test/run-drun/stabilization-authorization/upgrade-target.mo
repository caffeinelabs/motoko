import Prim "mo:⛔";

actor class UpgradeTarget() {
  var _stableArray = Prim.Array_tabulate<Nat>(100_000, func(index) { index });

  system func preupgrade() {
    Prim.debugPrint("PRE-UPGRADE HOOK!");
  };

  system func postupgrade() {
    Prim.debugPrint("POST-UPGRADE HOOK!");
  };
};
