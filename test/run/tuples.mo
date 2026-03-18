//MOC-FLAG -A=M0194
let tup1 = (1,2);
let (x,y) = tup1;
assert(x == 1);
assert(y == 2);
assert (tup1.0 == 1);
assert (tup1.1 == 2);

let nested1 = (1,(2,3));
let (_,nested2) = nested1;
let (_,z) = nested2;
let (_,(_,a)) = nested1;
assert (a == 3);
assert (z == 3);
assert (nested1.0 == 1);
assert ((nested1.1).0 == 2);
assert ((nested1.1).1 == 3);

type T = (Nat, T);
func f(x : T) { let (n, _) = x; };
