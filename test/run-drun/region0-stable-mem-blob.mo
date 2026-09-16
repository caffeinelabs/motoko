//MOC-FLAG --stable-regions

import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";
actor {

  stable var n : Nat64 = 0;
  assert (n == StableMemory.size());

  func valOfNat64(n : Nat64) : Blob {
    let size = P.nat64ToNat(n);
    let a = P.Array_tabulate<Nat8>(size, func i { P.natToNat8(i % 256) });
    P.arrayToBlob(a);
  };

  func zeroOfNat64(n : Nat64) : Blob {
    let size = P.nat64ToNat(n);
    let a = P.Array_tabulate<Nat8>(size, func i { 0 });
    P.arrayToBlob(a);
  };

  var i : Nat64 = 0;
  var size : Nat64 = 0;
  let max = n * 65536;
  while (i + size < max) {
    let v = valOfNat64(size);
    //P.debugPrint("store blob, load blob.");
    StableMemory.storeBlob(i, v);
    assert (StableMemory.loadBlob(i, P.nat64ToNat(size)) == v);
    i += size;
    size += 1;
  };

  system func preupgrade() {
    P.debugPrint("upgrading..." # debug_show n);
    let m = StableMemory.grow(1);

    assert (n == m);

    n += 1;

    P.debugPrint(debug_show {old = m; new = n; size = StableMemory.size()});

    assert (n == StableMemory.size());

    // check new page is clear
    var i : Nat64 = m * 65536;
    var size : Nat64 = 0;
    let max = i + 65536;
    while (i + size < max) {
      if false {
      P.debugPrint("i = " # debug_show i # "; size = " # debug_show size);
      P.debugPrint("inspecting the blob at offset " # debug_show i # " and expecting all zeros...");
      P.debugPrint(debug_show StableMemory.loadBlob(i, P.nat64ToNat(size)));
      };

      assert (StableMemory.loadBlob(i, P.nat64ToNat(size)) == zeroOfNat64(size));
      let blob = valOfNat64(size);
      // P.debugPrint("storing the blob " # debug_show blob # " at offset " # debug_show i);

      StableMemory.storeBlob(i, blob);
      i += size;
      size += 1;
    };

  };

  public func testBounds() : async () {
    if (n == 0) return;
    assert (n == StableMemory.size());
    P.debugPrint (debug_show {testBounds=n});
    // test bounds check
    var i : Nat64 = n * 65536 - 7;
    let max = i + 16;
    while (i < max) {
      try {
        await async {
          ignore StableMemory.loadBlob(i, 8);
        };
        P.debugPrint(debug_show{loadBlob = (i,8); size = n });
        assert false;
      }
      catch e {
        assert P.errorCode e == #canister_error;
      };
      try {
        await async StableMemory.storeBlob(i, valOfNat64(8));
        assert false;
      }
      catch e {
        assert P.errorCode e == #canister_error;
      };
      i += 1;
    };
  };

  system func postupgrade() {
    P.debugPrint("...upgraded" # debug_show n);
  };

}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL upgrade ""
//CALL ingress testBounds "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress testBounds "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress testBounds "DIDL\x00\x00"
//CALL upgrade ""

//MOC-FLAG -A=M0270
