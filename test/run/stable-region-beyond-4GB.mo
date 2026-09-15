//MOC-FLAG --max-stable-pages=1000000
import Prim "mo:prim";

let size: Nat64 = 4 * 1024 * 1024 * 1024 + 1; // 5 GB
let wasmPageSize: Nat64 = 64 * 1024; // 64 KB
let numberOfPages: Nat64 = (size + wasmPageSize - 1) / wasmPageSize;

Prim.debugPrint(debug_show {size = Prim.stableMemorySize()});
let r1 = Prim.regionNew();
let result = Prim.regionGrow(r1, numberOfPages);
assert(result == 0);
Prim.debugPrint(debug_show {size = Prim.regionSize(r1)});
let location = size - 1;
let initial = Prim.regionLoadNat8(r1, location);
assert(initial == 0);
Prim.debugPrint(debug_show {read = Prim.regionLoadNat8(r1, location)});
let testValue: Nat8 = 123;
Prim.regionStoreNat8(r1, location, testValue);
Prim.debugPrint(debug_show {read = Prim.regionLoadNat8(r1, location)});
let actual = Prim.regionLoadNat8(r1, location);
assert(actual == testValue);

//SKIP run-low
//SKIP run
//SKIP run-ir
