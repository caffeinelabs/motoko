import Nat32 "mo:core/Nat32";
import Nat64 "mo:core/Nat64";
import Text "mo:core/Text";
import Array "mo:core/Array";
import VarArray "mo:core/VarArray";

import StableMemory "mo:base/ExperimentalStableMemory"; // deprecated

persistent actor StableLog {

  func ensure(offset : Nat64) {
    let pages = (offset + 65536) >> 16;
    if (pages > StableMemory.size()) {
      let oldsize = StableMemory.grow(pages - StableMemory.size());
      assert (oldsize != 0xFFFF_FFFF_FFFF_FFFF);
    };
  };

  var base : Nat64 = 0; // implicitly `stable`

  public func log(t : Text) {
    let blob = Text.encodeUtf8(t);
    let size = Nat64.fromNat(blob.size());
    ensure(base + size + 4);
    StableMemory.storeBlob(base, blob);
    base += size;
    StableMemory.storeNat32(base, Nat32.fromNat(blob.size()));
    base += 4;
  };

  public query func readLast(count : Nat) : async [Text] {
    let a = VarArray.repeat<Text>("", count);
    var offset = base;
    var k = 0;
    while (k < count and offset > 0) {
      offset -= 4;
      let size = StableMemory.loadNat32(offset);
      offset -= Nat64.fromNat(Nat32.toNat(size));
      let blob = StableMemory.loadBlob(offset, Nat32.toNat(size));
      switch (Text.decodeUtf8(blob)) {
        case (?t) { a[k] := t };
        case null { assert false };
      };
      k += 1;
    };
    return Array.tabulate<Text>(k, func i { a[i] });
  };

};

