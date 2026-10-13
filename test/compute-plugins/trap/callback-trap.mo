//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

ignore Prim.fixtureApply<Nat, Nat>(func (_n : Nat) : Nat {
  Prim.trap("callback trap propagates")
}, 0);
