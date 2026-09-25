//MOC-FLAG --force-gc
import {
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";

actor _alloc {

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    let (m0, n0) = counters();
    var i = 0;
    type List = ?((), List);
    var l : List = null;
    while i < 1024 * 1024 * 16 {
      l := ?((), l);
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show (m1 - m0, n1 - n0));
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};
//SKIP run-low
//SKIP run
//SKIP run-ir
//CALL ingress go 0x4449444C0000
//CALL ingress go 0x4449444C0000
//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
