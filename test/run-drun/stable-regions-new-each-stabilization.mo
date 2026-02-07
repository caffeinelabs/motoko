//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --stable-regions
//MOC-ENV MOC_UNLOCK_PRIM=yesplease

import P "mo:⛔";
import Region "stable-region/Region";

actor {
  stable var n = 0;
  stable let regions : [var ?Region] = [var null, null, null];
  
  system func preupgrade() {
    P.debugPrint("upgrading... calling Region.new(), n=" # debug_show n);
    regions[n] := ?Region.new();
    n += 1;
  };
  func unwrap(i : Nat, ro : ?Region) : Region {
      P.debugPrint(" unwrapping. i=" # debug_show i);      
      switch ro {
      case null { assert false; loop { } };
      case (?r) r;
      }
  };
  public func sanityTest() : () {
    P.debugPrint("sanity check. n=" # debug_show n);
    if (n > 0) { P.debugPrint(debug_show Region.id(unwrap(0, regions[0]))) };
    if (n > 1) { P.debugPrint(debug_show Region.id(unwrap(1, regions[1]))) };
    if (n > 2) { P.debugPrint(debug_show Region.id(unwrap(2, regions[2]))) };
  };
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""
//CALL ingress sanityTest "DIDL\x00\x00"
