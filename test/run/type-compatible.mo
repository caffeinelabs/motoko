// Exercise T.compatible via and-patterns (AndP) where both legs are explicit.
// typing.ml:3914 calls T.compatible t1 t2 when both pattern legs have explicit types.
// compatible_typ arms exercised: Prim, Array, Opt, Tup, Variant, Obj

//MOC-FLAG -A=M0194

// Prim (same): compatible_typ (Prim p1) (Prim p2) when p1 = p2 -> true
let _a = switch (42 : Nat) {
  case (x and y) (x + y)
};

// Array: compatible_typ (Array t1') (Array t2') -> recursive on element types
let _b = switch ([1, 2, 3] : [Nat]) {
  case (xs and ys) (xs.size() + ys.size())
};

// Opt: compatible_typ (Opt t1') (Opt t2') -> recursive
let _c = switch (?(42 : Nat) : ?Nat) {
  case (ox and oy) 0
};

// Tup: compatible_typ (Tup ts1) (Tup ts2) via compatible_list
let _d = switch ((1, true) : (Nat, Bool)) {
  case ((n, _) and (_, b)) (n + (if b 1 else 0))
};

// Variant: compatible_tags exercises Lib.Both in compatible_tags
type Shape = {#circle : Nat; #rect : (Nat, Nat)};
let _e = switch (#circle 5 : Shape) {
  case (s and t) 0
};

// Obj: compatible_fields exercises Lib.Both in compatible_fields
let _f = switch ({x = 1; y = true} : {x : Nat; y : Bool}) {
  case ({x} and {y}) (x + (if y 1 else 0))
};

// Prim Null and ?T: Null is compatible with ?T
let _g = switch (null : Null) {
  case (n and m) { ignore n; 0 }
};

assert (_a == 84);
assert (_b == 6);
assert (_c == 0);
assert (_d == 2);
assert (_e == 0);
assert (_f == 2);
assert (_g == 0);
