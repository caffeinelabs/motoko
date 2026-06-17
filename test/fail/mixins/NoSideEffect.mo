import Prim "mo:prim";
import Timer "mo:core/Timer";
import { type Duration } "mo:core/Types";

mixin() {
    var b : Bool;

    func myFunc() : Nat { 5 };
    // This fails!
    transient let x : Nat = myFunc();

    func check() : async () {
        Prim.debugPrint(debug_show "Version 0");
    };
};
