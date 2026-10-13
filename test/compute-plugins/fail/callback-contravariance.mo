//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

// A callback requiring a narrower input must not accept arbitrary opaque data.
ignore Prim.fixtureApply<Any, Nat>(func (n : Nat) : Nat { n }, "wrong");
