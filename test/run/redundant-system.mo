//MOC-FLAG --error-format=json
import Prim "mo:prim";

// A redundant `<system>` warns (M0196) and the call is checked as if it were absent
func id<T>(x : T) : T = x;

func _f<system>() {
  Prim.debugPrint<system>("plain"); // no type parameters
  assert id<system, Nat>(1) == 1; // remaining type arguments are kept
  assert id<system>(2) == 2; // remaining type arguments are inferred
};
