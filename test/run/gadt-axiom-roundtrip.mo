// M9 axiom 3: Round-trippable through existential.
// Construct an arm with an existential (witness packed at construction),
// pattern-match it (witness becomes a skolem in scope), re-construct
// from the destructured parts. Type is preserved (Expr<Bool> in,
// Expr<Bool> out); the skolem identity from match is the witness on
// re-construct.

type Expr<A> = {
  #int  type A = Nat  : Nat;
  #bool type A = Bool : A;
  #eq   type A = Bool, type B : ((B, B) -> Bool, Expr<B>, Expr<B>);
};

func natEq(a : Nat, b : Nat) : Bool = a == b;

func eval<A>(e : Expr<A>) : A =
  switch e {
    case (#int n) n;
    case (#bool b) b;
    case (#eq (cmp, x, y)) cmp(eval x, eval y);
  };

// Round-trip: dissect each arm, immediately re-construct it.
// The existential's skolem from the pattern-match becomes the witness
// on re-construct; outer type Expr<Bool> is preserved.
func roundtrip(e : Expr<Bool>) : Expr<Bool> =
  switch e {
    case (#bool b) #bool b;
    case (#eq (cmp, x, y)) #eq (cmp, x, y);
  };

let orig1 : Expr<Bool> = #eq (natEq, #int 5, #int 5);
let trip1 = roundtrip orig1;
assert eval orig1 == eval trip1;       // semantically preserved
assert eval trip1 == true;

func boolEq(a : Bool, b : Bool) : Bool = a == b;
let orig2 : Expr<Bool> = #eq (boolEq, #bool true, #bool true);
let trip2 = roundtrip orig2;
assert eval orig2 == eval trip2;
assert eval trip2 == true;

let orig3 : Expr<Bool> = #bool false;
let trip3 = roundtrip orig3;
assert eval orig3 == eval trip3;
assert eval trip3 == false;
