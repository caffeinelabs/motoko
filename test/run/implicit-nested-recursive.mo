// A recursive module type unfolds to itself, so nested search must not
// report `M.next.zero`, `M.next.next.zero`, ... as extra candidates.
// Resolution happens inside functions so no value of the type is needed.

type R = module { next : R; zero : Nat };

type A = module { b : B; zero : Nat };
type B = module { a : A };

func f(zero : (implicit : Nat)) : Nat = zero;

func direct(M : R) : Nat = f();
func mutual(M : A) : Nat = f();

ignore (direct, mutual);
