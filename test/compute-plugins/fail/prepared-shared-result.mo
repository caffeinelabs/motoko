//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

type Remote = shared () -> async { key : Prim.PreparedA };
let unused : ?Remote = null;
ignore unused;
