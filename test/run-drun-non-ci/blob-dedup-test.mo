import Prim "mo:prim";

actor {

  let keepAlive : [var Blob] = Prim.Array_tabulateVar<Blob>(50_000, func(i : Nat) : Blob = "");
  var counter = 0;

  public func test(b : Blob) : async () {
    if (counter % 5 != 0) {
      // Don't keep alive every 5th blob.
      keepAlive[counter] := b;
    };
    counter += 1;
  };

  public func test2() : async () {

    // The test deduplicates 50K blobs but keeps alive only 40K.
    // So there should be 10K dead blobs.
    // Then we confirm deletion (prune) 5 of them.
    // In the end there should be 9_995 dead blobs.

    var i : Nat16 = 0;
    let magicBytes : [Nat8] = [0x21, 0x63, 0x61, 0x66, 0x21];
    while (i < 50_000) {
      // Create an array with the magic bytes and i.
      // i is a Nat16 so we need to convert it to 2 Nat8.add
      let (i8_1, i8_2) = Prim.explodeNat16(i);
      // Prim.debugPrint(debug_show (i8_1));
      // Prim.debugPrint(debug_show (i8_2));
      let concat = Prim.Array_tabulate(magicBytes.size() + 2, func(j : Nat) : Nat8 = if (j < magicBytes.size()) { magicBytes[j] } else { if (j == magicBytes.size()) { i8_1 } else { i8_2 } });
      let b = Prim.arrayToBlob(concat);
      await test(b);

      i += 1;
    };

    var n = 30;
    // try to trigger GC.
    while (n > 0) {
      // Allocate large array.
      let _arr = Prim.Array_init<Nat>(1_000 * 1_000, 1);
      await async {};
      n -= 1;
    };

    let x = Prim.getDeadBlobs();
    switch x {
      case (?deadBlobs) {
        Prim.debugPrint(debug_show (deadBlobs.size()));
        assert (deadBlobs.size() == 10_000);

        Prim.pruneConfirmedDeadBlobs([deadBlobs[0], deadBlobs[1], deadBlobs[2], deadBlobs[3], deadBlobs[4]]);

        let x2 = Prim.getDeadBlobs();
        switch x2 {
          case (?deadBlobs2) {
            Prim.debugPrint(debug_show (deadBlobs2.size()));
            assert (deadBlobs2.size() == 9_995);
          };
          case null {};
        };
      };
      case null {};
    };

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-NO-FORCE-GC

//CALL ingress test2 "DIDL\x00\x00"
