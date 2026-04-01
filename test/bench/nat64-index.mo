// Benchmark: NatN.toNat() peephole vs plain Nat array indexing
//MOC-FLAG --package core $MOTOKO_CORE
import Nat64 "mo:core/Nat64";
import Nat32 "mo:core/Nat32";
import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
  Array_init;
  natToNat32;
  natToNat64;
  nat32ToNat;
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

  // Prim: nat32ToNat()
  public func nat32PrimIndex() : async () {
    let (m0, n0) = counters();
    let arrSize32 : Nat32 = natToNat32(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat32 = 0;
      while (n < arrSize32) {
        acc +%= arr[nat32ToNat(n)];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat32_prim_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Core lib: Nat32.toNat()
  public func nat32CoreIndex() : async () {
    let (m0, n0) = counters();
    let arrSize32 : Nat32 = natToNat32(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat32 = 0;
      while (n < arrSize32) {
        acc +%= arr[Nat32.toNat(n)];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat32_core_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Method: n.toNat()
  public func nat32MethodIndex() : async () {
    let (m0, n0) = counters();
    let arrSize32 : Nat32 = natToNat32(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat32 = 0;
      while (n < arrSize32) {
        acc +%= arr[n.toNat()];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat32_method_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Prim: nat64ToNat()
  public func nat64PrimIndex() : async () {
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
    debugPrint("nat64_prim_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Core lib: Nat64.toNat()
  public func nat64CoreIndex() : async () {
    let (m0, n0) = counters();
    let arrSize64 : Nat64 = natToNat64(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat64 = 0;
      while (n < arrSize64) {
        acc +%= arr[Nat64.toNat(n)];
        n += 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat64_core_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Method: n.toNat()
  public func nat64MethodIndex() : async () {
    let (m0, n0) = counters();
    let arrSize64 : Nat64 = natToNat64(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat64 = 0;
      while (n < arrSize64) {
        acc +%= arr[n.toNat()];
        n += 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat64_method_index: " # debug_show (m1 - m0, n1 - n0));
  };
};

//CALL ingress setup 0x4449444C0000
//CALL ingress natIndex 0x4449444C0000
//CALL ingress nat32PrimIndex 0x4449444C0000
//CALL ingress nat32CoreIndex 0x4449444C0000
//CALL ingress nat32MethodIndex 0x4449444C0000
//CALL ingress nat64PrimIndex 0x4449444C0000
//CALL ingress nat64CoreIndex 0x4449444C0000
//CALL ingress nat64MethodIndex 0x4449444C0000
