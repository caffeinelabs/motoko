//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

type Leaf = Prim.PreparedA;
type Loop<T> = Other<T>;
type Other<T> = Loop<T>;
let unused : ?Leaf = null;
ignore unused;
