import Prim "mo:⛔";

actor a {
  type List = ?([var Int], List);
  var list : List = null;

  public func grow(n : Nat) : async Nat {
    list := ?(Prim.Array_init(n * 1024 * 256, 0), list);
    return  Prim.rts_memory_size();
  };

  // Disabling for Ubuntu tests
  public func go() : async () {

    var i = 1;
    var pre = Prim.rts_memory_size();
    var preMutatorInstructions = 0;
    var preCollectorInstructions = 0;
    while i < 4 {
      assert(Prim.rts_memory_size() >=
        Prim.rts_heap_size());
      let size = await grow(i);
      let mutatorInstructions = Prim.rts_mutator_instructions();
      let collectorInstructions = Prim.rts_collector_instructions();
      /* too noisy for comparison across gc flavours
       Prim.debugPrint(debug_show({
         memory_size = size;
         mutator_instructions  = mutatorInstructions;
         collector_instructions = collectorInstructions
       }));
*/
      assert (pre <= size);
      assert (preMutatorInstructions <= mutatorInstructions);
      assert (preCollectorInstructions <= collectorInstructions);
      pre := size;
      preMutatorInstructions := mutatorInstructions;
      preCollectorInstructions := collectorInstructions;
      i += 1;
    };
  }
};

await a.go(); //OR-CALL ingress go "DIDL\x00\x00"
