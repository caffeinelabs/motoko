// Fixed-width naturals wider than a machine word must survive an upgrade with their values
// intact. Mirrors upgrade-bignums.mo, which does the same for the arbitrary-precision types.
//
// These are represented as fixed-size limb blobs rather than bignums, so this exercises a
// different persistence path: the value is a Blob to the graph copier, but the stable type
// descriptor carries a code of its own so that a Nat and a Nat256 can never be mistaken for
// one another across an upgrade.
import Prim "mo:prim";

actor {
    var small : Nat128 = 340282366920938463463374607431768211455;
    var big : Nat256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
    var arr : [Nat256] = [0, 1, 18446744073709551616];

    public func modify() : async () {
        // wrapping, so the maxima roll over rather than trapping -- and rolling over is
        // itself worth persisting, since it proves the whole limb blob was written back
        small +%= 1;
        big +%= 2;
        arr := [arr[0] +% 7, arr[1] *% 3, arr[2] +% arr[2]];
    };

    public func print() : async () {
        Prim.debugPrint(debug_show (small));
        Prim.debugPrint(debug_show (big));
        Prim.debugPrint(debug_show (arr));
    };
};

//CALL ingress print "DIDL\x00\x00"
//CALL ingress modify "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
//CALL ingress modify "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress print "DIDL\x00\x00"
