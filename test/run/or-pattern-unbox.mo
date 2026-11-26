//MOC-FLAG -A=M0190
import Prim "mo:⛔";

// An or-pattern binding `v` at different types in each arm. The variable
// gets the lub of both types (here `Any`). The backend must not store the
// right arm's value with the left arm's unboxed representation.

func f(x : { #a : Float; #b : Nat }) : Any { let (#a v) or (#b v) = x; v };
func g(x : { #a : Nat64; #b : ?Text }) : Any { let (#a v) or (#b v) = x; v };

ignore f(#a 1.5);
Prim.debugPrint("float left");
ignore f(#b 42);
Prim.debugPrint("float right");

ignore g(#a 7);
Prim.debugPrint("nat64 left");
ignore g(#b null);
Prim.debugPrint("nat64 right null");
ignore g(#b (?"hi"));
Prim.debugPrint("nat64 right text");

// the same, as a parameter pattern
func p((#a(v)) or (#b(v)) : { #a : Nat64; #b : ?Text }) : Any { v };
ignore p(#a(7));
Prim.debugPrint("param left");
ignore p(#b(null));
Prim.debugPrint("param right null")
