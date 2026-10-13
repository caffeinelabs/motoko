//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

let erased : Any = Prim.preparedA("prepared bytes");
let restored : Prim.PreparedA = erased;
ignore restored;
