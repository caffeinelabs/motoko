//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

type Pair = (Nat, Nat);
func pair(value : Nat) : Pair = (value, value);
ignore Prim.fixtureSplit<Nat, Nat, Nat>(pair, 1);
