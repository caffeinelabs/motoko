//MOC-FLAG --trap-on-call-error -A=M0194
import Prim "mo:⛔";
actor {
  var count = 0;
  public func inc() : async () {
    count += 1;
  };

  public func repeat() : async Nat {
    loop {
      ignore inc();
    }
  };

  public func repeatUntil() : async Nat {
    var sent = 0;
    try {
      loop {
       ignore inc();
       sent += 1
      }
    } catch (e) {
      Prim.debugPrint(debug_show {sent});
      return sent
    }
  };

  public func get() : async Nat {
    Prim.debugPrint(debug_show {count});
    return count;
  }

}
//CALL ingress repeat 0x4449444C0000
//CALL ingress get 0x4449444C0000
//CALL ingress repeatUntil 0x4449444C0000
//CALL ingress get 0x4449444C0000
