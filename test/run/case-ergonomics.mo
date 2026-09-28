// Optional `;` between switch cases, unparenthesized case patterns, and the
// whitespace-sensitive `??`.

let i = 3;
let s = switch i {
  case 0 { "zero" }
  case 3 { "three" }
  case _ { "many" }
};
assert (s == "three");

func opt(o : ?Nat) : Nat = switch o {
  case null { 0 }
  case ?n { n + 1 }
};
assert (opt(null) == 0);
assert (opt(?41) == 42);

type T = { #leaf; #node : Nat };
func tag(t : T) : Nat = switch t {
  case #leaf { 0 }
  case #node(n) { n }
};
assert (tag(#leaf) == 0);
assert (tag(#node(7)) == 7);

func sign(n : Int) : Text = switch n {
  case -1 { "neg" }
  case 0 { "zero" }
  case _ { "other" }
};
assert (sign(-1) == "neg");
assert (sign(0) == "zero");

// `??x` (unspaced) still introduces two options; `?? ` (spaced) is the operator
let nn : ??Nat = ??1;
switch (nn : ??Nat) {
  case ??n { assert (n == 1) }
  case _ { assert false }
};

// record literal on the RHS of `??`
let ro : ?{ x : Nat } = null;
let r2 = ro ?? { x = 5 };
assert (r2.x == 5);

// a block on the RHS of `??` uses `do { ... }`
let d = (null : ?Nat) ?? do { let k = 2; k + 1 };
assert (d == 3);

// `or`, `and` and `: T` over unparenthesized case patterns
type Ord = { #less; #equal; #greater };
func le(o : Ord) : Bool = switch o {
  case #less or #equal { true }
  case #greater { false }
};
assert le(#less) and le(#equal) and not le(#greater);

func small(o : ?Nat) : Bool = switch o {
  case null or ?0 or ?1 { true }
  case ?_ { false }
};
assert small(null) and small(?1) and not small(?2);

type V = { #a : Nat; #b : Nat; #c };
func payload(v : V) : Nat = switch v {
  case #a(n) or #b(n) { n }
  case #c { 0 }
};
assert (payload(#b(4)) == 4);

func both(r : { x : Nat; y : Nat }) : Nat = switch r {
  case { x = 0 } and { y } { y }
  case { x } : { x : Nat } { x }
};
assert (both({ x = 0; y = 9 }) == 9);
assert (both({ x = 3; y = 9 }) == 3);
