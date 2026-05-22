// Multiple distinct duplicates in one `and`-pattern. The combinator-
// based detection in `gather_pat` (`try_all` + `try_both`) must
// collect ALL duplicates in a single type-check pass — `error`'s
// `Recover` is caught per-binding, iteration continues, then the
// combinator re-raises so the surrounding compilation still bails.
//
// Expect three M0260 diagnostics: one per duplicate name.
let (a : Nat, b : Nat, c : Nat) and (a : Nat, b : Nat, c : Nat) = (1, 2, 3);
ignore a;
ignore b;
ignore c
