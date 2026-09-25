let f = func() : Int { g () };
let g = func() : Int { x };
let x = 1;
assert(f() == 1);
