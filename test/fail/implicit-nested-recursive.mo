// A recursive module type unfolds into the same type at every depth, so the
// nested search (bounded by the maximum depth) yields one candidate per
// unfolding. A direct field wins over all of them, but among the nested
// candidates, all of equal type, none is preferred: `M.next` may well hold a
// different `zero` than `M.next.next`.

type R = module { next : R; zero : Nat };

func f(zero : (implicit : Nat)) : Nat = zero;

func _direct(M : R) : Nat = f(); // M.zero

func _nested(M : module { next : R }) : Nat = f();
