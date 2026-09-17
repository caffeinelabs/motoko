//MOC-FLAG -A=M0194,M0198

// .vals() is deprecated (M0269) on arrays and Blob; .values() is the alias.
// Both the value-position (`let it = x.vals()`) and the call-position
// (`for (... in x.vals())`) spellings go through try_infer_dot_exp.

import Prim "mo:⛔";

do {
  let a = [1, 2, 3];
  let it = a.vals();            // value position, immutable array
  ignore (it.next());
  for (x in a.vals()) {}        // call position
};

do {
  let m : [var Nat] = [var 1, 2, 3];
  let it = m.vals();            // mutable array
  ignore (it.next());
  for (x in m.vals()) {}
};

do {
  let b : Blob = "hi";
  let it = b.vals();            // Blob
  ignore (it.next());
  for (x in b.vals()) {}
};

// .values() must NOT warn
do {
  let a = [1, 2, 3];
  let it = a.values();
  ignore (it.next());
  for (x in a.values()) {}
};

do {
  let b : Blob = "hi";
  let it = b.values();
  ignore (it.next());
  for (x in b.values()) {}
};

Prim.debugPrint "ok";
