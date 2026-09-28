// Normal function allow subtyping
do { func foo(h : Int -> ()) : (Nat -> ()) = h };

// Shared functions do, too
do { func foo(h : shared Int -> ()) : (shared Nat -> ()) = h };

// The same with abstract types
do { func foo<A <: {}>(h : {} -> ()) : ({ x : Nat } -> ()) = h };
do { func foo<A <: {}>(h : shared {} -> ()) : (shared { x : Nat } -> ()) = h };

// But not incomaptible types
do { func foo(h : shared Nat -> ()) : (shared Text -> ()) = h };
