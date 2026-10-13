//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/echo
import Prim "mo:⛔";

let inferred = { echo = Prim.fixtureEcho };
let _ : { echo : Blob -> Blob } = inferred;
