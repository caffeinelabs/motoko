//SKIP run
//SKIP run-ir
//SKIP run-low

import {
  Array_tabulate;
  rts_heap_size;
  debugPrint;
} = "mo:⛔";

actor {
  public func go() : async () {
    var x : Float32 = 3.14;
    let m0 : Int = rts_heap_size();
    let arr = Array_tabulate<Float32>(128 * 1024, func _ {
      let v = x;
      x += 1.0;
      v
    });
    let m1 : Int = rts_heap_size();
    // 128 * 1024 elements × 8 bytes (bit-tagged Vanilla i64) + 24 bytes (array header) ≈ 1 MiB
    debugPrint(debug_show { elements = arr.size(); bytes = m1 - m0 });
    assert (m1 - m0 >= 1024 * 1024);
    assert (m1 - m0 < 1024 * 1024 + 256);
  };
};

//CALL ingress go 0x4449444C0000
