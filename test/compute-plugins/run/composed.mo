//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/echo --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

assert Prim.fixtureEcho(Prim.fixtureShape(1 : Nat, true)) == "nb";
assert Prim.fixtureShape(Prim.fixtureEcho("abc")) == "B";
