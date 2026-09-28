import Prim "mo:prim";

actor {
  var lastInstructions = 0;
  var length = 0;
  var stableArray : [Nat] = [];

  if length > 0 {
    assert (Prim.rts_upgrade_instructions() >= lastInstructions);
    lastInstructions := Prim.rts_upgrade_instructions();
    assert(lastInstructions > 0);
  };
  // Assert that the upgrade instructions are defined, i.e. no backwards compatibility to old Motoko programs.
  assert(Prim.rts_upgrade_instructions() != 18_446_744_073_709_551_615);
  Prim.debugPrint("Ignore Diff: Upgrade instructions: " # debug_show (Prim.rts_upgrade_instructions()));

  public func increase() : async () {
    if length == 0 {
      length := 1_000;
    } else {
      length *= 10;
    };
    let newArray = Prim.Array_tabulate<Nat>(
      length,
      func(index) {
        index * index;
      },
    );
    stableArray := newArray;
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress increase "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress increase "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress increase "DIDL\x00\x00"
//CALL upgrade ""
