// Exercise `has_no_subtypes_or_supertypes` (type.ml) via bi_match's
// `choose_under_constrained`. That heuristic runs ONLY for an INVARIANT,
// under-constrained type variable. `[var T]` in the return makes T invariant,
// and a one-sided use leaves it under-constrained, forcing the solver to consult
// the predicate:
//   upper<T> : only an upper bound (Non .. t) → resolves to t iff `has_no_subtypes t`
//   lower<T> : only a lower bound (t .. Any)  → resolves to t iff `has_no_supertypes t`
// (An `id<T>(x : T) : T` pins T exactly and never reaches this path.)
func upper<T>(_consume : T -> ()) : [var T] = [var];
func lower<T>(_produce : () -> T) : [var T] = [var];

// Fixed-width primitives are EXACT (both no-subtypes and no-supertypes) — exercise
// both wrappers across the Prim scalar arm.
let _ = upper(func (_ : Bool) {});
let _ = lower(func () : Bool = true);
let _ = upper(func (_ : Nat8) {});
let _ = lower(func () : Nat64 = 0);
let _ = upper(func (_ : Int8) {});
let _ = lower(func () : Int64 = 0);
let _ = upper(func (_ : Float) {});
let _ = lower(func () : Char = 'a');
let _ = upper(func (_ : Text) {});
let _ = lower(func () : Blob = "");

// Nat: NoSubtypes only (Nat <: Int) → resolves via `upper` only.
let _ = upper(func (_ : Nat) {});
// Int: NoSupertypes only → resolves via `lower` only.
let _ = lower(func () : Int = 0);
// Null: NoSubtypes only (bottom below ?T).
let _ = upper(func (_ : Null) {});
// Opt of an exact type: NoSupertypes (only Any is above ?Nat8).
let _ = lower(func () : ?Nat8 = ?(0 : Nat8));

// Tup arm (List.for_all over exact components).
let _ = upper(func (_ : (Nat8, Bool)) {});
// Array arm (recurses on the element).
let _ = upper(func (_ : [Nat8]) {});
// Mut arm: `[var Nat8]` is Array (Mut Nat8) → recursion hits `Mut _ -> true`.
let _ = upper(func (_ : [var Nat8]) {});
// Func arm (flips the mode for the domain): `Nat8 -> Bool` is exact both ways.
let _ = upper(func (_ : Nat8 -> Bool) {});
let _ = lower(func () : Nat8 -> Bool = func (n : Nat8) : Bool = n == 0);
// Con(Def) arm: the alias resolves through Con to the Prim scalar.
type MyNat8 = Nat8;
let _ = upper(func (_ : MyNat8) {});
