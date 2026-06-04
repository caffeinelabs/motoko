//MOC-FLAG -W M0236
// Contextual-dot lookup is weaker than argument-position inference whenever
// the original call relies on bidirectional coercion. Two flavours:
//
//   1. Literal receivers — `check_lit` coerces (Text→Blob, Nat→Nat8) when the
//      target type is known; receiver position has no target. M0236 skips
//      `LitE` receivers via `is_postfix_exp`.
//
//   2. Container/branch receivers — `ArrayE` element checks and `IfE`/branch
//      lub use the parameter type as context. Without it, element types
//      default and lub up to `Any`. M0236 guards these via a pre-mode trial:
//      re-infer the receiver and require the un-coerced type to still subsume
//      the parameter type.

module Blob {
  public func isEmpty(self : Blob) : Bool { self == "" };
};

module Nat8 {
  public func toText(self : Nat8) : Text { ignore self; "" };
};

module Array {
  public func toBlob(self : [Nat8]) : Blob { ignore self; "" };
};

// (1) Literal receivers
// Compiles — Text literal coerces to the expected `Blob` param.
ignore Blob.isEmpty("\00\01");
// Fails — `"\00\01"` infers as `Text`, which has no `isEmpty` (M0072).
ignore "\00\01".isEmpty();

// Compiles — Nat literal coerces to the expected `Nat8` param.
ignore Nat8.toText(42);
// Fails — `42` infers as `Nat`, which is not an object type (M0070).
ignore 42.toText();

// (2) ArrayE receiver with element coercion
// Compiles — each `Nat` element coerces to `Nat8` against the param type.
ignore Array.toBlob([1, 2, 3]);
// Fails — `[1, 2, 3]` infers as `[Nat]`, which has no `toBlob` (M0072).
ignore [1, 2, 3].toBlob();

// (3) ArrayE receiver with branch lub
//   reduced from `motoko-core/Base64.encode`: elements mix `Nat8` indexings
//   with a default-typed `Nat` literal, and branches lub up via the param.
func base64Pad(b : [Nat8], i : Nat) : Blob =
  Array.toBlob([b[i], if (i == 0) b[i] else 61]);
// Fails — without context the if-branches lub to `Any`, so the literal infers
// as `[Any]` and has no `toBlob` (M0072).
func base64PadBroken(b : [Nat8], i : Nat) : Blob =
  [b[i], if (i == 0) b[i] else 61].toBlob();
ignore base64Pad;
ignore base64PadBroken;
