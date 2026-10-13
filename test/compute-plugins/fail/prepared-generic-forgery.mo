//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

func identity<T>(value : T) : T { value };
ignore identity<Prim.PreparedA>("unvalidated bytes" : Blob);
