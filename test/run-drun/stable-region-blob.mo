//MOC-FLAG --stable-regions

import P "mo:⛔";
import Region "stable-region/Region";
actor {

  stable var n : Nat64 = 0;

  stable let r = Region.new();

  assert (n == Region.size(r));

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
    Region.storeBlob(r, i, v);
    assert (Region.loadBlob(r, i, P.nat64ToNat(size)) == v);
    i += size;
    size += 1;
  };


  system func preupgrade() {
    P.debugPrint("upgrading..." # debug_show n);
    let m = Region.grow(r, 1);

    assert (n == m);

    n += 1;

    P.debugPrint(debug_show {old = m; new = n; size = Region.size(r)});

    assert (n == Region.size(r));

    // check new page is clear
    var i : Nat64 = m * 65536;
    var size : Nat64 = 0;
    let max = i + 65536;
    while (i + size < max) {
      assert (Region.loadBlob(r, i, P.nat64ToNat(size)) == zeroOfNat64(size));
      Region.storeBlob(r, i, valOfNat64(size));
      i += size;
      size += 1;
    };
  };

  public func testBounds() : async () {
    if (n == 0) return;
    assert (n == Region.size(r));
    P.debugPrint (debug_show {testBounds=n});
    // test bounds check
    var i : Nat64 = n * 65536 - 7;
    let max = i + 16;
    while (i < max) {
      try {
        await async {
          ignore Region.loadBlob(r, i, 8);
        };
        assert false;
      }
      catch e {
        assert P.errorCode e == #canister_error;
      };
      try {
        await async Region.storeBlob(r, i, valOfNat64(8));
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
