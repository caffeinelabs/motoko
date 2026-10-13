//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/echo
import Prim "mo:⛔";

assert Prim.fixtureEcho("") == "";
assert Prim.fixtureEcho("abc\00\ff") == "abc\00\ff";
let echo = Prim.fixtureEcho;
let methods = { echo };
let bytes = Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(65_537,
  func(i) { Prim.natToNat8(i % 256) }));
assert echo(bytes) == bytes;
assert echo(echo(bytes)) == bytes;
assert methods.echo(bytes) == bytes;

// Fixed-input intrinsics use ordinary wrappers when a callback is needed.
func copy(bytes : Blob) : Blob = echo(bytes);
let callback : Blob -> Blob = copy;
func apply<A, B>(f : A -> B, value : A) : B = f(value);
assert callback(bytes) == bytes;
assert apply<Blob, Blob>(copy, bytes) == bytes;
