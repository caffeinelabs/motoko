import P "mo:⛔";
import Region "stable-region/Region";

// The Region analogue of the deleted `regions-pay-as-you-go` test: the RTS
// accounts for region pages lazily ("pay as you go"), so growing a region
// moves the physical size only when the logical size demands it.
actor {

  P.debugPrint("=============");
  P.debugPrint ("do nothing");
  let s1 = P.rts_stable_memory_size();
  let l1 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s1;l1}));
  P.debugPrint ("Region.grow(r1,1)");
  let r1 = Region.new();
  let _ = Region.grow(r1, 1);
  let s2 = P.rts_stable_memory_size();
  let l2 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s2;l2;r1 = Region.id(r1)}));
  P.debugPrint ("Region.new()");
  let r2 = Region.new();
  let s3 = P.rts_stable_memory_size();
  let l3 = P.rts_logical_stable_memory_size();
  P.debugPrint (debug_show({s3;l3;r1 = Region.id(r1); r2 = Region.id(r2)}));
  P.debugPrint ("Region.grow(r2,1)");
  let _ = Region.grow(r2, 1);
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