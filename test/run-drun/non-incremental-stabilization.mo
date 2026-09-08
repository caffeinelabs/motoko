//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
import Prim "mo:prim";

actor {
    stable var stableNat = 12345678901234567890123456789012345678901234567890123456789012345678901234567890;
    stable var stableInt = -2345678901234567890123456789012345678901234567890123456789012345678901234567890;
    stable var stableText = "Motoko graph-copy-based upgrade test";
    
    public func print() : async () {
        Prim.debugPrint(debug_show (stableNat));
        Prim.debugPrint(debug_show (stableInt));
        Prim.debugPrint(debug_show (stableText));
    };

    system func preupgrade() {
        Prim.debugPrint("PRE-UPGRADE HOOK!");
    };

    system func postupgrade() {
        Prim.debugPrint("POST-UPGRADE HOOK!");
    };
};

// Testing different invalid and correct combinations of explicit (de)stabilization calls

//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG -A=M0270
