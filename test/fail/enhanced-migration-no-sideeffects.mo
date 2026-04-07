//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test1
//MOC-FLAG -A=M0194
//MOC-FLAG --package core $MOTOKO_CORE

import Prim "mo:prim";
import Timer "mo:core/Timer";
import { type Duration } "mo:core/Types";

actor {
    var b : Bool;

    func myFunc() : Nat { 5 };
    // This fails!
    transient let x : Nat = myFunc();

    func check() : async () {
        Prim.debugPrint(debug_show "Version 0");
    };

    func disallowMe() : Duration {
        b := true;
        #seconds 5;
    };
    // This fails!
    transient let y = Timer.setTimer<system>(disallowMe(), check);

    func allowed<system>() : Duration {
        // This is fine, the user knows what they're doing.
        b := false;
        #seconds 55;
    };
    // This works.
    transient var z = Timer.setTimer<system>(allowed<system>(), check);
};
