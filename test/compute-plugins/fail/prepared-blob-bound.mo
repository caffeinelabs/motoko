//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
import Prim "mo:⛔";

func bytes<T <: Blob>(value : T) : Blob { value };
ignore bytes<Prim.PreparedA>(Prim.preparedA("prepared bytes"));
