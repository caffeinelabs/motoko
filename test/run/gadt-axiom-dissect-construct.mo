// M9 axiom 2: Dissect ⇒ Construct (same arm).
// Inside `case (#tag p) body`, the expression `#tag(p)` re-constructs a
// value that fits the same outer GADT type as the scrutinee.

type Expr<A> = {
  #int : type A = Nat in Nat;
  #bool : type A = Bool in A;
};

// Identity over Expr<Bool>: dissect and re-construct each arm. The
// reconstructed value must inhabit Expr<Bool> again.
func id_expr(e : Expr<Bool>) : Expr<Bool> =
  switch e {
    case (#bool b) #bool b;
    // case (#int n) is statically unreachable (refinement A=Nat ≠ Bool),
    // pruned by M5 — omitting it is still exhaustive.
  };

let e : Expr<Bool> = #bool false;
let e2 = id_expr e;
let b : Bool = switch e2 { case (#bool b) b };
assert b == false;

// let-else mirror of `id_expr`: dissect via refutable let-else,
// re-construct from the binding. Must land back at Expr<Bool>.
func id_expr_le(e : Expr<Bool>) : Expr<Bool> {
  let #bool b = e else { return e };
  #bool b
};

let e3 = id_expr_le e;
let b2 : Bool = switch e3 { case (#bool b) b };
assert b2 == false;
