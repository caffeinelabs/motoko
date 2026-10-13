//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

actor {
  let state : { key : Any } = { key = Prim.preparedA("prepared bytes") };
  transient let restored : Prim.PreparedA = state.key;
};
