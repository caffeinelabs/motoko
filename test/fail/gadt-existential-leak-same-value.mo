// Variant-arm existential leak via same-value double-match in a
// single switch. Sharper than `gadt-cross-arm-mixing.mo` (two
// different values, nested switches): here ONE value `p` is matched
// twice in the tuple `(p, p)` of a single switch, proving that
// Path B's fresh-skolem mint is keyed on `pat.at`, not on
// value-identity.
//
// The two TagPs at the two pat.at regions mint distinct fresh
// skolems X1, X2. `g a` then needs X1 <: X2 (g's param type), which
// fails by stamp inequality even though X1 and X2 happen to descend
// from the same runtime witness.

type Pack = { #pack : type X in (X, X -> Text) };

let p : Pack = #pack (25, func (x : Nat) : Text = debug_show x);

switch (p, p) {
  case (#pack (a, _), #pack (_, g)) g a
};
