// Exercise compatible_typ interior arms via and-patterns in lambda parameters.
// Lambda parameters go through infer_pat (not check_pat), so when both legs
// of an AndP are explicit (AnnotP), the true,true branch fires and calls
// T.compatible t1 t2 at typing.ml:3914.
//
// This covers:
//   compatible_typ Obj arm + compatible_fields Lib.Both arm
//   compatible_typ Tup arm + compatible_list
//   compatible_typ Array arm
//   compatible_typ Opt arm
//   compatible_typ Variant arm + compatible_tags Lib.Both arm
//   compatible_typ Func arm
//   compatible_typ Mut arm (mutable array)

//MOC-FLAG -A=M0194

// Obj arm + compatible_fields Lib.Both arm:
// Both legs annotate the same structural object type; physically distinct allocations.
// compatible_fields is called with alignment Lib.Both for field "a".
let doObj = func((x : {a : Nat}) and (y : {a : Nat})) : Nat = x.a + y.a;
assert (doObj ({a = 21}) == 42);

// Tup arm + compatible_list:
// compatible_typ is called on Tup[Nat,Bool], Tup[Nat,Bool] -> hits Tup arm.
let doTup = func((x : (Nat, Bool)) and (y : (Nat, Bool))) : Nat {
  let (n, _) = x;
  ignore y;
  n
};
assert (doTup (5, true) == 5);

// Array arm:
// compatible_typ is called on Array Nat, Array Nat -> hits Array arm.
let doArr = func((xs : [Nat]) and (ys : [Nat])) : Nat {
  ignore ys;
  xs.size()
};
assert (doArr ([1, 2, 3]) == 3);

// Mutable array arm:
// compatible_typ is called on Mut(Array Nat), Mut(Array Nat) -> hits Mut arm
// then recurses into Array Nat.
let doMutArr = func((xs : [var Nat]) and (ys : [var Nat])) : Nat {
  ignore ys;
  xs.size()
};
assert (doMutArr ([var 1, 2, 3]) == 3);

// Opt arm:
// compatible_typ is called on Opt Nat, Opt Nat -> hits Opt arm.
let doOpt = func((ox : ?Nat) and (oy : ?Nat)) : Nat {
  ignore oy;
  switch ox { case (?n) n; case null 0 }
};
assert (doOpt (?7) == 7);

// Variant arm + compatible_tags Lib.Both arm:
// V type has tags #a and #b; compatible_tags is called with Lib.Both for each.
type V = {#a : Nat; #b : Bool};
let doVariant = func((c : V) and (d : V)) : Nat {
  ignore d;
  switch c { case (#a n) n; case (#b _) 0 }
};
assert (doVariant (#a 10 : V) == 10);

// Func arm:
// compatible_typ is called on Func types -> always true.
let doFunc = func((f : Nat -> Nat) and (g : Nat -> Nat)) : Nat =
  f(1) + g(2);
assert (doFunc (func(n : Nat) : Nat = n * 2) == 6);
