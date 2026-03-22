// Benchmark: Nat64 vs Nat array indexing
import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
  rts_lifetime_instructions;
  Array_init;
  natToNat64;
  nat64ToNat;
} = "mo:⛔";

persistent actor Nat64Index {

  transient let arrSize = 256;
  transient let arr : [var Nat64] = Array_init<Nat64>(arrSize, 0);

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func setup() : async () {
    var k = 0;
    while (k < arrSize) {
      arr[k] := natToNat64(k * 0x12345);
      k += 1;
    };
  };

  // Baseline: Nat loop counter, Nat index
  public func natIndex() : async () {
    let (m0, n0) = counters();
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n = 0;
      while (n < arrSize) {
        acc +%= arr[n];
        n += 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // New: Nat64 loop counter, Nat64 index (no conversion)
  public func nat64Index() : async () {
    let (m0, n0) = counters();
    let arrSize64 : Nat64 = natToNat64(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat64 = 0;
      while (n < arrSize64) {
        acc +%= arr[n];
        n += 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat64_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Old workaround: Nat64 loop counter, toNat() for index
  public func nat64ToNatIndex() : async () {
    let (m0, n0) = counters();
    let arrSize64 : Nat64 = natToNat64(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat64 = 0;
      while (n < arrSize64) {
        acc +%= arr[nat64ToNat(n)];
        n += 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat64_toNat_index: " # debug_show (m1 - m0, n1 - n0));
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress setup 0x4449444C0000
//CALL ingress natIndex 0x4449444C0000
//CALL ingress nat64Index 0x4449444C0000
//CALL ingress nat64ToNatIndex 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
