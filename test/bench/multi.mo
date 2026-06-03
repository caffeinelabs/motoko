// Benchmark: three multi-value-friendly algorithms from the literature.
// Designed to exercise the wasm multi-value codegen path
// (`--experimental-multi-value`) under load.
//
// * `fibPair` — canonical example from Andreas Rossberg's multi-value
//   proposal: single recursion that returns `(fib(n), fib(n+1))`,
//   collapsing the naive O(2^n) double recursion to O(n).
// * `egcd`   — Extended Euclidean / Bezout, returns the triple
//   `(gcd, x, y)` with `ax + by = gcd(a, b)`. 3-value recursion.
// * `leb128` — production-realistic divmod loop. Encodes
//   unsigned values to LEB128 via a helper that returns `(q, r)` from
//   one operation. Every IC canister hits this path on Candid encode.
//
// With multi-value off (default) each tuple-returning call round-trips
// the extra values through `multi_val_*` globals (FakeMultiVal). With
// `--experimental-multi-value` on, the values stay on the wasm stack
// across the function boundary.
import {
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";

persistent actor Multi {

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  // Returns (fib(n), fib(n+1)). Each recursive call yields both values
  // the parent needs, eliminating the naive `fib(n-1) + fib(n-2)`
  // double recursion.
  func fibPair(n : Nat) : (Nat, Nat) =
    if (n == 0) (0, 1)
    else {
      let (a, b) = fibPair (n - 1);
      (b, a + b)
    };

  // egcd(a, b) returns (g, x, y) with a*x + b*y = g = gcd(a, b).
  func egcd(a : Int, b : Int) : (Int, Int, Int) =
    if (b == 0) (a, 1, 0)
    else {
      let (g, x1, y1) = egcd(b, a % b);
      (g, y1, x1 - (a / b) * y1)
    };

  // The codegen-relevant primitive: one helper, two outputs. Every
  // LEB128 byte requires both q and r from the same n / 128 step.
  func divmod128(n : Nat64) : (Nat64, Nat64) = (n / 128, n % 128);

  // Counts how many LEB128 bytes `v` would encode to. Hot loop: one
  // `divmod128` per byte, sum stays in registers.
  func leb128Length(v0 : Nat64) : Nat {
    var v = v0;
    var len = 1;
    loop {
      let (q, _r) = divmod128(v);
      if (q == 0) return len;
      v := q;
      len += 1;
    }
  };

  public func fibBench() : async () {
    let (m0, n0) = counters();
    var sink : Nat = 0;
    var i = 0;
    while (i < 1_000) {
      let (a, _) = fibPair(40);
      sink += a;
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show { fib = sink; heap = m1 - m0; cycles = n1 - n0 });
  };

  public func egcdBench() : async () {
    let (m0, n0) = counters();
    var sink : Int = 0;
    var i = 0;
    while (i < 10_000) {
      // Consecutive Fibonacci numbers — worst case for Euclidean depth.
      let (g, x, y) = egcd(46368, 75025);
      sink += g + x + y;
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show { egcd_check = sink; heap = m1 - m0; cycles = n1 - n0 });
  };

  public func lebBench() : async () {
    let (m0, n0) = counters();
    var sink : Nat = 0;
    var i = 0;
    while (i < 100_000) {
      sink += leb128Length(0xDEAD_BEEF_CAFE_BABE);
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show { leb_len_sum = sink; heap = m1 - m0; cycles = n1 - n0 });
  };

  public func go() : async () {
    await fibBench();
    await egcdBench();
    await lebBench();
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
