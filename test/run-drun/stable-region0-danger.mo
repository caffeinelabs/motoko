//MOC-FLAG --stable-regions
// THIS TEST IS NOT RUN ON ANY FLAVOUR
// Its a counterexample to show what goes wrong if we naively support Region.region0() without
// extra steps to ensure correct aliasing.
import P "mo:⛔";
import Region "stable-region/Region";
import Region0 "stable-mem/StableMemory";

// counter-example: shows why we can't expose region0 without further steps to ensure
// correct aliasing of all region0 objects, pre and post upgrade
actor {

  stable let r0 = Region.region0(); // THIS WOULD BE DANGEROUS if permitted

  ignore Region.grow(r0, 1);

  system func preupgrade() {
  };

  public func sanityTest() : () {
    // test that r01 correctly aliases r0, even after upgrade.
    let r01 = Region.region0();
    P.debugPrint(debug_show {s0 = Region.size(r0);  s01 = Region.size(r01)});
    assert Region.size(r0) == Region.size(r01);
    ignore Region.grow(r0, 1);
    P.debugPrint(debug_show {s0 = Region.size(r0);  s01 = Region.size(r01)});
    assert Region.size(r0) == Region.size(r01);
  };
}

//SKIP tc
//SKIP run
//SKIP run-low
//SKIP run-ir
//SKIP run-drun
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
