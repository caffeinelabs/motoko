import P "mo:⛔";
import Region "../stable-region/Region";

actor {

    // measure out three blocks' worth of bytes.
    let pageInBytes = 1 << 16 : Nat64;
    let blockInBytes = pageInBytes * 128 : Nat64;
    let size = blockInBytes * 3 : Nat64;
    var i = 0 : Nat64;
    var b = 0 : Nat8;

    // Check size for necessary number of pages.
    stable let r = Region.new();

    let reqPages = size / pageInBytes;

    P.debugPrint("reqPages = " # (debug_show reqPages));
    P.debugPrint("Region.size(r) = " # (debug_show Region.size(r)));

    assert Region.size(r) == reqPages;

    // Load out previously-stored byte pattern in a defined interval.
    // The interval serves for faster test runs on the CI, to avoid `drun` batch limit.
    // Check each byte is what we would have written, if we were repeating the same logic again.
    while (i < size) {
        let expected = b;
        let loaded = Region.loadNat8(r, i);
        //P.debugPrint(" - " # (debug_show {i; expected; loaded}));
        assert loaded == expected;
        i := i + 10;
        b := b +% 1;
    };

    P.debugPrint ("actor1: checked region.");

    stable var r1 = Region.new();

    P.debugPrint ("actor1: allocated another region.");
}
