// allocate 2 big arrays with a 10^5 entries each, and populate them
// with Int32s and Nat32s (somewhat randomly)
import {
  Array_tabulate;
  performanceCounter;
  intToNat32Wrap;
  intToInt32Wrap;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";

actor Tagged {

  func arrNat32(seed : Nat) : ([Nat32], Nat) {
    var state = seed;
    let cutoff = 2_147_483_629;
    var taggable = 0;
    (
      Array_tabulate<Nat32>(
        100_000,
        func _ {
          state := (state + 17) * 57 % cutoff;
          let wrapped = intToNat32Wrap state;
          let bits = (wrapped >> 30) & 3;
          if bits == 0 or bits == 3 { taggable += 1 };
          wrapped;
        },
      ),
      taggable,
    );
  };

  func arrInt32(seed : Nat) : ([Int32], Nat) {
    var state = seed;
    let cutoff = 4294967311;
    var taggable = 0;
    (
      Array_tabulate<Int32>(
        100_000,
        func _ {
          state := (state + 17) * 57 % cutoff;
          let wrapped = intToInt32Wrap state;
          let bits = (wrapped >> 30) & 3;
          if bits == 0 or bits == 3 { taggable += 1 };
          wrapped;
        },
      ),
      taggable,
    );
  };

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    let (m0, n0) = counters();
    let (_, nt) = arrNat32(7);
    let (m1, n1) = counters();
    debugPrint(debug_show (nt, m1 - m0, n1 - n0));

    let (i0, j0) = counters();
    let (_, it) = arrInt32(13);
    let (i1, j1) = counters();
    debugPrint(debug_show (it, i1 - i0, j1 - j0));
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
