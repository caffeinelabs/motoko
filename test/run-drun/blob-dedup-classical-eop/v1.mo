// Provides provenance for the committed classical `v1-old.wasm` fixture used by
// `blob-dedup-classical-eop.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.

import Prim "mo:prim";

persistent actor {

  let keepAlive : [var Blob] = [var "!caf!hello", "!caf!world", "!caf!hello", "!caf!world", "!caf!letmetestyou", "bla", "blabla", "test"];

  public func test() : async () {
    Prim.debugPrint(debug_show (keepAlive.size()));

    //Prim.debugPrint(debug_show (Prim.isStorageBlobLive("!caf!hello")));
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
