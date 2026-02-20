//MOC-FLAG -A=M0194
var x : Int = 0;
assert (x == 0);

let _ = {a = x := 1; b = x := 2};
assert (x == 2);

let base = {b = x := 3; c = 42; a = x := 4};
assert (x == 4);

let result = {base with b = x := 5; a = x := 6 };
assert (x == 6);
assert (result == {a = (); b = (); c = 42});

let _ = {(do { x := 7; base }) with b = x := 8 };
assert (x == 8);

let _ = { (do { x := 9; base }) and (do { x := 10; object { let a = 5 } })};
assert (x == 10);
