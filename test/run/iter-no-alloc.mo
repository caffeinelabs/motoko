import Prim "mo:⛔";

let iters = 10_000;

func iter<A>(what : Text, xs : [A]) {
  let before = Prim.rts_heap_size();
  for (_ in xs.keys()) {};
  for (_ in xs.vals()) {};
  for (_ in xs.values()) {};
  let after = Prim.rts_heap_size();
  Prim.debugPrint("Allocation per iteration (" # what # "): " # debug_show ((after-before) / iters : Nat));
};

type FixOpt = ?FixOpt;

iter("Nat", Prim.Array_tabulate<Nat>(iters,func x = x));
iter("Nat16", Prim.Array_tabulate<Nat16>(iters,func x = Prim.natToNat16 x));
iter("Nat32", Prim.Array_tabulate<Nat32>(iters,func x = Prim.natToNat32 x));
iter("?Nat, all null", Prim.Array_tabulate<?Nat>(iters,func x = null));
iter("FixOpt, all ?null", Prim.Array_tabulate<FixOpt>(iters,func x = ?null));
iter("FixOpt, all ??null", Prim.Array_tabulate<FixOpt>(iters,func x = ??null));
iter("?Nat, all values", Prim.Array_tabulate<?Nat>(iters,func x = ?x));
iter("record", Prim.Array_tabulate<{foo : Nat}>(iters,func x = ({ foo = x })));

//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG -A=M0269
