// Demonstrates that __tuple and __record (binary) are disambiguated by the hole's
// arg-list shape even when both combiners are in scope simultaneously.
//
//   implicit : (R, R) -> T   — two-argument function  → __record (binary path)
//   implicit : P -> T        — single-argument function, even if `type P = (R, R)`
//                              T.promote P = T.Tup[R;R] → __tuple
//
// The dispatch depends on whether the tuple is written inline or through a type alias.
// Writing `({}, {}) -> T` is always two-arg and routes to __record (binary) even if
// __tuple is also in scope.
//
// This mirrors Motoko's existing convention: arity is syntactic.
// `(A, B) -> C` is a two-arg function
// `P -> C` is one-arg even when `type P = (A, B)` — an alias is never
// unpacked into multiple arguments. Implicit derivation just reads that arity.
//
// Combiners must live in separate modules (Motoko prohibits duplicate top-level names).
//MOC-FLAG -W=M0223,M0236,M0237

// ── Combiners in separate modules ────────────────────────────────────────────

// __record (binary path): receives [(field_name, thunk for per-field binary result)]
module RecRender {
  public func render(__record : [(Text, () -> Text)]) : Text {
    var s = "{";
    var first = true;
    for ((lab, v) in __record.vals()) {
      if (not first) { s #= ", " };
      s #= lab # ":" # v();
      first := false;
    };
    s #= "}";
    s;
  };
};

// __tuple: receives [thunk for per-element unary result], formats as "(v, v, ...)"
module TupRender {
  public func render(__tuple : [() -> Text]) : Text {
    var s = "(";
    var first = true;
    for (v in __tuple.vals()) {
      if (not first) { s #= ", " };
      s #= v();
      first := false;
    };
    s #= ")";
    s;
  };
};

// ── Per-field binary instances (for __record binary) ─────────────────────────
module TextBin { public func render(x : Text, y : Text) : Text = x # "|" # y };
module NatBin {
  public func render(x : Nat, y : Nat) : Text = debug_show x # "|" # debug_show y;
};

// ── Per-element unary instances (for __tuple) ─────────────────────────────────
module TextUn { public func render(self : Text) : Text = self };
module NatUn { public func render(self : Nat) : Text = debug_show self };

// ── Entry points ──────────────────────────────────────────────────────────────

// Two-arg hole: (R, R) → Text — __record fires via binary path
func zip<R>(x : R, y : R, render : (implicit : (R, R) -> Text)) : Text = render(x, y);

// Single-arg hole: T → Text — dispatches to __tuple (TupleKind) when T is a tuple
func show<T>(x : T, render : (implicit : T -> Text)) : Text = render(x);

// ── Tests ─────────────────────────────────────────────────────────────────────

// (R, R) → Text with R = { n : Nat; name : Text }
// __record (binary) fires; fields in lexicographic order: n (NatBin), then name (TextBin)
type P = { n : Nat; name : Text };
let a : P = { n = 1; name = "Alice" };
let b : P = { n = 2; name = "Bob" };
assert zip(a, b) == "{n:1|2, name:Alice|Bob}";

// T → Text with T = (Text, Nat) — single-arg, tuple domain
// __tuple fires; element 0 (TextUn), element 1 (NatUn)
assert show(("hello", 3 : Nat)) == "(hello, 3)";

// Edge case: ({}, {}) → Text — empty record, TWO-arg → __record (binary), no fields
// The fold body never executes; combiner receives [] and returns "{}"
func zeroFields(x : {}, y : {}, render : (implicit : ({}, {}) -> Text)) : Text = render(x, y);
assert zeroFields({}, {}) == "{}";

// Alias case: type Pair = (Text, Nat) is stored as a type alias in the AST.
// `implicit : Pair -> Text` has a single arg — routes to __tuple (not __record binary).
type Pair = (Text, Nat);
func showPair(x : Pair, render : (implicit : Pair -> Text)) : Text = render(x);
assert showPair(("world", 7 : Nat)) == "(world, 7)";
