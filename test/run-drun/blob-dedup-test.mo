import Prim "mo:prim";

actor {

  public func test(b : Blob) : async () {
    Prim.debugPrint(debug_show (b));
  };

  // WeakRef type.
  type WeakRef = {
    ref : weak Blob;
  };
  // A linked list of WeakRefs.
  type List = {
    var next : ?List;
    value : ?WeakRef;
    originalBlob : Blob;
    index : Nat;
  };
  func getHashArrayLen(hashArray : [var List]) : Nat {
    var len = 0;
    var i = 0;
    while (i < 16_384) {
      len += hashArray[i].index;
      i += 1;
    };
    len;
  };

  public func test2() : async () {
    let blob0 : Blob = "a";
    let blob1 : Blob = "!caf!hello";
    let blob2 : Blob = "!caf!world";
    let blob3 : Blob = "acaf!hello";
    let blob4 : Blob = "!caf!letmetestyou";
    await test(blob1);
    await test(blob2);
    await test(blob3);
    await test(blob1);
    await test(blob2);
    await test(blob3);
    var counter = 20;
    while (counter > 0) {
      await test(blob1);
      await test(blob2);
      await test(blob3);
      await test(blob0);
      await test(blob4);
      counter -= 1;
    };

    var n = 30;
    // try to trigger GC.
    while (n > 0) {
      // Allocate large array.
      let _arr = Prim.Array_init<Nat>(1_000 * 1_000, 1);
      await async {};
      n -= 1;
    };

    let hash = Prim.__getDedupTable();
    switch hash {
      case (?hashArray) {
        // The dedup table should still have 3 elements.
        assert (getHashArrayLen(hashArray) == 3);
      };
      case null {};
    };

    let x = Prim.getDeadBlobs();
    switch x {
      case (?deadBlobs) {
        assert (deadBlobs.size() == 3);
      };
      case null {};
    };

    assert (Prim.isStorageBlobLive(blob1) == false);

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-NO-FORCE-GC

//CALL ingress test2 "DIDL\x00\x00"
