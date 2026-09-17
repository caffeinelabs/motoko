//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stabilization-instruction-limit=10000

import Prim "mo:prim";

actor {
    stable var largeArray = Prim.Array_tabulate<Nat>(100_000, func(index) { index });

    public func check() : async () {
        Prim.debugPrint("Array of length " # debug_show (largeArray.size()));
        var index = 0;
        while (index < largeArray.size()) {
            assert (largeArray[index] == index);
            index += 1;
        };
    };

    system func preupgrade() {
        Prim.debugPrint("PRE-UPGRADE HOOK!");
    };

    system func postupgrade() {
        Prim.debugPrint("POST-UPGRADE HOOK!");
    };
};

//CALL ingress check "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG -A=M0270
