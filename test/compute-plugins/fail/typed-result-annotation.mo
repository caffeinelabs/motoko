//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

let decode : Text -> { #ok : Text; #err : Text } = Prim.fixtureDecode;
ignore decode;
