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

Prim.debugPrint ("a = " # debug_show a);
Prim.debugPrint ("r = " # debug_show r);
Prim.debugPrint ("lit = " # debug_show lit);
