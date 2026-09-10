//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stabilization-instruction-limit=10000

import Prim "mo:prim";

actor {
    transient let flexibleNat = do {
        Prim.debugPrint("Initialize flexible Nat");
        1
    };
    stable var stableNat = do {
        Prim.debugPrint("Initialize stable Nat");
        12345678901234567890123456789012345678901234567890123456789012345678901234567890;
    };
    stable var stableInt = do {
        Prim.debugPrint("Initialize stable Int");
        -2345678901234567890123456789012345678901234567890123456789012345678901234567890;
    };
    stable var stableText = do {
        Prim.debugPrint("Initialize stable text");
        "Motoko incremental graph-copy-based upgrade test";
    };
    stable var stableArray = do {
        Prim.debugPrint("Initialize stable array");
        Prim.Array_tabulate<Nat>(100, func (index) { index });
    };
    stable var stableObject = do {
        Prim.debugPrint("Initialize stable object");
        { stableNat; stableInt; stableText; stableArray; }
    };
    transient let flexibleText = do {
        Prim.debugPrint("Initialize flexible text");
        "Flexible text"
    };
    // To trigger incremental serialization/deserialization
    stable var _largeStableArray = do {
         Prim.Array_tabulate<Nat>(100_000, func (index) { index });
    };

    public func print() : async () {
        Prim.debugPrint(debug_show (flexibleNat));
        Prim.debugPrint(debug_show (flexibleText));
        Prim.debugPrint(debug_show (stableNat));
        Prim.debugPrint(debug_show (stableInt));
        Prim.debugPrint(debug_show (stableText));
        Prim.debugPrint(debug_show (stableArray));
        Prim.debugPrint(debug_show (stableObject));
    };

    system func preupgrade() {
        Prim.debugPrint("PRE-UPGRADE HOOK!");
    };

    system func postupgrade() {
        Prim.debugPrint("POST-UPGRADE HOOK!");
    };
};

//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG -A=M0270
