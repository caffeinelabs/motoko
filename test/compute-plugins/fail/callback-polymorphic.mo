//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

func identity<T>(value : T) : T = value;
ignore Prim.fixtureApply<Nat, Nat>(identity, 0);
