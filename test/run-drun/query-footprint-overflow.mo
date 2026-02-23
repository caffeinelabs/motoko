//MOC-FLAG -A=M0194
//CLASSICAL-PERSISTENCE-ONLY
import { Array_tabulate; stableVarQuery } = "mo:⛔"

actor footprint = {
    let e0 = Array_tabulate<Text>(125, // length 1144 below...
                                  func _ = "seedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseedseed");
    let e1 = [e0, e0];
    let e2 = [e1, e1];
    let e3 = [e2, e2];
    let e4 = [e3, e3];
    let e5 = [e4, e4];
    let e6 = [e5, e5];
    let e7 = [e6, e6];
    let e8 = [e7, e7];
    let e9 = [e8, e8];
    let e10 = [e9, e9];
    let e11 = [e10, e10];
    let e12 = [e11, e11];
    let e13 = [e12, e12];
    let e14 = [e13, e13];
    let e15 = [e14, e14];
    stable var expl = e15;

    public func delegate() : async Nat64 {
        let stableVarInfo = stableVarQuery();
        let { size } = await stableVarInfo();
        size
    };

};

//CALL ingress delegate "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
