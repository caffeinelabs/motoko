// Skolem escape via parametric subsumption to `Any`.
//
// Two `#pack` arms in one switch produce two distinct fresh skolems
// `X1` and `X2` (Path B mint per pat.at). The polymorphic
// `consume<T>(_ : Pair<(T, T)>) : ()` would need
//   (X1, X1) <: (T, T)  AND  (X2, X2) <: (T, T)
// for some single `T`. Inference picks the only common supertype
// of two abstract-bound cons — `T = Any` — and the call type-checks.
//
// This is the dual of the `.0`-projection M9010 rule: there, we
// reject naming a position inside a still-packed existential; here,
// inference quietly coalesces two distinct opened witnesses through
// the universal upper bound `Any`. The witnesses escape their `case`
// scopes without a hard error.
//
// By parametricity `consume` cannot observe T, so this is
// strictly-safe today. But conceptually it should be rejected —
// `T = Any` is a lossy subsumption that erases the two distinct
// skolem identities. The strict Cardelli rule is "skolems may flow
// into a generic parameter, but inference must not promote them to
// `Any` to satisfy the bound."
//
// **Desired behaviour (not yet implemented):** at least emit a
// warning when inference's chosen solution for a type variable is
// `Any` *and* the only reason was that multiple distinct
// abstract-bound cons had to coalesce. A future inference-side fix
// (mark inference vars that arose from generic-param solving so
// they refuse the `Any` solution when multiple distinct skolems
// would merge) would turn this into a hard error.
//
// Test currently type-checks (exit 0). When the warning lands the
// capture will pick it up; when the hard error lands the capture
// will reject. Either way the regression suite catches the change.

type Pair<T> = (T, T);
type Pack = { #pack : type X in (X, X) };

let payload = (25, 42);
let p : Pack = #pack payload;

func consume<T>(_ : Pair<(T, T)>) : () = ();

switch (p, p) {
  case (#pack different, #pack different1) consume (different, different1)
};
