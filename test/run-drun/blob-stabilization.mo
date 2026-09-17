import Prim "mo:prim";

actor {
    let blobSize = 32 * 1024 * 1024;
    // 2^25 bytes as a rope of shared halves, only materialized by `encodeUtf8`.
    var text = "x";
    var doublings = 25;
    while (doublings > 0) {
        text := text # text;
        doublings -= 1;
    };

    stable let blob = Prim.encodeUtf8(text);
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
