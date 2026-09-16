//MOC-FLAG -A=M0145
import { debugPrint } = "mo:⛔";

// simplest: both legs succeed, two disjoint bindings on the same value
let (a : Nat) and (b : Nat) = 5;
debugPrint (debug_show {a; b});

// option pattern + bare name — ?x constrains, s captures the whole value
let (?x) and s = ?7;
debugPrint (debug_show {x; s});

// nested and: three legs bind three names to the same value
let (x3 : Nat) and (y3 : Nat) and (z3 : Nat) = 11;
debugPrint (debug_show {x3; y3; z3});

// and mixed with or — `and` binds tighter than `or`, so this groups as
// `((#a n) and m) or ((#b n) and m) : ...`
let ((#a n) and m) or ((#b n) and m) : { #a : Nat; #b : Nat } = #a 13;
debugPrint (debug_show {n; m});

// refutable AndP in a switch — the `?x4` leg is refutable and drives
// the fail-continuation through `(^^^)` on `CanFail` codes.
func describeOpt(o : ?Nat) : Text =
  switch o {
    case ((?x4) and s4) (debug_show {s4; x4});
    case null "none";
  };
debugPrint (describeOpt (?42));
debugPrint (describeOpt null);

// failing AndP leg falls through to the next case — verifies that the
// fail-continuation threads all the way out to `orsPatternFailure`.
// Tests BOTH leg-positions failing in turn.
type Status = { #Ok : Nat; #Err : Text };

// leg-1 fails: `(#Ok 42)` is the refutable side; when it fails the
// whole AndP fails and flow advances to the next case.
func leftFails(s : Status) : Text =
  switch s {
    case (#Ok 42 and s5) ("42-left: " # debug_show s5);
    case (#Ok n) ("ok " # debug_show n);
    case (#Err e) ("err " # e);
  };
debugPrint (leftFails (#Ok 42));
debugPrint (leftFails (#Ok 7));
debugPrint (leftFails (#Err "boom"));

// leg-2 fails: leg 1 `(#Ok _)` accepts any #Ok; leg 2 narrows further.
// When scrutinee is `#Ok 7`, leg 1 succeeds (any #Ok), but leg 2
// rejects (needs payload 99) — so fail-continuation must cancel leg 1's
// partial match and advance to the next case.
func rightFails(s : Status) : Text =
  switch s {
    case ((#Ok _) and (#Ok 99)) "99-right";
    case (#Ok n) ("ok " # debug_show n);
    case (#Err _) "err";
  };
debugPrint (rightFails (#Ok 99));
debugPrint (rightFails (#Ok 7));

// AndP inside a function body. A top-level `func addBoth(AndP) : Nat =
// …` desugars to `let addBoth = func …`, putting the FuncE into
// inference mode — which rejects AndP with M0261. We lift the scrutinee
// into check_pat via a switch here; equivalently, `let f : Nat -> Nat =
// func(a and b) = …` or any HOF/class position that supplies the
// expected Func type also works.
func addBoth(p : Nat) : Nat =
  switch p {
    case ((x6 : Nat) and y6) (x6 + y6);
  };
debugPrint (debug_show (addBoth 21));

// AndP nested inside a TupP
let ((a7 : Nat) and b7, c7) = (7, "world");
debugPrint (debug_show {a7; b7; c7});

// three-way and with a refutable middle leg — exercises rho threading
// in rename/subst (each leg contributes its own bindings).
func peelOpt(o : ?Nat) : Text =
  switch o {
    case (?y8 and x8 and z8) (debug_show {x8; y8; z8});
    case null "null";
  };
debugPrint (peelOpt (?11));
debugPrint (peelOpt null);

// Type-level bindings via TypPF on each AndP leg. Exercises the
// `check_pat_typ_dec` AndP arm in `typing.ml` — both legs contribute
// type identifiers that must end up in scope after the pattern. If the
// arm dropped one of `te1`/`te2`, the post-pattern reference would
// fail with M0030 (unbound type identifier).
let { type T1 } and { type T2 } : module { type T1 = Nat; type T2 = Text } =
  module { public type T1 = Nat; public type T2 = Text };
let t1v : T1 = 99;
let t2v : T2 = "type-and";
debugPrint (debug_show {t1v; t2v});

// Public let bindings with varied patterns in an object exercise vis_pat /
// vis_pat_field across every pattern arm (only VarP was reached before).
let M = object {
  public let (a, b) = (1, 2);
  public let { x = c } = { x = 3 };
  public let (?d) = ?(4 : Nat);
  public let (#tag e) = (#tag 5 : {#tag : Nat});
  public let (f : Nat) = 6;
  public let ((g : Nat) and h) = 7 : Nat;
};
assert (M.a + M.b + M.c + M.d + M.e + M.f + M.g + M.h == 35);
