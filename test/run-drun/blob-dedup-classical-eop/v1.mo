import Prim "mo:prim";

actor {

  let keepAlive : [var Blob] = [var "!caf!hello", "!caf!world", "!caf!hello", "!caf!world", "!caf!letmetestyou", "bla", "blabla", "test"];

  public func test() : async () {
    Prim.debugPrint(debug_show (keepAlive.size()));

    //Prim.debugPrint(debug_show (Prim.isStorageBlobLive("!caf!hello")));
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//CLASSICAL-PERSISTENCE-ONLY
//MOC-FLAG --legacy-persistence
