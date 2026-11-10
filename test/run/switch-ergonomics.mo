// Syntax ergonomics (#6344) in the north-star form of #6352 (the brace discipline):
// unparenthesized heads terminated by `{`, braced bodies, bare operands for operator-like keywords,
// `{` in expression position is a record and a block is `do { }`.
// The still-supported legacy forms live in switch-ergonomics-legacy.mo.

func inc(n : Nat) : Nat { n + 1 };

// full expressions as condition
let a = true;
let b = false;
if a and not b {} else { assert false };

var i = 0;
while i < 3 { i += 1 };
assert i == 3;

// full expressions as scrutinee: calls, projections, operators, indexing
switch inc(1) {
  case 2 {}
  case _ { assert false }
};

let p = { x = 1; y = 2 };
switch p.x {
  case 1 {}
  case _ { assert false }
};

switch i + 1 {
  case 4 {}
  case _ { assert false }
};

let arr = [1, 2, 3];
switch arr[1] {
  case 2 {}
  case _ { assert false }
};

// a record scrutinee needs parentheses: the `{` after the scrutinee always opens the cases
switch ({ x = 1 }) {
  case r { assert r.x == 1 }
};

// a call in the condition: unspaced `(` extends the head
if inc(2) == 3 {} else { assert false };

// operators spaced on both sides, or on neither, extend the head
var k : Int = 3;
if k - 1 > 0 {} else { assert false };
if k-1 > 0 {} else { assert false };
while k - 1 > 0 { k -= 1 };
assert k == 1;

// `if` as an expression, with variant branches
let cmp = if i > 0 { #pos } else { #zero };
assert cmp == #pos;

// `else if` chains through atomic and extended heads alike, braced throughout
if inc(0) == 0 { assert false } else if inc(0) == 1 {} else { assert false };
let chain = if inc(0) == 0 { 0 } else if a { 1 } else if inc(1) == 2 { 2 } else { 3 };
assert chain == 1;

// `for p in e { }`: the pattern is delimited by `in`, the head by `{`
var sum = 0;
for x in arr.values() { sum += x };
assert sum == 6;
for (k, v) in [(1, 2), (3, 4)].values() { assert k + 1 == v };

// arms are braced; no `;` between them
let s = switch i {
  case 0 { "zero" }
  case 3 { "three" }
  case _ { "many" }
};
assert s == "three";

// unparenthesized case patterns with deterministic extent
func opt(o : ?Nat) : Nat {
  switch o {
    case null { 0 }
    case ?n { n + 1 }
  }
};
assert opt(null) == 0;
assert opt(?41) == 42;

type T = { #leaf; #node : Nat };
func tag(t : T) : Nat {
  switch t {
    case #leaf { 0 }
    case #node(n) { n }
  }
};
assert tag(#leaf) == 0;
assert tag(#node(7)) == 7;

func sign(n : Int) : Text {
  switch n {
    case -1 { "neg" }
    case 0 { "zero" }
    case _ { "other" }
  }
};
assert sign(-1) == "neg";
assert sign(0) == "zero";

// `??x` (unspaced) still introduces two options; `?? ` (spaced) is the operator
let nn : ??Nat = ??1;
switch (nn : ??Nat) {
  case (??n) { assert n == 1 }
  case _ { assert false }
};

// the right-hand side of `??` is expression position: `{` is a record, a block is `do { }`
let ro : ?{ x : Nat } = null;
let r2 = ro ?? { x = 5 };
assert r2.x == 5;
let d = (null : ?Nat) ?? do { let k = 2; k + 1 };
assert d == 3;

// `do { }` is an expression, so it is a valid head like any other: tolerated and discouraged, not legacy —
// a natural consequence of the grammar, as `if { c } { }` is in Rust; steering away from it is a formatter/lint concern
if do { let k = 1; k == 1 } {} else { assert false };
var w = 0;
while do { w < 2 } { w += 1 };
assert w == 2;

// operator-like keywords take a bare operand: `break l e` like `return e`
let bk = label l : Nat loop { break l inc(41) };
assert bk == 42;
let bd = label m : Nat loop { break m do { let k = 6; k * 7 } };
assert bd == 42;
