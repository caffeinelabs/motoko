//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

func decode<T>(text : Text) : { #ok : T; #err : Text } {
  Prim.fixtureDecode<T>(text)
};
ignore decode;
