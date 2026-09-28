type Obj = { var x : Nat };
func foo<O <: Obj>(o : O) : Nat {
  o.x
};
let o : Obj = { var x = 1 : Nat };
assert(foo<{ var x : Nat }>(o) == 1);
