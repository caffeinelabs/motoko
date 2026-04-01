// Benchmark: NatN.toNat() peephole vs plain Nat array indexing
import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
  Array_init;
  natToNat8;
  natToNat16;
  natToNat32;
  natToNat64;
  nat8ToNat;
  nat16ToNat;
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

  // Nat8 loop counter, nat8ToNat() for index
  public func nat8ToNatIndex() : async () {
    let (m0, n0) = counters();
    let arrSize8 : Nat8 = natToNat8(arrSize - 1);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat8 = 0;
      while (n < arrSize8) {
        acc +%= arr[nat8ToNat(n)];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat8_toNat_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat16 loop counter, nat16ToNat() for index
  public func nat16ToNatIndex() : async () {
    let (m0, n0) = counters();
    let arrSize16 : Nat16 = natToNat16(arrSize);
    var outer = 0;
    while (outer < 1000) {
      var acc : Nat64 = 0;
      var n : Nat16 = 0;
      while (n < arrSize16) {
        acc +%= arr[nat16ToNat(n)];
        n +%= 1;
      };
      outer += 1;
    };
    let (m1, n1) = counters();
    debugPrint("nat16_toNat_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat32 loop counter, nat32ToNat() for index
  public func nat32ToNatIndex() : async () {
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
    debugPrint("nat32_toNat_index: " # debug_show (m1 - m0, n1 - n0));
  };

  // Nat64 loop counter, nat64ToNat() for index
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
};

//CALL ingress setup 0x4449444C0000
//CALL ingress natIndex 0x4449444C0000
//CALL ingress nat8ToNatIndex 0x4449444C0000
//CALL ingress nat16ToNatIndex 0x4449444C0000
//CALL ingress nat32ToNatIndex 0x4449444C0000
//CALL ingress nat64ToNatIndex 0x4449444C0000
