import Prim "mo:⛔";

let a0 = Prim.rts_total_allocation();
let a = [1, 2, 3];
assert(a.size() == 3);
let b : Blob = "hello";
assert(b.size() == 5);
let c : Text = "hello world";
assert(c.size() == 11);

let a1 = Prim.rts_total_allocation();

Prim.debugPrint("Allocation delta:  " # debug_show (a1 - a0 : Nat));

// evaluation order tests
ignore((do { Prim.debugPrint "a"; a }).size());
ignore((do { Prim.debugPrint "b"; b }).size());
ignore((do { Prim.debugPrint "c"; c }).size());

ignore((do { Prim.debugPrint "a"; a }).size(Prim.debugPrint "A"));
ignore((do { Prim.debugPrint "b"; b }).size(Prim.debugPrint "B"));
ignore((do { Prim.debugPrint "c"; c }).size(Prim.debugPrint "C"));
