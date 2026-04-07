//MOC-FLAG --force-gc --package core $MOTOKO_CORE
import {
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";
import Nat "mo:core/Nat"

actor _alloc {

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    let (m0, n0) = counters();
    for (i in Nat.range(0, 1024)) {
      assert i < 1024
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
