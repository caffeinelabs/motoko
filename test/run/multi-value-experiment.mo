//MOC-FLAG --experimental-multi-value
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

func pair(n : Nat64) : (Nat64, Nat64) = (n, n + 1);

let (a, b) = pair(42);
Prim.debugPrint(debug_show a # " " # debug_show b);

// Multi-value codegen: `pair`'s helper function emits a multi-result
// signature instead of stashing values through `multi_val_*` globals.
//CHECK: (result i{{32|64}} i{{32|64}})
//CHECK: func $pair{{.*}}(result i{{32|64}} i{{32|64}})
//CHECK-NOT: multi_val_
