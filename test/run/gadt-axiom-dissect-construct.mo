// M9 axiom 2: Dissect ⇒ Construct (same arm).
// Inside `case (#tag p) body`, the expression `#tag(p)` re-constructs a
// value that fits the same outer GADT type as the scrutinee.

type Expr<A> = {
  #int  type A = Nat  : Nat;
  #bool type A = Bool : A;
};

// Identity over Expr<Bool>: dissect and re-construct each arm. The
// reconstructed value must inhabit Expr<Bool> again.
func id_expr(e : Expr<Bool>) : Expr<Bool> {
  switch e {
    case (#bool b) #bool b;
    // case (#int n) is statically unreachable (refinement A=Nat ≠ Bool),
    // pruned by M5 — omitting it is still exhaustive.
  };
};

let e : Expr<Bool> = #bool false;
let e2 = id_expr e;
let b : Bool = switch e2 { case (#bool b) b };
assert b == false;
