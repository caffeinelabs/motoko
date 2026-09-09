import Prim "mo:prim";

actor {

  let keepAlive : [var Blob] = Prim.Array_tabulateVar<Blob>(100, func(i : Nat) : Blob = "");
  var counter = 0;

  public func test(b : Blob) : async () {
    // No more appending to the keepAlive array.
    let a = b;
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
  func getLiveBlobs(hashArray : [var List]) : Nat {
    var len = 0;
    var i = 0;
    while (i < 16_384) {
      var list = hashArray[i];
      label countLoop loop {
        let weakRef = list.value;
        switch weakRef {
          case (?weakRef) {
            let deref = Prim.weakGet(weakRef.ref);
            switch deref {
              case (?deref) { len += 1 };
              case null {};
            };
          };
          case null {};
        };
        let next = list.next;
        switch next {
          case (?next) { list := next };
          case null { break countLoop };
        };
      };
      i += 1;
    };
    len;
  };
  func showMeAllBlobs(hashArray : [var List]) : () {
    var i = 0;
    while (i < 16_384) {
      // Deref the weak ref.
      let weakRef = hashArray[i].value;
      switch weakRef {
        case (?weakRef) {
          Prim.debugPrint(debug_show (hashArray[i].originalBlob));
          Prim.debugPrint(debug_show ("============"));
          Prim.debugPrint(debug_show (weakRef.ref));
        };
        case null {};
      };
      i += 1;
    };
  };

  public func test2() : async () {

    let hash = Prim.__getDedupTable();
    switch hash {
      case (?hashArray) {
        Prim.debugPrint(debug_show (getHashArrayLen(hashArray)));
        assert (getHashArrayLen(hashArray) == 6);
        //showMeAllBlobs(hashArray);
      };
      case null {};
    };

  };

  let blobArr : [Blob] = [
    "a",
    "!caf!hello",
    "!caf!world",
    "acaf!hello",
    "!caf!letmetestyou",
  ];

  public func test3() : async () {

    var n = 20;
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
        // The number of live blobs should be 3.
        // because only blobs from v1 are kept alive.
        // blobs from v2 are collected by the GC since there is nothing referencing them.
        Prim.debugPrint(debug_show (getLiveBlobs(hashArray)));
        assert (getLiveBlobs(hashArray) == 3);
      };
      case null {};
    };

    let deadBlobs = Prim.getDeadBlobs();
    switch deadBlobs {
      case (?deadBlobs) {
        assert (deadBlobs.size() == 3);
      };
      case null {};
    };

    assert (Prim.isStorageBlobLive("coffeerules") == false);
    assert (Prim.isStorageBlobLive("hello") == true);
    assert (Prim.isStorageBlobLive("world") == true);
    assert (Prim.isStorageBlobLive("letmetestyou") == true);
    assert (Prim.isStorageBlobLive("chocolaterules") == false);

    Prim.pruneConfirmedDeadBlobs(["coffeerules"]);

    let deadBlobs2 = Prim.getDeadBlobs();
    switch deadBlobs2 {
      case (?deadBlobs) {
        assert (deadBlobs.size() == 2);
      };
      case null {};
    };

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
