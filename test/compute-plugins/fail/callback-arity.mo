//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

type Pair = (Nat, Nat);
func pair(value : Pair) : Nat = value.0 + value.1;
ignore Prim.fixtureApply2<Nat, Nat, Nat>(pair, 1, 2);
