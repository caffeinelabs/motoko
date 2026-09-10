//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";

actor {
    stable let state = {
        var number = 0;
        var text = "Test";
    };
    state.number += 1;
    state.text #= "Test";
    Prim.debugPrint(debug_show(state));

    public func test() : async () {
        Prim.debugPrint("Ignore Diff: Upgrade instructions: " # debug_show (Prim.rts_upgrade_instructions()));
    };
};
