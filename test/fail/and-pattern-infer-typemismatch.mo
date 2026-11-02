// Both legs of the `and`-pattern carry explicit type annotations,
// but those types are incompatible. In inference position (function
// parameter without an outer type annotation) the typer infers each
// leg independently and reports M0262 when the two inferred types
// can't both accept the same scrutinee.
func f((_x : Nat) and (_y : Text)) = ();
ignore f
