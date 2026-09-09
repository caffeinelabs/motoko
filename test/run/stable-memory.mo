import Prim "mo:prim";

/* regions */

let r1 = Prim.regionNew();
let 0 = Prim.regionGrow(r1, 16);
let 16 = Prim.regionSize(r1);
Prim.debugPrint(debug_show {size = Prim.regionSize(r1)});
let 0 = Prim.regionLoadNat8(r1, 0);
Prim.debugPrint(debug_show {read = Prim.regionLoadNat8(r1, 0)});
Prim.regionStoreNat8(r1, 0, 66);
Prim.debugPrint(debug_show {read = Prim.regionLoadNat8(r1, 0)});
let 66 = Prim.regionLoadNat8(r1, 0);
Prim.debugPrint(debug_show {read = Prim.regionLoadNat8(r1, 0)});


//SKIP run-low
//SKIP run
//SKIP run-ir
