//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

func invalid<T>() {
  type Escaped = { key : Prim.PreparedA; value : T };
};
ignore invalid;
