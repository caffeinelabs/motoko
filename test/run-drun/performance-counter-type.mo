import Prim "mo:⛔";

// `performanceCounter(1)` counts the instructions of the whole call context,
// across awaits, while `performanceCounter(0)` restarts with every message.
// The counter type used to be dropped on the way to the system API, so that
// both read counter 0.
actor {
  public func go() : async () {
    var i = 0;
    while (i < 100_000) { i += 1 };
    let before = Prim.performanceCounter(0);
    await async {};
    let message = Prim.performanceCounter(0);
    let context = Prim.performanceCounter(1);
    assert message < before;
    assert context > before;
    Prim.debugPrint("ok");
  };
};
//CALL ingress go 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low
