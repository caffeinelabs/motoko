//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration/migrations
//MOC-FLAG --package core $MOTOKO_CORE

import Timer "mo:core/Timer";

import Prim "mo:prim";

actor {
    let zero : Nat;
    let one : [var Nat];
    let two : [var Text];

    func check() : async () {
        Prim.debugPrint(debug_show "Version 0");
        Prim.debugPrint(debug_show { zero; one; two });
    };

    transient let _t : Nat = Timer.setTimer<system>(#seconds 0, check);

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP wasm-run
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
