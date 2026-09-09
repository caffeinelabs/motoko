//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stabilization-instruction-limit=10000 -A=M0270

import Prim "mo:⛔";
import Cycles = "cycles/cycles";
import TestActor "incremental-actor-class-stabilization/test-actor";

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
    let testActor = await TestActor.TestActor(1234567890123456789012345678901234567890, "Test actor", Prim.Array_tabulate<Nat>(100_000, func(index) { index }));
    let testStabilization = useIncrementalStabilization(testActor);
    await testActor.print();
    await testStabilization.__motoko_stabilize_before_upgrade();
    try {
      await testActor.print();
      assert false;
    } catch (e) {
      Prim.debugPrint(Prim.errorMessage(e));
    };
    Prim.debugPrint("Upgrade");
    let upgraded = await (system TestActor.TestActor)(#upgrade testActor)(0, "", []);
    assert (testActor == upgraded);
    try {
      await testActor.print();
      assert false;
    } catch (e) {
      Prim.debugPrint(Prim.errorMessage(e));
    };
    await testStabilization.__motoko_destabilize_after_upgrade();
    await testActor.print();
  };
};

//CALL ingress run "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low
