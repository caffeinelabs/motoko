//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --sanity-checks
import Prim "mo:⛔";

module Keys {
  public type Key = Prim.PreparedA;
  public type Box<T> = { key : Key; value : T };
};
type A = Keys.Key;
type B = Prim.PreparedB;

func identity<T>(value : T) : T = Prim.fixtureIdentity<T>(value);

// Bottom is a subtype of every prepared type, although no caller can supply it.
func unreachable(value : None) : Nat = Prim.preparedLenA(value);
ignore unreachable;

let empty = Prim.preparedA("");
assert Prim.preparedLenA(empty) == 0;
assert Prim.preparedReadA(empty) == "";
let binary : Blob = "\00\ff\01\80\00\7f\fe\02\00\ff\00\01\02\03\04\05\06";
let a = Prim.preparedA(binary);
let b = Prim.preparedB(binary);
assert Prim.preparedReadA(a) == binary;
assert Prim.preparedReadB(b) == binary;

let long = Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(65_537,
  func(i) { Prim.natToNat8((i * 131 + 17) % 256) }));
let prepared = Prim.preparedA(long);
let box : Keys.Box<Nat> = { key = prepared; value = 19 };
assert Prim.preparedReadA(identity<Keys.Box<Nat>>(box).key) == long;
assert box.value == 19;
let aliases : [A] = [prepared, identity<A>(prepared)];
let record = { key = aliases[0]; read = func() : Blob {
  Prim.preparedReadA(aliases[1])
} };
let wrapped : ?{ key : A; read : () -> Blob } = ?record;
switch wrapped {
  case null assert false;
  case (?value) {
    assert Prim.preparedLenA(value.key) == 65_537;
    assert value.read() == long;
  };
};

// Nested plugin callbacks can allocate and grow memory while opaque arguments
// are retained in managed locals and on the operand stack.
var calls = 0;
let returned = Prim.fixtureTwice<A>(func(value : A) : A {
  calls += 1;
  ignore Prim.Array_init<Nat>(131_072, calls);
  let nested = Prim.fixtureApply<A, A>(func(inner : A) : A {
    ignore Prim.Array_init<Nat>(131_073, calls + 1);
    assert Prim.preparedReadA(inner) == long;
    identity<A>(inner)
  }, value);
  assert Prim.preparedReadA(value) == long;
  nested
}, prepared);
assert calls == 2;
assert Prim.preparedReadA(returned) == long;
assert Prim.preparedReadA(prepared) == long;
assert Prim.preparedReadB(identity<B>(b)) == binary;

// Reusing a prepared value does not clone it or allocate a wrapper per call.
var total = 0;
var iteration = 0;
let allocationBefore = Prim.rts_total_allocation();
while (iteration < 1_000) {
  total += Prim.preparedLenA(identity<A>(prepared));
  iteration += 1;
};
let allocationAfter = Prim.rts_total_allocation();
assert total == 65_537_000;
assert allocationAfter == allocationBefore;
