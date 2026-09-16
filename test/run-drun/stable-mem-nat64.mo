import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";
actor {

  stable var n : Nat64 = 0;
  assert (n == StableMemory.size());

  func valOfNat64(n : Nat64) : Nat64 { P.natToNat64(P.nat64ToNat(n)); };
  let inc : Nat64 = 8;

  var i : Nat64 = 0;
  let max = n * 65536;
  while (i < max) {
    let v = valOfNat64(i);
    assert (StableMemory.loadNat64(i) == v);
    StableMemory.storeNat64(i, ^v);
    assert (StableMemory.loadNat64(i) == ^v);
    StableMemory.storeNat64(i, v);
    i += inc
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
    let max = i + 65536;
    while (i < max) {
      assert (StableMemory.loadNat64(i) == 0);
      StableMemory.storeNat64(i, valOfNat64(i));
      i += inc
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
          ignore StableMemory.loadNat64(i);
        };
        assert false;
      }
      catch e {
        assert P.errorCode e == #canister_error;
      };
      try {
        await async StableMemory.storeNat64(i, valOfNat64(i));
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
