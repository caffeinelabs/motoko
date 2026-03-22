// Benchmark: fixed-width Nat vs Nat array indexing
import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
  rts_lifetime_instructions;
  Array_init;
  natToNat8;
  natToNat16;
  natToNat32;
  natToNat64;
  nat64ToNat;
} = "mo:⛔";

persistent actor NatXIndex {

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

  // Nat8 loop counter, Nat8 index
  public func nat8Index() : async () {
    let (m0, n0) = counters();
    let arrSize8 : Nat8 = natToNat8(arrSize - 1);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat8 = 0;
      while (n < arrSize8) {
        acc +%= arr[n];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat8_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat16 loop counter, Nat16 index
  public func nat16Index() : async () {
    let (m0, n0) = counters();
    let arrSize16 : Nat16 = natToNat16(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat16 = 0;
      while (n < arrSize16) {
        acc +%= arr[n];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat16_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat32 loop counter, Nat32 index
  public func nat32Index() : async () {
    let (m0, n0) = counters();
    let arrSize32 : Nat32 = natToNat32(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat32 = 0;
      while (n < arrSize32) {
        acc +%= arr[n];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat32_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat64 loop counter, Nat64 index
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
//CALL ingress nat8Index 0x4449444C0000
//CALL ingress nat16Index 0x4449444C0000
//CALL ingress nat32Index 0x4449444C0000
//CALL ingress nat64Index 0x4449444C0000
//CALL ingress nat64ToNatIndex 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
