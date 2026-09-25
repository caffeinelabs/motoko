import Prim "mo:⛔";

actor {
  public func go() : async Nat {
    try {
      let 5 = 6 else { throw Prim.error "test error" };
      return 2;
    } catch _ {
      return 1;
    };
  };
};

//CALL ingress go "DIDL\x00\x00"
