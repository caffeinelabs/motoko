// Coverage-check regression test for and-patterns. Each switch here
// is exhaustive and every case is reachable — `coverage.ml`'s
// `InAnd1` context must:
//   1) not spuriously flag any case as redundant,
//   2) not leave any refined-desc path uncovered,
//   3) propagate a refutable leg's narrowing correctly to the next
//      case so the fall-through branch is seen as useful.

type Status = { #Ok : Nat; #Err : Text };

// leg 1 refutable (narrows to #Ok 42), leg 2 irrefutable (any Status);
// fall-through to `#Ok n` and `#Err _` must stay reachable
func a(s : Status) : Nat =
  switch s {
    case ((#Ok 42) and _) 42;
    case (#Ok n) n;
    case (#Err _) 0;
  };
ignore a (#Ok 42); ignore a (#Ok 7); ignore a (#Err "x");

// leg 1 irrefutable, leg 2 refutable narrower than leg 1 — the AndP
// matches exactly where leg 2 would, but the InAnd1 flow must still
// reach both legs and leave the remaining `#Ok n` / `#Err _` slices
// uncovered-for-this-case (consumed by later cases)
func b(s : Status) : Nat =
  switch s {
    case (_ and (#Ok 99)) 99;
    case (#Ok n) n;
    case (#Err _) 0;
  };
ignore b (#Ok 99); ignore b (#Ok 1); ignore b (#Err "y");

// both legs refutable on the same shape — vacuous (accepts `#Ok 99`),
// pat1's complement still flows through to case 2 (`#Err _`)
// covering the rest of the type
func c(s : Status) : Nat =
  switch s {
    case ((#Ok _) and (#Ok 99)) 99;
    case (#Ok n) n;
    case (#Err _) 0;
  };
ignore c (#Ok 99); ignore c (#Ok 1); ignore c (#Err "z");

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp
