//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration/migrations

import Prim "mo:prim";

(with migration = (func(_ : {}) : { var field2 : Nat } { { var field2 = 121 } }))
actor {
    var field2 : Nat;

    public func check() : async () {
        Prim.debugPrint(debug_show field2);
    };
};

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP wasm-run

//CALL ingress check "DIDL\x00\x00"
