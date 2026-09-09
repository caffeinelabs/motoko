import Prim "mo:prim";

actor {

  let keepAlive : [var Blob] = [var "!caf!hello", "!caf!world", "!caf!hello", "!caf!world", "!caf!letmetestyou", "bla", "blabla", "test"];

  public func test() : async () {

    // Make one of the blobs dead.
    keepAlive[4] := "bla";

    // Force a GC run.
    var n = 20;
    // try to trigger GC.
    while (n > 0) {
      // Allocate large array.
      let _arr = Prim.Array_init<Nat>(1_000 * 1_000, 1);
      await async {};
      n -= 1;
    };

    Prim.debugPrint(debug_show (keepAlive.size()));
    let deadBlobs = Prim.getDeadBlobs();
    switch deadBlobs {
      case (?deadBlobs) {
        assert (deadBlobs.size() == 1);
        assert (Prim.blobCompare(deadBlobs[0], "letmetestyou") == 0);
      };
      case null {};
    };

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
