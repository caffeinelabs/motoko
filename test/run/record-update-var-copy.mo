import Prim "mo:⛔";

// `{ r with ... }` shallow-copies r's inherited var fields into fresh
// mutable cells, exactly as the hand-expanded record literal
// `{ var x = r.x; var y = r.y; ... }` would.

let r = { var x = 1; var y = 2 };

let a = { r with z = 3 };
let lit = { var x = r.x; var y = r.y; z = 3 };

// both captured the base's values at construction time
assert a.x == 1;
assert a.y == 2;
assert lit.x == 1;
assert lit.y == 2;

// mutating the copy does not propagate to the base
a.x += 10;
assert a.x == 11;
assert r.x == 1;

// mutating the base does not propagate to the copy
r.y += 20;
assert r.y == 22;
assert a.y == 2;

// a and the literal are independent copies as well
a.y += 1;
assert a.y == 3;
assert lit.y == 2;

// a field initializer that mutates an inherited non-overwritten var must feed
// the copied cell: the copy snapshots after initializers run (matches wasm).
let r2 = { var p = 1; q = 0 };
let b2 = { r2 with q = do { r2.p += 1; 0 } };
assert b2.p == 2; // gap var copied after the initializer's side effect
assert b2.q == 0;
assert r2.p == 2; // base advanced independently

Prim.debugPrint ("a = " # debug_show a);
Prim.debugPrint ("r = " # debug_show r);
Prim.debugPrint ("lit = " # debug_show lit);
