// `label x [: T] = <default> <body>` — labeled early-exit with a fallthrough
// default (issue #6163).  Desugars to `label x [: T] do { <body>; <default> }`,
// so the default is the value when no `break x` fires and — being the block's
// tail — is evaluated only on demand (like the `else` branch of `if`).
//
// Two defaulted forms, each with a non-bottom or a bottom (`None`, e.g.
// `return`) default:
//
//            | non-bottom default | bottom default (return)
//   ---------+--------------------+------------------------
//   x : T =  |       (A1)         |        (A2)
//   x =      |   type error*      |        (B2)
//
//   * compact + non-bottom needs the label type, which is only () without an
//     annotation — see test/fail/label-default-compact.mo.

// ---- (A1) annotated, non-bottom default ----

// Bool: the `break` value wins; the default is the fallthrough.
func found(xs : [Nat], target : Nat) : Bool {
  label hit : Bool = false
    for (x in xs.vals()) { if (x == target) break hit true }
};
assert (found([1, 2, 3], 2));
assert (not found([1, 2, 3], 9));

// Option: the default `null : Null` widens to `?Nat` via the annotation.
func firstEven(xs : [Nat]) : ?Nat {
  label res : ?Nat = null
    for (x in xs.vals()) { if (x % 2 == 0) break res (?x) }
};
assert (firstEven([1, 3, 4, 5]) == ?4);
assert (firstEven([1, 3, 5]) == null);

// Variant Result: a non-nullary default must be parenthesised (the default
// operand is parsed at `exp_nullary`).
type R = { #ok : Nat; #err : Text };
func lookup(xs : [Nat], target : Nat) : R {
  label res : R = (#err "not found")
    for (x in xs.vals()) { if (x == target) break res (#ok x) }
};
assert (lookup([10, 20, 30], 20) == #ok 20);
assert (lookup([10, 20, 30], 99) == #err "not found");

// ---- (A2) annotated, bottom default (`return` : None <: T) ----

// "Find the index, or return a sentinel": fallthrough returns early.
func indexOf(xs : [Nat], target : Nat) : Nat {
  label found : Nat = (return 999)
    for (i in xs.keys()) { if (xs[i] == target) break found i }
};
assert (indexOf([10, 20, 30], 20) == 1);
assert (indexOf([10, 20, 30], 99) == 999);

// ---- (B2) compact, bottom default ----
// No annotation, so the label is unit; the bottom default (None) fits () by
// subtyping and the breaks are unit.  The control flow is the payload.
func hasEven(xs : [Nat]) : Bool {
  label scan = (return false)          // no even found: return false
    for (x in xs.vals()) { if (x % 2 == 0) break scan };  // found one: fall out
  true
};
assert (hasEven([1, 3, 4]));
assert (not hasEven([1, 3, 5]));

// ---- laziness: the default runs only on fallthrough ----
var defaultEvals = 0;
func pick(hit : Bool) : Nat {
  label out : Nat = (do { defaultEvals += 1; 99 })
    do { if hit { break out 7 } }
};
assert (pick(true) == 7);
assert (defaultEvals == 0);   // a `break` fired — default must NOT run
assert (pick(false) == 99);
assert (defaultEvals == 1);   // fell through — default evaluated exactly once
