import Prim "mo:⛔";
import Cycles = "cycles/cycles";
import M0 "class-import/empty";
import M1 "class-import/one";
import M2 "class-import/two";
import M3 "class-import/trap";

actor a {
  public func go() : async () {
    // To get lots of cycles in drun
    if Cycles.balance() == 0 { await Cycles.provisional_top_up_actor(a, 100_000_000_000_000) };

    // test no arg class
    Cycles.add<system>(2_000_000_000_000);
    let empty : M0.Empty = await M0.Empty();
    await empty.test();

    // test single arg class
    Cycles.add<system>(2_000_000_000_000);
    let one : M1.One = await M1.One("one");
    await one.test();

    // test two arg class
    Cycles.add<system>(2_000_000_000_000);
    let two : M2.Two = await M2.Two("one", "two");
    await two.test();

    // test non-trapping install
    try {
      Cycles.add<system>(2_000_000_000_000);
      let _trap : M3.Trap = await M3.Trap(false);
    } catch _ {
      assert false;
    };

    // test trapping install
    try {
      Cycles.add<system>(2_000_000_000_000);
      let _trap : M3.Trap = await M3.Trap(true);
      assert false;
    } catch _ {
      Prim.debugPrint("caught trap");
    };

  };
};

a.go() //OR-CALL ingress go "DIDL\x00\x00"

//SKIP-SANITY-CHECKS
//SKIP run
//SKIP run-ir
//SKIP run-low

// check exactly 4 embedded wasms
//CHECK:  canister_update __motoko_async_helper
//CHECK:  canister_update __motoko_async_helper
//CHECK:  canister_update __motoko_async_helper
//CHECK:  canister_update __motoko_async_helper
//CHECK:  canister_update __motoko_async_helper
//CHECK-NOT:  canister_update __motoko_async_helper
