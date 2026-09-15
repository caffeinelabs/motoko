//MOC-FLAG --max-stable-pages=1000000
import Prim "mo:prim";

let size: Nat64 = 4 * 1024 * 1024 * 1024 + 1; // 5 GB
let wasmPageSize: Nat64 = 64 * 1024; // 64 KB
let numberOfPages: Nat64 = (size + wasmPageSize - 1) / wasmPageSize;

Prim.debugPrint(debug_show {size = Prim.stableMemorySize()});
let result = Prim.stableMemoryGrow(numberOfPages);
assert(result == 0);
Prim.debugPrint(debug_show {size = Prim.stableMemorySize()});
let location = size - 1;
let initial = Prim.stableMemoryLoadNat8(location);
assert(initial == 0);
Prim.debugPrint(debug_show {read = Prim.stableMemoryLoadNat8(location)});
let testValue: Nat8 = 123;
Prim.stableMemoryStoreNat8(location, testValue);
Prim.debugPrint(debug_show {read = Prim.stableMemoryLoadNat8(location)});
let actual = Prim.stableMemoryLoadNat8(location);
assert(actual == testValue);

//SKIP run-low
//SKIP run
//SKIP run-ir
