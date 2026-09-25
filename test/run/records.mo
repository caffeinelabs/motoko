let rec1 = { a = 1; b = 2 };
assert(rec1.a == 1);
assert(rec1.b == 2);

let { a = x; b = y } = rec1;
assert(x == 1);
assert(y == 2);

let rec2 = { b = 3; c = 4; d = rec1 };
assert(rec2.b == 3);
assert(rec2.c == 4);

assert(rec1.a == 1);
assert(rec1.b == 2);

assert(rec2.d.a == 1);
assert(rec2.d.b == 2);

let z = 13;
let rec3 = { x; z : Int; var y; a : Int = 4; var b : Int = 5 };
assert(rec3.x == x);
assert(rec3.y == y);
assert(rec3.z == 13);
assert(rec3.a == 4);
assert(rec3.b == 5);
