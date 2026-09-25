// test type components in types
do {
  type MT = module { type T = Null; type T<A> = A }; // reject
};

do {
  type MT = module { T : Null; type T<A> = A }; // accept
};

do {
  type Upper<A <: {}> = A;
  type B1 = module { type T<P1 <: { x : Int }> = Upper<P1> }; // accept
  type B2 = module { type T<P2 <: Null> = Upper<P2> }; // reject
  type B3 = module { type T<P3> = Upper<P3> }; // reject
};

do {
  type List<A> = ?(A, List<A>);
  type R1 = module { type List<A> = List<A> }; // accept (non-recursive, refers to previous List)
  type R2 = module { type Bogus<A> = Bogus<A> }; // reject (non-recursive, refers to nothing)
};

do {
  type Outer = module { type Open<A, B> = (A, B) }; // accept (closed type definition)
};

do {
  type Outer<A> = module { type Open<B> = (A, B) }; // reject (open type definition)
};
