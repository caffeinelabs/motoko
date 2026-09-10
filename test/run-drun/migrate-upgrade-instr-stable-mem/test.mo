//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";

import StableMemory "../stable-mem/StableMemory";


actor {
    ignore StableMemory.grow(10);
    Prim.debugPrint("Stable memory size: " # debug_show(StableMemory.size()));

    public func test() : async () {
        Prim.debugPrint("Upgrade instructions: " # debug_show (Prim.rts_upgrade_instructions()));
    };
};
