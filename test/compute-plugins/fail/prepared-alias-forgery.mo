//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

module Keys {
  public type Key = Prim.PreparedA;
};
type Alias = Keys.Key;
let forged : Alias = "unvalidated bytes";
ignore forged;
