import Prim "mo:⛔";

// construct a 30 byte principal (illegal)
let a = Prim.Array_tabulate<Nat8>(30, func i { Prim.natToNat8(i) });
let b = Prim.arrayToBlob(a);
let p = Prim.principalOfBlob(b); // should trap!
let t = debug_show(p);
Prim.debugPrint(t);
