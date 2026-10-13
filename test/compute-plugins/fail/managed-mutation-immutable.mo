//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
import Prim "mo:⛔";
Prim.fixtureMutSet<Nat>([1, 2], 0, 3);
Prim.fixtureMutWrite<Nat>({ value = 1 }, 2);
