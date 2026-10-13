//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

let service : actor { call : shared () -> async () } = actor "aaaaa-aa";
ignore Prim.fixtureShape(service.call);
