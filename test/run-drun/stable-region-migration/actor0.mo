import P "mo:⛔";
import Region "../stable-region/Region";

actor {

    // measure out three blocks' worth of bytes.
    let pageInBytes = 1 << 16 : Nat64;
    let blockInBytes = pageInBytes * 128 : Nat64;
    let size = blockInBytes * 3 : Nat64;
    var i = 0 : Nat64;
    var b = 0 : Nat8;

    // Grow to the necessary number of pages.
    let reqPages = size / pageInBytes;

    P.debugPrint("reqPages = " # (debug_show reqPages));

    stable let r = Region.new();
    assert Region.grow(r, reqPages) == 0;
    assert Region.size(r) == reqPages;

    // write byte pattern, in a defined interval.
    while (i < size) {
        Region.storeNat8(r, i, b);
        i := i + 10;
        b := b +% 1;
    };

    P.debugPrint ("actor0: init'ed.");
}
