//MOC-FLAG --stable-regions
import Prim "mo:⛔";

// A block-aligned load/store that ends exactly at the end of the region
// must not resolve a block past the region's last one.
actor {
  let pages : Nat64 = 256; // two 8 MiB blocks of 128 pages each
  let size = 16 * 1024 * 1024;
  let lastOfBlock1 : Nat64 = 8 * 1024 * 1024 - 8;
  let lastOfBlock2 : Nat64 = 16 * 1024 * 1024 - 8;

  public func go() : async () {
    let r = Prim.regionNew();
    assert Prim.regionGrow(r, pages) == 0;
    assert Prim.regionSize(r) == pages;
    Prim.regionStoreNat64(r, lastOfBlock1, 0x1111_1111_1111_1111);
    Prim.regionStoreNat64(r, lastOfBlock2, 0x2222_2222_2222_2222);

    let blob = Prim.regionLoadBlob(r, 0, size);
    assert blob.size() == size;

    let r2 = Prim.regionNew();
    assert Prim.regionGrow(r2, pages) == 0;
    Prim.regionStoreBlob(r2, 0, blob);
    assert Prim.regionLoadNat64(r2, 0) == 0;
    assert Prim.regionLoadNat64(r2, lastOfBlock1) == 0x1111_1111_1111_1111;
    assert Prim.regionLoadNat64(r2, lastOfBlock2) == 0x2222_2222_2222_2222;
    assert Prim.regionLoadBlob(r2, 0, size) == blob;
    Prim.debugPrint("ok");
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
