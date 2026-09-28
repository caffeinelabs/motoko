func g(()) {};
let h = g : ((),) -> ();

// this is accepted
h(());

// This is rejected, even if it could be seen
// as passing the single argument which is unit.
// It is interpreted as passing the one tuple containing unit.
h((),);

func k((a : Nat, b : Bool)) {};
let l : ((Nat, Bool),) -> () = k;

l 42 false; // rejected, 42 is not of type (Nat, Bool)

l(42, false); // accepted, first-class pair implicitly converted to argument of call
l((42, false)); // accepted, parenthesis around values is redundant

l((42, false),); // rejected, implicitly converted to passing
// first class one-tuple (enclosing pair) as the sole argument.
