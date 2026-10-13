//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
//MOC-FLAG --sanity-checks
import Prim "mo:⛔";

let values = Prim.fixtureMutTabulate<Nat>(4, func(i) { i * 10 });
let alias = values;
Prim.fixtureMutSet<Nat>(values, 2, 99);
assert alias[2] == 99;
alias[1] := 88;
assert Prim.fixtureMutGet<Nat>(values, 1) == 88;
assert Prim.fixtureMutTabulate<Nat>(0, func(_) { assert false; 0 }).size() == 0;

// Generic values retain the same references, including closures and cycles.
func write<T>(a : [var T], value : T) { Prim.fixtureMutSet<T>(a, 0, value) };
let counter = [var 1];
let payload = func() : Nat { counter[0] };
let closures = Prim.fixtureMutTabulate<() -> Nat>(2, func(_) { payload });
write<() -> Nat>(closures, payload);
counter[0] := 42;
assert closures[0]() == 42 and closures[1]() == 42;
let record = Prim.fixtureMutRecord<[var Nat]>(counter);
let narrow : { var value : [var Nat] } = record;
assert Prim.fixtureMutRead<[var Nat]>(narrow)[0] == 42;
assert Prim.fixtureMutMarker(record) == 7;
let replacement = [var 53];
Prim.fixtureMutWrite<[var Nat]>(narrow, replacement);
assert record.value[0] == 53;
assert record.spare[0] == 42; // Fresh mutable fields, shared initial payload.
record.value[0] := 71;
assert replacement[0] == 71;

// An ordinary record can carry extra fields in any hash order.
let wider = { var value = 4; extra = "keep"; a = false; z = 55 };
Prim.fixtureMutWrite<Nat>(wider, 9);
assert wider.value == 9 and wider.extra == "keep";
assert Prim.fixtureMutRead<Nat>(wider) == 9;
type Node = { var value : ?Node; var spare : ?Node; marker : Nat };
let node : Node = Prim.fixtureMutRecord<?Node>(null);
Prim.fixtureMutWrite<?Node>(node, ?node);
switch (Prim.fixtureMutRead<?Node>(node)) {
  case (?same) { same.spare := ?node; switch (node.spare) { case (?_) {}; case null { assert false } } };
  case null { assert false };
};

// Scalar stores use the compiler's pointer-free path; nested options do not.
let bools = Prim.fixtureMutTabulate<Bool>(1, func(_) { false });
Prim.fixtureMutSet<Bool>(bools, 0, true);
assert bools[0];
let cell = Prim.fixtureMutRecord<Bool>(false);
Prim.fixtureMutWrite<Bool>(cell, true);
assert cell.value and not cell.spare;
let options = Prim.fixtureMutTabulate<??Null>(1, func(_) { null });
Prim.fixtureMutSet<??Null>(options, 0, ??null);
assert options[0] == ??null;

// Full-operand helpers evaluate each expression once before resolving a slot.
var order = "";
Prim.fixtureMutOperands<Nat>(
  func() { order #= "a"; values },
  func() { order #= "i"; 0 },
  func() {
    order #= "v";
    Prim.fixtureMutSet<Nat>(values, 0, 123);
    ignore Prim.Array_init<Nat>(131072, 1);
    456
  },
);
assert order == "aiv" and values[0] == 456;
order := "";
let constructed = Prim.fixtureMutFields<Nat>(
  func() { order #= "a"; Prim.fixtureMutGet<Nat>(values, 0) },
  func() { order #= "b"; Prim.fixtureMutSet<Nat>(values, 0, 789); 91 },
);
assert order == "ab";
order := "";
Prim.fixtureMutFieldOperands<Nat>(
  func() { order #= "o"; constructed },
  func() {
    order #= "v";
    Prim.fixtureMutWrite<Nat>(constructed, 400);
    ignore Prim.Array_init<Nat>(131072, 2);
    456
  },
);
assert order == "ov";
assert constructed.value == 456 and constructed.spare == 91;
assert values[0] == 789;
assert Prim.fixtureMutEarlyArray() == "early";
assert Prim.fixtureMutEarlyRecord() == "early";
