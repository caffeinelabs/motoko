import Prim "mo:⛔";
import StableMemory "stable-mem/StableMemory";

actor {

  func ensure(offset : Nat64) {
    let pages = (offset + 65536) >> 16;
    if (pages > StableMemory.size()) {
      let oldsize = StableMemory.grow(pages - StableMemory.size());
      assert (oldsize != 0xFFFF_FFFF_FFFF_FFFF);
    };
  };

  stable var base : Nat64 = 0;

  public func log(t : Text) : () {
    let blob = Prim.encodeUtf8(t);
    let size = Prim.natToNat64(blob.size());
    ensure(base + size + 4);
    StableMemory.storeBlob(base, blob);
    base += size;
    StableMemory.storeNat32(base, Prim.natToNat32(blob.size()));
    base += 4;
  };

  public query func readLast(count : Nat) : async [Text] {
    let a = Prim.Array_init<Text>(count, "");
    var offset = base;
    var k = 0;
    while (k < count and offset > 0) {
      offset -= 4;
      let size = StableMemory.loadNat32(offset);
      offset -= Prim.natToNat64(Prim.nat32ToNat(size));
      let blob = StableMemory.loadBlob(offset, Prim.nat32ToNat(size));
      switch (Prim.decodeUtf8(blob)) {
        case (?t) { a[k] := t };
        case null { assert false };
      };
      k += 1;
    };
    return Prim.Array_tabulate<Text>(k, func i { a[i] });
  };


  // testing
  stable var count = 0;

  public func populate() : async () {
    // populate
    let limit = count + 10;
    while (count < limit) {
      log(debug_show(count));
      count += 1;
    };
  };

  public func readAll() : async () {
    let ts = await readLast(count);
    for (t in ts.values()) {
      Prim.debugPrint(t);
    };
  };

  public func readExtra() : async () {
    let ts = await readLast(2*count);
    for (t in ts.values()) {
      Prim.debugPrint(t);
    }
  }


}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress populate "DIDL\x00\x00"
//CALL ingress readAll "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress populate "DIDL\x00\x00"
//CALL ingress readAll "DIDL\x00\x00"
//CALL ingress readExtra "DIDL\x00\x00"
