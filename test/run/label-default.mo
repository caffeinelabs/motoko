// `label x = <default> <body>` — labeled early-exit with a fallthrough default
// (issue #6163).  With no annotation the label's type is taken from the default
// (the block's final expression), exactly like a `var`/`let` initializer; an
// annotation, when present, wins.  The default is the block's tail, so it is
// evaluated only on demand, like the `else` branch of `if`.

// ---- compact: type inferred from the default ----

// Bool, in a checking position (the function's return type).
func found(xs : [Nat], target : Nat) : Bool {
  label hit = false
    for (x in xs.vals()) { if (x == target) break hit true }
};
assert (found([1, 2, 3], 2));
assert (not found([1, 2, 3], 9));

// ... and in a synthesis position (a `let` with no annotation): the label type
// is inferred from `false` regardless of context.
let anyTwo = label hit = false
  for (x in [1, 2, 3].vals()) { if (x == 2) break hit true };
assert anyTwo;

// A sentinel whose bare type is too narrow is widened by annotating the
// *default* — the initializer rule, exactly like `var x : ?Nat = null`.
func firstEven(xs : [Nat]) : ?Nat {
  label r = (null : ?Nat)
    for (x in xs.vals()) { if (x % 2 == 0) break r (?x) }
};
assert (firstEven([1, 3, 4, 5]) == ?4);
assert (firstEven([1, 3, 5]) == null);

// ---- annotation wins (the flag is consulted only when unannotated) ----

func punt(xs : [Nat]) : ?Int {
  label p : ?Int = null              // annotation ?Int wins over (null : Null)
    for (x in xs.vals()) { if (x == 2) break p (?1) }
};
assert (punt([1, 2, 3]) == ?1);
assert (punt([1, 3]) == null);

// A bottom default needs the annotation (the inferred tail type would be None,
// which no real `break` value conforms to).
func indexOf(xs : [Nat], target : Nat) : Nat {
  label found : Nat = (return 999)   // annotation Nat; (return …) : None <: Nat
    for (i in xs.keys()) { if (xs[i] == target) break found i }
};
assert (indexOf([10, 20, 30], 20) == 1);
assert (indexOf([10, 20, 30], 99) == 999);

// A *compact* bottom default infers to None, which is "forgotten": the label
// falls back to the unit default, so a unit `break` exits it while the loop
// otherwise diverges through the default (here `return`).
func hasEven(xs : [Nat]) : Bool {
  label scan = (return false)
    for (x in xs.vals()) { if (x % 2 == 0) break scan };
  true
};
assert (hasEven([1, 3, 4]));
assert (not hasEven([1, 3, 5]));

// ---- laziness: the default runs only on fallthrough ----
var defaultEvals = 0;
func pick(hit : Bool) : Nat {
  label out = (do { defaultEvals += 1; 99 })   // inferred Nat
    do { if hit { break out 7 } }
};
assert (pick(true) == 7);
assert (defaultEvals == 0);   // a `break` fired — default must NOT run
assert (pick(false) == 99);
assert (defaultEvals == 1);   // fell through — default evaluated exactly once

// ---- bare label unchanged ----
func loopit() { label l loop { break l } };
loopit();
