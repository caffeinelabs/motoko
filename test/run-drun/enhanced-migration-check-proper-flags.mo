//MOC-FLAG --enhanced-migration

import Prim "mo:prim";

(with multi_migration = (func(_ : {}) : { var field1 : Nat } { { var field1 = 121 } }))
persistent actor {
    var field1 : Nat;

    public func check() : async () {
        Prim.debugPrint(debug_show field1);
    };
};

//CLASSICAL-PERSISTENCE-ONLY
//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP wasm-run

//CALL ingress check "DIDL\x00\x00"
