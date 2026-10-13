//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
import Prim "mo:⛔";
let values = [var 0];
Prim.fixtureMutOperands<Nat>(
  func() { Prim.debugPrint("array"); values },
  func() { Prim.debugPrint("index"); 1 },
  func() { Prim.debugPrint("value"); 99 },
);
