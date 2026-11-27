import Prim "mo:⛔";

// A `timeout` on a raw call reaches the callee (#6551): the outer
// self-call into `@call_raw` must hand the deadline on to the real call.
actor A {
  public func probe() : async Bool { Prim.replyDeadline() != 0 };

  public func go() : async () {
    let typed = await (with timeout = 10) probe();
    let raw = await (with timeout = 10) Prim.call_raw(Prim.principalOfActor(A), "probe", to_candid(()));
    let plain = await Prim.call_raw(Prim.principalOfActor(A), "probe", to_candid(()));
    let ?viaRaw : ?Bool = from_candid(raw) else { Prim.trap("raw: no answer") };
    let ?viaPlain : ?Bool = from_candid(plain) else { Prim.trap("plain: no answer") };
    Prim.debugPrint(debug_show {typed; viaRaw; viaPlain})
  }
}

//CALL ingress go "DIDL\x00\x00"
