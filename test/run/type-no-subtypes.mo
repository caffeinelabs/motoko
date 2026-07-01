// Exercise has_no_subtypes_or_supertypes via bi-directional type inference.
// bi_match calls has_no_supertypes / has_no_subtypes for invariant type variables
// constrained to (t, Any) or (Non, t).
//
// All fixed-width primitives (Bool, Nat8…Nat64, Int8…Int64, Float, Char, Text, Blob)
// return true for both NoSubtypes and NoSupertypes (exact types).

func id<T>(x : T) : T = x;

// Exact fixed-width integer types
let _ : Bool  = id(true);
let _ : Nat8  = id(0 : Nat8);
let _ : Nat16 = id(0 : Nat16);
let _ : Nat32 = id(0 : Nat32);
let _ : Nat64 = id(0 : Nat64);
let _ : Int8  = id(0 : Int8);
let _ : Int16 = id(0 : Int16);
let _ : Int32 = id(0 : Int32);
let _ : Int64 = id(0 : Int64);
let _ : Float = id(0.0 : Float);
let _ : Char  = id('a');
let _ : Text  = id("hello");
let _ : Blob  = id("" : Blob);

// Nat (NoSubtypes only — Nat <: Int, so Nat has no proper subtypes)
let _ : Nat = id(42 : Nat);

// Int (NoSupertypes only — Nat <: Int, so Int has no proper supertypes)
let _ : Int = id(42 : Int);

// Null (NoSubtypes only — Null is bottom below ?T)
let _ : Null = id(null);

// Opt — NoSupertypes: ?Nat has no proper supertypes (only Any is above it)
let _ : ?Nat = id(?(42 : Nat));

// Tup — exercises List.for_all in Tup arm; (Nat8, Bool) is exact
let _ : (Nat8, Bool) = id((0 : Nat8, true));

// Array — exercises Array arm
let _ : [Nat] = id([1, 2, 3]);

// Mut — exercises Mut arm (always true)
let _ : Nat = (do { var x = id(7); x });

// Func — exercises Func arm: (Nat8 -> Bool) is exact (both input and output exact)
let _ : Nat8 -> Bool = id(func(n : Nat8) : Bool = n == 0);

// Con(Def) arm — type alias resolves through Con; exercises Def branch in
// has_no_subtypes_or_supertypes (and ultimately reaches the Prim Bool -> true path)
type MyNat8 = Nat8;
let _ : MyNat8 = id(0 : MyNat8);

// Variant — exercises _ -> false arm; variant type is not exact (subtyping exists)
// so has_no_subtypes_or_supertypes returns false, type variable is not pinned
let _ = id(#a 42);

// Obj — exercises _ -> false arm for object types (width subtyping exists)
let _ = id({x = 1; y = 2});
