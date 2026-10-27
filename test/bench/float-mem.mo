//SKIP run
//SKIP run-ir
//SKIP run-low

import {
  Array_tabulate;
  intToFloat;
  performanceCounter;
  rts_heap_size;
  debugPrint;
} = "mo:⛔";

actor {
  public func go() : async () {
    let m0 : Int = rts_heap_size();
    let c0 = performanceCounter(0);
    let arr = Array_tabulate<Float>(128 * 1024, func i = intToFloat i + 0.5);
    let c1 = performanceCounter(0);
    let m1 : Int = rts_heap_size();
    // every element is a boxed `Float`: 8 bytes (array slot) + box
    debugPrint(debug_show { elements = arr.size(); bytes = m1 - m0; instructions = c1 - c0 });
  };
};

//CALL ingress go 0x4449444C0000
