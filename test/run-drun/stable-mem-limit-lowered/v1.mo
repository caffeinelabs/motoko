//MOC-FLAG --max-stable-pages 400
import Prim "mo:⛔";

persistent actor {
  let r = Prim.regionNew();

  public func grow() : async () {
    assert Prim.regionGrow(r, 256) == 0;
    Prim.debugPrint(debug_show(Prim.regionSize(r)))
  }
}
