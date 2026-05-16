// M9 axiom 1: Construct ⇒ Dissect.
// A value built with #tag(args) pattern-matches back to args of the
// expected refined type. The runtime value is preserved.

type Expr<A> = {
  #int : type A = Nat in Nat;
  #bool : type A = Bool in A;
};

// Refinement-only arm: #int refines A to Nat.
let e_int : Expr<Nat> = #int 42;
let n : Nat = switch e_int { case (#int x) x };
assert n == 42;

// Refinement-only arm: #bool refines A to Bool.
let e_bool : Expr<Bool> = #bool true;
let b : Bool = switch e_bool { case (#bool y) y };
assert b == true;
