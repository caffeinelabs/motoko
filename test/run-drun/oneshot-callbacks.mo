//MOC-NO-FORCE-GC
import Prim "mo:⛔";

actor a {

  public func drill(rabbit_hole : Nat8) : async () {
    Prim.debugPrint("drill! " # debug_show rabbit_hole # " " # debug_show Prim.rts_callback_table_count());
    try {
      await async {
        if rabbit_hole == 0 {
          assert false
        } else {
          await drill(rabbit_hole - 1);
          // force the reject continuation one third of the time
          assert rabbit_hole % 3 == 0
        }
      }
    } catch _ {
      // force the cleanup callback half of the time
      assert rabbit_hole % 2 == 0
    }
  };

  public shared func oneway_ping() : () {
    Prim.debugPrint("ping! " # debug_show Prim.rts_callback_table_count());
  };

  public func ping() : async () {
    Prim.debugPrint("ping-async! " # debug_show Prim.rts_callback_table_count());
  };

  public func go(trigger_cleanup : Bool) : async () {
    Prim.debugPrint("go 0: " # debug_show Prim.rts_callback_table_count());
    oneway_ping();
    await async {
      Prim.debugPrint("go 1: " # debug_show Prim.rts_callback_table_count())
    };
    await ping();
    try {
      ignore await async {
        Prim.debugPrint("go 2: " # debug_show Prim.rts_callback_table_count());
        assert false;
        42
      }
    } catch _ { if trigger_cleanup { ignore 42 / 0 } }
  };
};

await a.drill(42); //OR-CALL ingress drill "DIDL\x00\x01\x7B\x2A"
await a.go(true); //OR-CALL ingress go "DIDL\x00\x01\x7E\x01"
await a.go(false); //OR-CALL ingress go "DIDL\x00\x01\x7E\x00"
await a.go(true); //OR-CALL ingress go "DIDL\x00\x01\x7E\x01"

//SKIP run
//SKIP run-low
//SKIP run-ir
