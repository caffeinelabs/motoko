import Prim "mo:prim";

actor {

  let keepAlive : [var Blob] = Prim.Array_tabulateVar<Blob>(100, func(i : Nat) : Blob = "");
  var counter = 0;

  public func test(b : Blob) : async () {
    keepAlive[counter] := b;
    counter += 1;
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
    var counter = 20;
    while (counter > 0) {
      await test(blobArr[0]);
      await test(blobArr[1]);
      await test(blobArr[2]);
      await test(blobArr[3]);
      await test(blobArr[4]);
      counter -= 1;
    };

    let hash = Prim.__getDedupTable();
    switch hash {
      case (?hashArray) {
        Prim.debugPrint(debug_show (getHashArrayLen(hashArray)));
        assert (getHashArrayLen(hashArray) == 3);
      };
      case null {};
    };
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
