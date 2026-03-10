//MOC-FLAG --stable-regions

import P "mo:⛔";
import Region "stable-region/Region";

actor {
  stable var r1 = Region.new();
  stable var aliases = [r1, r1];
  stable var id : Nat32 = 0xFFFF;
  stable var size : Nat64 = 0xFFFF_FFFF;

  system func preupgrade() {
    ignore Region.grow(r1, 8);
    assert Region.size(r1) == 8;
    assert Region.size(r1) == Region.size(aliases[0]);
    assert Region.size(r1) == Region.size(aliases[1]);
    P.debugPrint("pre");
    size := Region.size(r1);
    P.debugPrint(debug_show (#pre{size}));
  };

  system func postupgrade() {
    P.debugPrint(debug_show (#post{id;size;reg_size=Region.size(r1)}));
    assert Region.size(r1) == size;
    ignore Region.grow(r1, 8);
    assert Region.size(r1) == 16;
    assert Region.size(r1) == Region.size(aliases[0]);
    assert Region.size(r1) == Region.size(aliases[1]);
  };

  public func sanityTest() : () {
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
