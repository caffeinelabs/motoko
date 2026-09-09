import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";
import Region "stable-region/Region";
actor {
  P.debugPrint("=============");
  P.debugPrint ("do nothing");
  let s1 = P.rts_stable_memory_size();
  let l1 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s1;l1}));
  P.debugPrint ("SM.grow(1)");
  transient let _ = StableMemory.grow(1);
  let s2 = P.rts_stable_memory_size();
  let l2 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s2;l2}));
  P.debugPrint ("let r1=Region.new()");
  stable let r1 = Region.new();
  let s3 = P.rts_stable_memory_size();
  let l3 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s3;l3;r1 = Region.id(r1)}));
  P.debugPrint ("let _ = Region.grow(r1,1)");
  transient let _ = Region.grow(r1, 1);
  let s4 = P.rts_stable_memory_size();
  let l4 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s4;l4;r1 = Region.id(r1)}));
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL upgrade ""
