import Prim "mo:prim";

actor {
    let blobSize = 32 * 1024 * 1024;
    let stablePageSize = 64 * 1024;
    ignore Prim.stableMemoryGrow(Prim.natToNat64(blobSize / stablePageSize));

    stable let blob = Prim.stableMemoryLoadBlob(0, blobSize);
    stable let small = (123_456_789_123_456_789, "TEST");

    public query func check() : async () {
        assert (blob.size() == blobSize);
        assert (small == (123_456_789_123_456_789, "TEST"));
    };
};
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress check "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade
//CALL ingress check "DIDL\x00\x00"

