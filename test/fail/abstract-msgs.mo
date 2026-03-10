// In function definitions, parameters with abstract types are not fine
do { shared func foo<A <: Any>( x : A ) : () = (); };
do { shared func foo<A <: Any>() : ?A = null; };
do { func foo<A <: Any>() : () = do {
  do { shared func bar( x : A ) : () = (); };
  do { shared func bar() : async ?A { null } };
}};

// In function calls, parameters with abstract types are not fine
do { func foo<A <: Any>( f : shared A -> (), x : A )  = (f x); };
do { func foo<A <: Any>( f : shared () -> async A ) : async A = async { await (f ())}; };

// Just in types, away from definitions and calls, parameters with abstract types are still not fine
do { let x : ?(shared <A <: Any>A -> ()) = null; };
do { let x : ?(shared <A <: Any>() -> async A) = null; };
do { let x : ?(<A <: Any>(shared A -> ()) -> ()) = null; };
do { let x : ?(<A <: Any>(shared () -> async A) -> ()) = null; };


// Phantom parameters are fine
do {
  type X<B <: Any> = shared () -> ();
  func foo<A <: Any>() { shared func bar(f : X<A>) : () {}; () }
};
