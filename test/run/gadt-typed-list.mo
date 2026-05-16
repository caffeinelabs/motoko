// Length-indexed list (Vec). The N index counts the elements at the
// type level: List<A, Zero> is empty, List<A, Succ<Zero>> has one,
// List<A, Succ<Succ<Zero>>> has two, etc. head/tail are only callable
// on a non-empty list (head : List<A, Succ<M>> -> A).
//
// `type Zero = ()` would alias to unit and collapse the index — use
// distinct singleton variants instead so the GADT refinement can
// discriminate. `type Succ<N> = { #succ : N }` references N so each
// successor wraps its predecessor structurally.

type Zero = { #zero };
type Succ<N> = { #succ : N };

type List<A, N> = {
  #nil  : type N = Zero in ();
  #cons : type M, type N = Succ<M> in (A, List<A, M>);
};

func head<A, M>(l : List<A, Succ<M>>) : A =
  switch l {
    case (#cons (x, _)) x;
  };

func tail<A, M>(l : List<A, Succ<M>>) : List<A, M> =
  switch l {
    case (#cons (_, xs)) xs;
  };

let empty : List<Nat, Zero>                       = #nil ();
let one   : List<Nat, Succ<Zero>>                 = #cons (1, empty);
let two   : List<Nat, Succ<Succ<Zero>>>           = #cons (2, one);
let three : List<Nat, Succ<Succ<Succ<Zero>>>>     = #cons (3, two);

assert head one == 1;
assert head two == 2;
assert head three == 3;
assert head (tail three) == 2;
assert head (tail (tail three)) == 1;

// Reassembly axiom: destructure a non-empty list and re-cons; same
// length type preserved across the round-trip. The existential `M`
// in the #cons arm is what makes this non-trivial — destructuring
// binds a fresh M for the tail, and re-consing has to pack it back.
func reassemble<A, M>(l : List<A, Succ<M>>) : List<A, Succ<M>> =
  switch l {
    case (#cons (x, xs)) #cons (x, xs);
  };

let r1 = reassemble three;
assert head r1 == 3;
assert head (tail r1) == 2;
assert head (tail (tail r1)) == 1;
