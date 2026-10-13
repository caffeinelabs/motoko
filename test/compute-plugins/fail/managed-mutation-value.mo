//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
import Prim "mo:⛔";
Prim.fixtureMutSet<Nat>([var 1], 0, "bad");
Prim.fixtureMutWrite<Nat>({ var value = 1 }, "bad");
