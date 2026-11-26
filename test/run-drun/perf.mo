import Prim "mo:⛔";

actor {

  public func go() : async () {
     let c1 = Prim.performanceCounter(0);
     var n = 1000;
     while (n > 0) {
        n-=1;
     };
     let c2 = Prim.performanceCounter(0);
     // Prim.debugPrint(debug_show(c2 - c1)); // too noisy for --sanity-checks
     assert (c2 >= c1);
  };

  // Counter 0 restarts with each message; counter 1 counts the whole call
  // context, across awaits.
  public func callContext() : async () {
    var i = 0;
    while i < 100_000 { i += 1 };
    let before = Prim.performanceCounter(0);
    await async {};
    let message = Prim.performanceCounter(0);
    let context = Prim.performanceCounter(1);
    assert message < before;
    assert context > before
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
//CALL ingress callContext "DIDL\x00\x00"
