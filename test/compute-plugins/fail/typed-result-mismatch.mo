//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

let _ : { #ok : Nat; #err : Text } = Prim.fixtureDecode<Text>("text");
