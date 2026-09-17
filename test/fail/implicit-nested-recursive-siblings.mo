// Pruning recursive unfoldings must not hide genuine ambiguity: sibling
// modules of equal type are still both candidates, while the recursive
// `M.next` chain contributes nothing new.

type R = module { next : R; other : module { zero : Nat }; zero : Nat };

func f(zero : (implicit : Nat)) : Nat = zero;

func g(M : R) : Nat = f();
