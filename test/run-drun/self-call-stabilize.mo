//MOC-FLAG -A=M0194
import P "mo:⛔";
import Cycles = "cycles/cycles";
import Proxy "self-call-stabilize/proxy";

// A canister that forwards arbitrary calls to itself can be used to call
// `__motoko_stabilize_before_upgrade`, which used to accept self-calls. All
// further messages were then blocked until a controller upgraded it.
// The proxy is a child canister: unlike the test actor in drun, it is not
// its own controller.
actor a {
  public func run() : async () {
    if (Cycles.balance() == 0) {
      await Cycles.provisional_top_up_actor(a, 100_000_000_000_000);
    };
    Cycles.add<system>(2_000_000_000_000);
    let p = await Proxy.Proxy();

    await p.ping();
    await p.others();
    await p.ping();
    try { await p.freeze() } catch e { P.debugPrint(debug_show ("freeze", P.errorMessage e)) };
    try { await p.ping() } catch e { P.debugPrint(debug_show ("ping", P.errorMessage e)) };

    P.debugPrint("upgrade");
    ignore await (system Proxy.Proxy)(#upgrade p)();
    await p.ping();
  };
};

//CALL ingress run "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
