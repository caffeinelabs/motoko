// A recursive module type unfolds into the same type at every depth, so the
// search (bounded by the maximum depth) yields one candidate per unfolding.
// Like any same-group candidates of equal type they are reported as ambiguous
// rather than picking the shallowest: `M.next` may well hold a different `zero`.

type R = module { next : R; zero : Nat };

func f(zero : (implicit : Nat)) : Nat = zero;

func g(M : R) : Nat = f();
