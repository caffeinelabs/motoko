//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

let inferred = { shape = Prim.fixtureShape };
let _ : { shape : () -> Blob } = inferred;
