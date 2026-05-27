// End-to-end test for structural implicit derivation (compiles and runs).
// Uses only inline definitions — no external packages required.
//MOC-FLAG --package core $MOTOKO_CORE -W=M0223,M0236,M0237

import Array "mo:core/Array";

// A simple serialisation target: list of (name, text) pairs
type Fields = [(Text, Text)];

// Structural combiner: collects per-field Text thunks into a Fields.
// __record triggers structural synthesis; elem_typ = Text.
func fieldOf(__record : [(Text, () -> Text)]) : Fields = __record.map(func((k, f)) = (k, f()));

// Per-type instances: each returns Text (the elem_typ)
module TextField { public func fieldOf(self : Text) : Text = self };
module BoolField {
  public func fieldOf(self : Bool) : Text = if self "true" else "false";
};
module NatField { public func fieldOf(self : Nat) : Text = debug_show self };

// Entry point: implicit `fieldOf : R -> Fields`
func inspect<R>(x : R, fieldOf : (implicit : R -> Fields)) : Fields = fieldOf(x);

// Single-field record
let r1 = inspect({ name = "Alice" });
assert r1 == [("name", "Alice")];

// Two-field record (Motoko sorts fields lexicographically in records)
let r2 = inspect({ active = true; count = (3 : Nat) });
assert r2 == [("active", "true"), ("count", "3")];

// Three-field record
let r3 = inspect({ flag = false; tag = "x"; n = (0 : Nat) });
assert r3 == [("flag", "false"), ("n", "0"), ("tag", "x")];

// Mutable field: T.as_immut strips the mutability so NatField.fieldOf resolves
let r4 = do {
  var count : Nat = 7;
  inspect({ count });
};
assert r4 == [("count", "7")];
