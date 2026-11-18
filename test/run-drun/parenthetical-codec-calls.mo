import Prim "mo:⛔";

// Typed calls to methods with a codec send/expect Candid, which the codec
// doesn't speak: M0281 warns about them. Raw calls are fine.
actor A {
  func utf8(t : Text) : Blob = Prim.encodeUtf8(t);
  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  (with decoder = text)
  public func decoded(t : Text) : async Text { t };
  (with encoder = utf8)
  public func encoded(t : Text) : async Text { t };
  (with decoder = text; encoder = utf8)
  public func both(t : Text) : async Text { t };
  public func plain(t : Text) : async Text { t };

  // not called: these would fail at runtime
  public func typed() : async () {
    ignore await A.decoded("a"); // warns: sends Candid
    ignore await A.encoded("b"); // warns: expects Candid
    ignore await A.both("c")     // warns twice
  };

  public func go() : async () {
    Prim.debugPrint(await A.plain("plain"));
    let reply = await Prim.call_raw(Prim.principalOfActor(A), "both", "raw");
    Prim.debugPrint(debug_show(reply))
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"
