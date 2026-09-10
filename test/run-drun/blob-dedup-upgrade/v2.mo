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

  let blobArr : [Blob] = [
    "a",
    "!caf!hello",
    "!caf!world",
    "acaf!hello",
    "!caf!letmetestyou",
  ];

  public func test2() : async () {

    let hash = Prim.__getDedupTable();
    switch hash {
      case (?hashArray) {
        Prim.debugPrint(debug_show (getHashArrayLen(hashArray)));
        assert (getHashArrayLen(hashArray) == 3);
      };
      case null {};
    };

    let blob0 : Blob = "a";
    let blob1 : Blob = "!caf!caffeinerules";
    let blob2 : Blob = "!caf!chocolaterules";
    let blob3 : Blob = "!caf!coffeerules";
    var counter = 20;
    while (counter > 0) {
      await test(blob0);
      await test(blob1);
      await test(blob2);
      await test(blob3);
      counter -= 1;
    };

    let newHash = Prim.__getDedupTable();
    switch newHash {
      case (?hashArray) {
        Prim.debugPrint(debug_show (getHashArrayLen(hashArray)));
        assert (getHashArrayLen(hashArray) == 6);
      };
      case null {};
    };
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
