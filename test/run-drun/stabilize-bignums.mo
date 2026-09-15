//MOC-FLAG --stabilization-instruction-limit=10000
import Prim "mo:prim";

actor {
    stable var unsigned : Nat = 12345678901234567890123456789012345678901234567890123456789012345678901234567890;
    stable var signed : Int = -12345678901234567890123456789012345678901234567890123456789012345678901234567890;

    public func modify() : async () {
        unsigned := 2 * unsigned + 1;
        signed := 2 * signed - 1;
    };

    public func print() : async () {
        Prim.debugPrint(debug_show (unsigned));
        Prim.debugPrint(debug_show (signed));
    };
};

//CALL ingress print "DIDL\x00\x00"
//CALL ingress modify "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL ingress modify "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress print "DIDL\x00\x00"
