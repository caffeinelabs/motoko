//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

assert Prim.fixtureShape() == "";
assert Prim.fixtureShape(1 : Nat) == "n";
assert Prim.fixtureShape(1 : Nat, "hello", true, "blob" : Blob) == "ntbB";
assert Prim.fixtureShape((1, "tuple")) == "T";
assert Prim.fixtureShape(((1, "tuple"),)) == "T";
assert Prim.fixtureShape([1, 2], ?3, { value = 4 }, #hello) == "aorv";

// Inferred aliases and record fields retain the variadic intrinsic type.
let shape = Prim.fixtureShape;
let methods = { shape };
assert shape(1 : Nat, false) == "nb";
assert methods.shape("text") == "t";
assert (if (true) shape else methods.shape)(?false) == "o";

var effects = "";
assert shape(do { effects #= "a"; 1 : Nat },
  do { effects #= "b"; false }) == "nb";
assert effects == "ab";
