//MOC-FLAG -A=M0194
//CLASSICAL-PERSISTENCE-ONLY
import { stableVarQuery } "mo:⛔";

actor footprint = {
    stable var s : Nat64 = 42;
    stable var hello : Text = "Hey You!";
    var lost : Text = "This will be lost";
    let e0 = "seed";
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
    stable var expl = e10;

    public func delegate() : async Nat64 {
        let stableVarInfo = stableVarQuery();
        let { size } = await stableVarInfo();
        size
    };

    system func preupgrade() {
        hello #= " Hello World!"
    }
};

//CALL ingress __motoko_stable_var_info "DIDL\x00\x00"
//CALL query __motoko_stable_var_info "DIDL\x00\x00"
//CALL ingress delegate "DIDL\x00\x00"
//CALL ingress delegate "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
