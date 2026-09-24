//MOC-FLAG -W M0236

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

module Array {
  public func toBlob(self : [Nat8]) : Blob { ignore self; "" };
  public func size(self : [Nat]) : Nat { ignore self; 0 };
};

// Compiles — each `Nat` element coerces to `Nat8` against the param type.
ignore Array.toBlob([1, 2, 3]);
// Fails — `[1, 2, 3]` infers as `[Nat]`, which has no `toBlob`.
ignore [1, 2, 3].toBlob();

// ArrayE receiver with branch lub (reduced from `motoko-core/Base64.encode`):
// a `Nat8` indexing and a default-typed `Nat` literal lub up via the param.
// Compiles — the param makes both branches `Nat8`.
func pad(b : [Nat8], i : Nat) : Blob = Array.toBlob([b[i], if (i == 0) b[i] else 61]);
// Fails — without context the branches lub to `Any`, so `[Any]` has no `toBlob`.
func padBroken(b : [Nat8], i : Nat) : Blob = [b[i], if (i == 0) b[i] else 61].toBlob();
ignore pad;
ignore padBroken;
