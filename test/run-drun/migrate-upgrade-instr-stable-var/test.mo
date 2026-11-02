//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";

actor {
    let state = {
        var number = 0;
        var text = "Test";
    };
    state.number += 1;
    state.text #= "Test";
    Prim.debugPrint(debug_show(state));

    public func test() : async () {
        let instructions = Prim.rts_upgrade_instructions();
        // The classical→EOP migration is measured; the old module had no counter (sentinel Nat64 max).
        assert (instructions != 18_446_744_073_709_551_615);
        assert (instructions > 0);
        Prim.debugPrint("Ignore Diff: Upgrade instructions: " # debug_show (instructions));
    };
};
