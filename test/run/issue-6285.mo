import Prim "mo:⛔";

// Self tail call whose argument is a tuple-returning call, not a literal tuple:
// the temps must be assigned from the bound tuple.
func step(a : Nat, b : Nat) : (Nat, Nat) = (a - 1, b + a);
func f(a : Nat, b : Nat) : Nat = if (a == 0) b else f (step(a, b));
Prim.debugPrint(debug_show (f(5, 0)));

// Tuple-typed parameters: projecting from the destination variable instead would
// be well-typed for `a`, and silently compute the wrong values.
func step2(a : (Nat, Nat), b : (Nat, Nat)) : ((Nat, Nat), (Nat, Nat)) = ((a.0 - 1, a.1), (b.0 + a.0, b.1));
func g(a : (Nat, Nat), b : (Nat, Nat)) : Nat = if (a.0 == 0) b.0 else g (step2(a, b));
Prim.debugPrint(debug_show (g((5, 0), (0, 0))));
