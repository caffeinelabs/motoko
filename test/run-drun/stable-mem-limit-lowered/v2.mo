//MOC-FLAG --max-stable-pages 200
import Prim "mo:⛔";

// Upgraded with a limit below the stable memory already in use.
persistent actor {
  let r = Prim.regionNew();

  public func grow() : async () {
    assert Prim.regionSize(r) == 256;
    assert Prim.regionGrow(r, 1) == 0xFFFF_FFFF_FFFF_FFFF;
    assert Prim.regionSize(r) == 256;
    Prim.debugPrint("refused")
  }
}
