//MOC-FLAG -W M0236
// Regression for the M0236 LitE-skip: contextual-dot lookup never tries the
// literal coercion that argument-position `check_lit` does, so for any literal
// receiver requiring coercion, `Module.f(lit)` may type-check while the
// equivalent `lit.f()` does not. The autofix must therefore skip `LitE`
// receivers entirely. Captured here: (1) the originals must NOT trigger M0236,
// (2) the equivalent rewrites must fail to type-check.
module Blob {
  public func isEmpty(self : Blob) : Bool { self == "" };
};

module Nat8 {
  public func toText(self : Nat8) : Text { ignore self; "" };
};

// `check_lit` coerces Text→Blob against the expected `Blob` param — compiles.
ignore Blob.isEmpty("\00\01");
// Contextual-dot sees `Text`, never tries Text→Blob — M0072.
ignore "\00\01".isEmpty();

// `check_lit` coerces Nat→Nat8 against the expected `Nat8` param — compiles.
ignore Nat8.toText(42);
// Contextual-dot sees `Nat`, never tries Nat→Nat8 — M0070.
ignore 42.toText();
