//MOC-FLAG -W M0236
// Contextual-dot lookup is weaker than argument-position inference for literal
// receivers. When a literal flows into an argument with a known parameter type,
// `check_lit` runs bidirectional coercion (Text→Blob, Nat→Nat8); the dot-
// receiver position is inferred against the literal's default type and never
// re-tries that coercion. So `Module.f(lit)` may type-check while the
// "equivalent" `lit.f()` does not — which is why M0236 skips `LitE` receivers.

module Blob {
  public func isEmpty(self : Blob) : Bool { self == "" };
};

module Nat8 {
  public func toText(self : Nat8) : Text { ignore self; "" };
};

// Compiles — Text literal coerces to the expected `Blob` param.
ignore Blob.isEmpty("\00\01");
// Fails — `"\00\01"` infers as `Text`, which has no `isEmpty` (M0072).
ignore "\00\01".isEmpty();

// Compiles — Nat literal coerces to the expected `Nat8` param.
ignore Nat8.toText(42);
// Fails — `42` infers as `Nat`, which is not an object type (M0070).
ignore 42.toText();
