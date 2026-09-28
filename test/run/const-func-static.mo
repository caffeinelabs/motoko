import Prim "mo:⛔";

let before = Prim.rts_heap_size();
func higher_order(foo: () -> ()) = foo();
func bar() = ();
higher_order(bar);
let after = Prim.rts_heap_size();
assert(+after - before == 0);
Prim.debugPrint("Ignore Diff: heap size increase " # debug_show (+after - before));

//SKIP run
//SKIP run-low
//SKIP run-ir
