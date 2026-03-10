//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stable-regions
//MOC-ENV MOC_UNLOCK_PRIM=yesplease

import P "mo:⛔";
import Region "stable-region/Region";
import StableMemory "stable-mem/StableMemory";

actor {
  stable var n = 0;
  stable var r1 = Region.new();
  stable var r2 = Region.new();

  let block_size_in_pages = 128 : Nat64;

  P.debugPrint "grow three big regions (including region0).";
  // Interleave growing regions by a block each:
  do {
    ignore StableMemory.grow(block_size_in_pages);
    ignore Region.grow(r1, block_size_in_pages);
    ignore Region.grow(r2, block_size_in_pages);

    ignore StableMemory.grow(block_size_in_pages);
    ignore Region.grow(r1, block_size_in_pages);
    ignore Region.grow(r2, block_size_in_pages);

    ignore StableMemory.grow(block_size_in_pages);
    ignore Region.grow(r1, block_size_in_pages);
    ignore Region.grow(r2, block_size_in_pages);
  };
  P.debugPrint "grow three big regions: done.";

  system func preupgrade() {
    P.debugPrint("upgrading... n=" # debug_show n);
    n += 1;
  };
  public func sanityTest() : () {
    P.debugPrint("sanity check. n=" # debug_show n);
    assert Region.id(r1) == 16;
    assert Region.id(r2) == 17;
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
