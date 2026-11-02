//MOC-FLAG -A=M0194
// compatible_typ arms via and-patterns: switch legs (inferred) and lambda legs
// (explicit annotations, hitting the both-explicit T.compatible path), + Null/Any.

type Shape = {#circle : Nat; #rect : (Nat, Nat)};
let _a = switch (42 : Nat) { case (x and y) (x + y) };
let _b = switch ([1, 2, 3] : [Nat]) { case (xs and ys) (xs.size() + ys.size()) };
let _c = switch (?(42 : Nat) : ?Nat) { case (ox and oy) 0 };
let _d = switch ((1, true) : (Nat, Bool)) { case ((n, _) and (_, b)) (n + (if b 1 else 0)) };
let _e = switch (#circle 5 : Shape) { case (s and t) 0 };
let _f = switch ({x = 1; y = true} : {x : Nat; y : Bool}) { case ({x} and {y}) (x + (if y 1 else 0)) };
let _g = switch (null : Null) { case (n and m) 0 };
let _h = switch (42 : Any) { case ((x : Any) and (y : Any)) 0 };
assert (_a == 84 and _b == 6 and _c == 0 and _d == 2 and _e == 0 and _f == 2 and _g == 0 and _h == 0);

type V = {#a : Nat; #b : Bool};
let doObj = func((x : {a : Nat}) and (y : {a : Nat})) : Nat = x.a + y.a;
let doTup = func((x : (Nat, Bool)) and (_y : (Nat, Bool))) : Nat = x.0;
let doArr = func((xs : [Nat]) and (_ys : [Nat])) : Nat = xs.size();
let doMutArr = func((xs : [var Nat]) and (_ys : [var Nat])) : Nat = xs.size();
let doOpt = func((ox : ?Nat) and (_oy : ?Nat)) : Nat = switch ox { case (?n) n; case null 0 };
let doVariant = func((c : V) and (_d : V)) : Nat = switch c { case (#a n) n; case (#b _) 0 };
let doFunc = func((f : Nat -> Nat) and (g : Nat -> Nat)) : Nat = f(1) + g(2);
assert (doObj ({a = 21}) == 42);
assert (doTup (5, true) == 5);
assert (doArr ([1, 2, 3]) == 3);
assert (doMutArr ([var 1, 2, 3]) == 3);
assert (doOpt (?7) == 7);
assert (doVariant (#a 10 : V) == 10);
assert (doFunc (func(n : Nat) : Nat = n * 2) == 6);
