import Prim "mo:⛔";

actor {
  var counter = 1;
  var emptyFuncGCInstructions = 0;
  public func emptyFunction() : async () {};
  public func getEmptyFuncGCInstructions() : async () {
    emptyFuncGCInstructions := Prim.rts_collector_instructions();
  };
  public func runInstructions() : async () {
    var n = 2000;
    while n > 0 {
      n := n - 1;
    };
    counter += 1;
  };
  public func checkInstructions() : async () {
    let prevFuncInstr = Prim.rts_mutator_instructions();
    let prevFuncGCInstr = Prim.rts_collector_instructions();

    let lifetimeInstr = Prim.rts_lifetime_instructions();

    // The lifetime instructions should be at least the gc+code of the previous function.
    assert lifetimeInstr >= (prevFuncInstr + prevFuncGCInstr);

    // We ran 3 functions so far, so the number of instructions given by GC
    // should be at least 3 * emptyFuncGCInstructions.
    assert lifetimeInstr >= 3 * emptyFuncGCInstructions;
    // Same but add also the instructions of the function runInstructions.
    assert lifetimeInstr >= 3 * emptyFuncGCInstructions + prevFuncInstr;

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress emptyFunction "DIDL\x00\x00"
//CALL ingress getEmptyFuncGCInstructions "DIDL\x00\x00"
//CALL ingress runInstructions "DIDL\x00\x00"
//CALL ingress checkInstructions "DIDL\x00\x00"
