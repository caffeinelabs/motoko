import Prim "mo:prim";

actor {

  var arr = Prim.Array_init<Nat>(13, 0);

  public func test() : async () {

    let wr1 = Prim.allocWeakRef(arr);
    let isLive1 = Prim.isLive(wr1);
    let deref1 = Prim.weakGet(wr1);

    Prim.debugPrint(debug_show wr1);
    Prim.debugPrint(debug_show isLive1);
    Prim.debugPrint(debug_show deref1);

    // Trigger GC by yielding to the scheduler.
    // The weak ref isLive should be false now and the deref should be null.
    var n = 10;
    while (n > 0) {
      n -= 1;
      // Allocate large array.
      arr := Prim.Array_init<Nat>(1_000, 1);
      await async {};
      let isLive1 = Prim.isLive(wr1);
      let deref1 = Prim.weakGet(wr1);
      Prim.debugPrint(debug_show isLive1);
      Prim.debugPrint(debug_show deref1);
    };

  };

  public func test2() : async () {
    var blob1 = Prim.Array_init<Nat64>(1, 1);
    var blob2 = Prim.Array_init<Nat64>(1, 2);
    var blob3 = Prim.Array_init<Nat64>(1, 3);

    var wrs = [Prim.allocWeakRef(blob1), Prim.allocWeakRef(blob2), Prim.allocWeakRef(blob3)];

    var idx = 0;
    while (idx < 5) {

      Prim.debugPrint(debug_show ("================"));
      for (wr in wrs.vals()) {
        Prim.debugPrint(debug_show (Prim.isLive(wr)));
      };

      blob3 := Prim.Array_init<Nat64>(1, 10);
      blob2 := Prim.Array_init<Nat64>(10, 13);
      blob1 := blob1;

      idx += 1;
      Prim.debugPrint(debug_show ("================"));

      await async {};
    };

  };

  public func test3() : async () {
    var blobs = [var Prim.Array_init<Nat64>(3, 1), Prim.Array_init<Nat64>(10, 2), Prim.Array_init<Nat64>(1, 3)];
    var wrs = [Prim.allocWeakRef(blobs[0]), Prim.allocWeakRef(blobs[1]), Prim.allocWeakRef(blobs[2])];

    var idx = 0;
    while (idx < 5) {

      Prim.debugPrint(debug_show ("================"));

      for (wr in wrs.vals()) {
        let val = Prim.weakGet(wr);
        Prim.debugPrint(debug_show (val));
        Prim.debugPrint(debug_show (Prim.isLive(wr)));
      };

      blobs[0] := Prim.Array_init<Nat64>(1, 13);

      idx += 1;
      Prim.debugPrint(debug_show ("================"));

      await async {}; // trigger GC.
    };

  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY

//CALL ingress test3 "DIDL\x00\x00"
