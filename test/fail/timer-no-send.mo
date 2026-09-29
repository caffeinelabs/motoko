import { debugPrint; setTimer } = "mo:⛔";

// no system capability: must not call `setTimer`
func _bowm() {
    ignore setTimer(1_000_000, false, func () : async () { });
};

// transferred system capability: may call `setTimer`
func _gawd<system>() {
    ignore setTimer(1_000_000, false, func () : async () { });

    debugPrint<system>("caveat"); // misplaced `<system>`
    func id<T>(x : T) : T = x;
    ignore id<system, Nat>(1); // misplaced `<system>`, `Nat` is kept
    ignore id<system>(2); // misplaced `<system>`, `T` is inferred

    ignore async 42 // not allowed
};
