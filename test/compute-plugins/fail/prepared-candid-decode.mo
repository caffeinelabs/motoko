//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

let decoded : ?Prim.PreparedA = from_candid(to_candid("bytes" : Blob));
ignore decoded;
