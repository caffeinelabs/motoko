//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stabilization-instruction-limit=10000 -A=M0270

import Prim "mo:⛔";
import Cycles = "cycles/cycles";
import UpgradeTarget "stabilization-authorization/upgrade-target";
import AccessTester "stabilization-authorization/access-tester";

actor a {
  type IncrementalStabilization = actor {
    __motoko_stabilize_before_upgrade : () -> async ();
    __motoko_destabilize_after_upgrade : () -> async ();
  };

  func useIncrementalStabilization(a : actor {}) : IncrementalStabilization {
    actor (debug_show (Prim.principalOfActor(a))) : IncrementalStabilization;
  };

  public func run() : async () {
    if (Cycles.balance() == 0) {
      await Cycles.provisional_top_up_actor(a, 100_000_000_000_000);
    };

    Cycles.add<system>(2_000_000_000_000);
    let upgradeTarget = await UpgradeTarget.UpgradeTarget();
    let testStabilization = useIncrementalStabilization(upgradeTarget);

    Cycles.add<system>(2_000_000_000_000);
    let accessTester = await AccessTester.AccessTester(testStabilization);
    await accessTester.test();

    Prim.debugPrint("Test upgrade");
    await testStabilization.__motoko_stabilize_before_upgrade();
    ignore await (system UpgradeTarget.UpgradeTarget)(#upgrade upgradeTarget)();

    await accessTester.test();
  };
};

//SKIP-SANITY-CHECKS
//CALL ingress run "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low
