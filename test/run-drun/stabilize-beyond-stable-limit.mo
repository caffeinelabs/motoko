//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stabilization-instruction-limit=100000 --max-stable-pages 16

// This test fails because the stabilization code is using virtual stablemem_grow, which caps growth to max-stable-pages (default is 65536, but lowered below). It should be using physical ic0_stable64_grow (and _size) instead.

// Unfixed, the bug would prevent a user relying on defaults from serializing more than 4GB of heap data, even in 64-bit mode.

import Prim "mo:prim";

actor {

    let pages : Nat64 = 16;
    if (Prim.stableMemorySize() == 0) {
      Prim.debugPrint("growing stable memory");
      ignore Prim.stableMemoryGrow(pages);
    };
    assert Prim.stableMemorySize() == pages;
    stable let blob = Prim.stableMemoryLoadBlob(0,65536);

    public func check() : async () {
        Prim.debugPrint(debug_show (blob.size()))
    };

    system func preupgrade() {
        Prim.debugPrint("PRE-UPGRADE HOOK!");
    };

    system func postupgrade() {
        Prim.debugPrint("POST-UPGRADE HOOK!");
    };
};

//CALL ingress check "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress __motoko_destabilize_after_upgrade "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low

//MOC-FLAG -A=M0270
