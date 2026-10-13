//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

var calls = 0;
func element(index : Nat) : {index : Nat; nested : [Nat]} {
  assert index == calls;
  calls += 1;
  // Nested construction and callbacks must preserve the outer array cursor.
  {index; nested = Prim.fixtureTabulate<Nat>(3, func(i) { index + i })}
};
assert Prim.fixtureTabulate<{index : Nat; nested : [Nat]}>(0, element) == [];
assert calls == 0;
let values = Prim.fixtureTabulate<{index : Nat; nested : [Nat]}>(257, element);
assert calls == 257;
assert values.size() == 257;
var index = 0;
for (value in values.values()) {
  assert value.index == index;
  assert value.nested == [index, index + 1, index + 2];
  index += 1;
};
