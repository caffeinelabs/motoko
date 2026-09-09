import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";
import Region "stable-region/Region";
actor {
  P.debugPrint("=============");
  P.debugPrint ("do nothing");
  let s1 = P.rts_stable_memory_size();
  let l1 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s1;l1}));
  P.debugPrint ("SM.grow(16)");
  transient let _ = StableMemory.grow(16);
  let s2 = P.rts_stable_memory_size();
  let l2 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s2;l2}));
  P.debugPrint ("Region.new()");
  let r1 = Region.new();
  let s3 = P.rts_stable_memory_size();
  let l3 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s3;l3;r1 = Region.id(r1)}));
  P.debugPrint ("Region.new()");
  stable let r2 = Region.new();
  let s4 = P.rts_stable_memory_size();
  let l4 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s4;l4;r1 = Region.id(r1); r2 = Region.id(r2)}));


}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL upgrade ""
//CALL upgrade ""
//CALL upgrade ""
