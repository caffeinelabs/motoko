// Syntax ergonomics (#6344): unparenthesized scrutinees and conditions,
// optional case semicolons, unparenthesized case patterns, `??` cleanup.

func inc(n : Nat) : Nat = n + 1;

// full expressions as condition
let a = true;
let b = false;
if a and not b {} else { assert false };

var i = 0;
while i < 3 { i += 1 };
assert (i == 3);

// full expressions as scrutinee: calls, projections, operators
switch inc(1) {
  case 2 {};
  case _ { assert false };
};

let p = { x = 1; y = 2 };
switch p.x {
  case 1 {};
  case _ { assert false };
};

switch i + 1 {
  case 4 {};
  case _ { assert false };
};

// a record scrutinee needs parentheses (the `{` after the scrutinee
// position always opens the cases)
switch ({ x = 1 }) {
  case r { assert (r.x == 1) };
};

// an unspaced `(` extends the condition (call argument) ...
if inc(2) == 3 {} else { assert false };
// ... while a spaced `(` (or any other atom) is the branch, keeping the
// pre-existing `if (cond) branch` style working (atomic heads only: an
// unparenthesized extended head requires braced branches, M0272)
if (inc(2) == 3) (assert true) else (assert false);
let cmp = if (i > 0) #pos else #zero;
assert (cmp == #pos);
// else-if chains work in the braced form
if inc(0) == 0 { assert false } else if inc(0) == 1 {} else { assert false };
let legacy = if (1 > 0) null else (?5);
assert (legacy == null);
// same for `[`: unspaced indexes the scrutinee, spaced starts the body
let arr = [1, 2, 3];
switch arr[1] {
  case 2 {};
  case _ { assert false };
};

// `for` heads no longer need parentheses either; the legacy form still works
var sum = 0;
for x in arr.vals() { sum += x };
assert (sum == 6);
for (k, v) in [(1, 2), (3, 4)].vals() { assert (k + 1 == v) };
for (x in arr.vals()) { sum += x };
assert (sum == 12);

// optional `;` between cases
let s = switch i {
  case 0 { "zero" }
  case 3 { "three" }
  case _ { "many" }
};
assert (s == "three");

// unparenthesized case patterns with deterministic extent
func opt(o : ?Nat) : Nat = switch o {
  case null 0;
  case ?n n + 1;
};
assert (opt(null) == 0);
assert (opt(?41) == 42);

type T = { #leaf; #node : Nat };
func tag(t : T) : Nat = switch t {
  case #leaf 0
  case #node(n) n
};
assert (tag(#leaf) == 0);
assert (tag(#node(7)) == 7);

func sign(n : Int) : Text = switch n {
  case -1 "neg"
  case 0 "zero"
  case _ "other"
};
assert (sign(-1) == "neg");
assert (sign(0) == "zero");

// `??x` (unspaced) still introduces two options; `?? ` (spaced) is the operator
let nn : ??Nat = ??1;
switch (nn : ??Nat) {
  case (??n) assert (n == 1);
  case _ assert false;
};

// record literal on the RHS of `??`
let ro : ?{ x : Nat } = null;
let r2 = ro ?? { x = 5 };
assert (r2.x == 5);

// a block on the RHS of `??` uses `do { ... }`
let d = (null : ?Nat) ?? do { let k = 2; k + 1 };
assert (d == 3);
