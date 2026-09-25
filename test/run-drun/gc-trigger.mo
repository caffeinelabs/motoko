//MOC-NO-FORCE-GC
import Prim "mo:prim";

actor {
  let retained = Prim.Array_init<Nat>(6 * 1024 * 1024, 0);
  // GC is triggered during initialization

  let heapSizeWithoutGarbage = Prim.rts_heap_size();
  var heapSizeWithGarbage = heapSizeWithoutGarbage;

  public func createGarbage(): async() {
    ignore Prim.Array_init<Nat>(1024 * 1024, 0);
    ignore Prim.Array_init<Nat>(1024 * 1024, 0);
    heapSizeWithGarbage := Prim.rts_heap_size();
    // Growth is too little to trigger the incremental GC.
  };

  public query func checkBeforeGC(): async() {
    assert(Prim.rts_heap_size() >= heapSizeWithoutGarbage);
  };

  public query func checkAfterGC(): async() {
    assert(retained.size() > 0); // ensures that the array is not collected
    let tolerance = 100_000;
    assert(Prim.rts_heap_size() < heapSizeWithGarbage + tolerance);
  };
};
//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress createGarbage "DIDL\x00\x00"
//CALL query checkBeforeGC "DIDL\x00\x00"
//CALL ingress __motoko_gc_trigger "DIDL\x00\x00"
//CALL ingress __motoko_gc_trigger "DIDL\x00\x00"
//CALL ingress __motoko_gc_trigger "DIDL\x00\x00"
//CALL query checkAfterGC "DIDL\x00\x00"
