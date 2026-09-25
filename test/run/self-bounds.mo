type List<A, B <: List<Nat,None>> = (A, B);

type List1<A, B <: List1<A,None>> = (A, B);

type List2<A, C, B <: List2<A,C,B>> = (A, B);

type List3<B <: List3<B,A,C>, A, C> = (A, B);
