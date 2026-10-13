//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
//MOC-FLAG --sanity-checks
import Prim "mo:⛔";

module Keys {
  public type Key = Prim.PreparedA;
  public type Container<T> = { key : Key; value : T };
};
type Alias = Keys.Key;
type Nested = { #empty; #key : ?[Alias] };

func identity<T>(value : T) : T { value };
func pluginIdentity<T>(value : T) : T { Prim.fixtureIdentity<T>(value) };
func apply<T>(value : T) : T {
  Prim.fixtureApply<T, T>(func (x : T) : T { identity<T>(x) }, value)
};

let key : Alias = Prim.preparedA("prepared bytes");
let container : Keys.Container<Nat> = { key; value = 42 };
let nested : Nested = #key(?[apply<Alias>(pluginIdentity<Alias>(key))]);
assert Prim.preparedReadA(container.key) == "prepared bytes";
assert container.value == 42;
switch nested {
  case (#key(?keys)) { assert Prim.preparedReadA(keys[0]) == "prepared bytes" };
  case _ { assert false };
};

let other : Prim.PreparedB = Prim.preparedB("different type");
assert Prim.preparedReadB(identity<Prim.PreparedB>(other)) == "different type";

let cells = Prim.fixtureMutTabulate<Alias>(2, func (_) { key });
let replacement = Prim.preparedA("replacement");
Prim.fixtureMutSet<Alias>(cells, 1, replacement);
assert Prim.preparedReadA(Prim.fixtureMutGet<Alias>(cells, 1)) == "replacement";
let record = Prim.fixtureMutRecord<Alias>(key);
Prim.fixtureMutWrite<Alias>(record, replacement);
assert Prim.preparedReadA(Prim.fixtureMutRead<Alias>(record)) == "replacement";
assert Prim.preparedReadA(record.spare) == "prepared bytes";

// Bottom is a subtype of every prepared type. An unreachable call must still
// compile normally; no inhabitant can reach the unchecked representation read.
func impossible(value : None) : Nat { Prim.preparedLenA(value) };
ignore impossible;

// Erasure to Any is legal; no narrowing operation recovers its prepared type.
let erased : Any = key;
ignore erased;
