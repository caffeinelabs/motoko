//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed --plugin ../../../_build/compute-plugin-tests/echo
import Prim "mo:⛔";

assert Prim.fixtureEcho(Prim.fixtureShape(1 : Nat, true)) == "nb";
assert Prim.fixtureShape(Prim.fixtureEcho("abc")) == "B";
