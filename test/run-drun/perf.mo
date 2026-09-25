import Prim "mo:⛔";

actor {

  public func go() : async () {
    let c1 = Prim.performanceCounter(0);
    var n = 1000;
    while n > 0 {
      n-=1;
    };
    let c2 = Prim.performanceCounter(0);
    // Prim.debugPrint(debug_show(c2 - c1)); // too noisy for --sanity-checks
    assert (c2 >= c1);
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
