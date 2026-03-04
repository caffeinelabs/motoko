// test small scalar ops
import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
  rts_lifetime_instructions;
} = "mo:⛔";

actor Tagged {

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    var i = 0;
    let (m0, n0) = counters();
    while (i < 16) {
      var n : Nat16 = 0;

      var temp : Nat16 = 0;
      while (n < 65535) {
        temp := n +% n;
        temp := n -% n;
        temp := ^n;
        temp := n *% n;
        n += 1;
      };
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show (m1 - m0, n1 - n0));
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
