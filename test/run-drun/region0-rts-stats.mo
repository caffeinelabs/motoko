//MOC-FLAG --stable-regions -A=M0194
import P "mo:⛔";
import StableMemory "stable-mem/StableMemory";
actor {
  let s1 = P.rts_stable_memory_size();
  let l1 = P.rts_logical_stable_memory_size();
  assert (s1 == 6 or s1 == 145 or s1 == 146);
  assert (l1 == 6 or l1 == 144);
  P.debugPrint ("Ignore Diff: "# debug_show({s1;l1}));
  let 0 = StableMemory.grow(16);
  stable var v = StableMemory.loadBlob(0, 65536);
  let s2 = P.rts_stable_memory_size();
  ();
  let l2 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s2;l2}));
}

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL upgrade ""
//CALL upgrade ""
//CALL upgrade ""