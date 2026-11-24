//MOC-FLAG --stable-regions --max-region-blocks 4
import Prim "mo:⛔";

// All regions share one table of 8 MiB blocks (128 pages each). Once the
// total reaches the cap, a grow that needs another block must fail cleanly,
// while grows within already-allocated blocks still succeed.
actor {
  let fail : Nat64 = 0xFFFF_FFFF_FFFF_FFFF;

  public func go() : async () {
    let r1 = Prim.regionNew();
    let r2 = Prim.regionNew();
    let r3 = Prim.regionNew();

    assert Prim.regionGrow(r1, 256) == 0; // blocks 1 and 2
    assert Prim.regionGrow(r2, 64) == 0;  // block 3, half full
    assert Prim.regionGrow(r3, 64) == 0;  // block 4, half full: cap reached
    Prim.regionStoreNat64(r1, 0, 0x1111_1111_1111_1111);
    Prim.regionStoreNat64(r3, 64 * 65536 - 8, 0x3333_3333_3333_3333);

    // within existing blocks: fine
    assert Prim.regionGrow(r2, 64) == 64;
    assert Prim.regionGrow(r1, 0) == 256;

    // anything needing a fifth block is refused, and nothing changes
    assert Prim.regionGrow(r1, 1) == fail;
    assert Prim.regionGrow(r2, 1) == fail;
    assert Prim.regionGrow(Prim.regionNew(), 1) == fail;
    assert Prim.regionSize(r1) == 256;
    assert Prim.regionSize(r2) == 128;
    assert Prim.regionSize(r3) == 64;

    // the last free pages of block 4 are still usable
    assert Prim.regionGrow(r3, 64) == 64;
    assert Prim.regionGrow(r3, 1) == fail;

    // existing data is intact
    assert Prim.regionLoadNat64(r1, 0) == 0x1111_1111_1111_1111;
    assert Prim.regionLoadNat64(r3, 64 * 65536 - 8) == 0x3333_3333_3333_3333;
    Prim.debugPrint("capped")
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
