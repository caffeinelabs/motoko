//MOC-FLAG --stable-regions
import Prim "mo:⛔";

// A block-aligned load/store that ends exactly at the end of the region
// must not resolve a block past the region's last one.
actor {
  let pages : Nat64 = 256; // two 8 MiB blocks of 128 pages each
  let size = 16 * 1024 * 1024;
  let lastOfBlock1 : Nat64 = 8 * 1024 * 1024 - 8;
  let lastOfBlock2 : Nat64 = 16 * 1024 * 1024 - 8;

  // Four blocks: the access vector is then 4 * sizeof(u16) = 8 bytes, a whole
  // number of words, so the blob has no trailing padding in which a stray read
  // could land. The range below covers the last two blocks and again ends
  // exactly at the region's end.
  let pages4 : Nat64 = 512;
  let block3 : Nat64 = 16 * 1024 * 1024;
  let lastOfBlock4 : Nat64 = 32 * 1024 * 1024 - 8;

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

    let r3 = Prim.regionNew();
    assert Prim.regionGrow(r3, pages4) == 0;
    assert Prim.regionSize(r3) == pages4;
    Prim.regionStoreNat64(r3, lastOfBlock4, 0x3333_3333_3333_3333);

    let tail = Prim.regionLoadBlob(r3, block3, size);
    assert tail.size() == size;
    Prim.regionStoreBlob(r3, block3, tail);
    assert Prim.regionLoadNat64(r3, lastOfBlock4) == 0x3333_3333_3333_3333;
    assert Prim.regionLoadBlob(r3, block3, size) == tail;

    Prim.debugPrint("ok");
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
