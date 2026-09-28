import Prim = "mo:⛔";
import Cycles = "cycles/cycles";
import Lib = "actor-class-cycles/C";

// test cycle transfer on class instantiation
actor a {

  func round(n : Nat) : Text {
    debug_show((n + 500_000_000_000) / 1_000_000_000_000) # "T";
  };

  public func go() : async () {
    Prim.debugPrint(debug_show({ Principal = Prim.principalOfActor a }));
    Prim.debugPrint(debug_show({ balance = round(Cycles.balance()) }));
    if Cycles.balance() == 0
      { await Cycles.provisional_top_up_actor(a, 100_000_000_000_000) };
    Prim.debugPrint(debug_show({ balance = round(Cycles.balance()) }));
    for i in [1, 2, 3].values() {
      Prim.debugPrint(debug_show({ iteration = i }));
      Prim.debugPrint(debug_show({ balance = round(Cycles.balance()) }));
      let c = await {
        if i == 1 {
          // test old-style
          Cycles.add<system>((i + 1) * 10_000_000_000_000);
          Lib.C();
        } else {
          (with cycles = (i + 1) * 10_000_000_000_000)
          Lib.C();
        }
      };
      let { current = cur; initial = init } = await c.balance();
      Prim.debugPrint(debug_show({ current = round(cur); initial = init }));
    }
  }
};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
