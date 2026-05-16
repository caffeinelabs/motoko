// M9 axiom 3, stronger form: round-trip with the outer type-parameter
// still abstract. Each case refines `T` (to Nat / Bool) inside its body
// and the reconstruction is well-typed at the refined `Expr<T>`; on exit
// `T` returns to abstract. This is the canonical "what GADTs are for".

type Expr<A> = {
  #int : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #eq : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>);
};

func roundtrip<T>(e : Expr<T>) : Expr<T> =
  switch e {
    case (#int n) #int n;
    case (#bool b) #bool b;
    case (#eq (cmp, x, y)) #eq (cmp, x, y);
  };

func eval<A>(e : Expr<A>) : A =
  switch e {
    case (#int n) n;
    case (#bool b) b;
    case (#eq (cmp, x, y)) cmp(eval x, eval y);
  };

// Round-trip at concrete instantiations: T = Nat
let n1 : Expr<Nat> = #int 7;
let n2 = roundtrip n1;
assert eval n1 == eval n2;
assert eval n2 == 7;

// T = Bool, refinement-only arm
let b1 : Expr<Bool> = #bool false;
let b2 = roundtrip b1;
assert eval b1 == eval b2;
assert eval b2 == false;

// T = Bool, existential arm — witness B is hidden by the outer Expr
func natEq(a : Nat, b : Nat) : Bool = a == b;
let e1 : Expr<Bool> = #eq (natEq, #int 3, #int 3);
let e2 = roundtrip e1;
assert eval e1 == eval e2;
assert eval e2 == true;
