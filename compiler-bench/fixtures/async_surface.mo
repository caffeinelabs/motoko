import Prim "mo:⛔";

/// Async/await and message scheduling surface (IR passes).
persistent actor {
  public func step() : async Nat {
    1;
  };

  public func chain() : async Nat {
    let a = await step();
    let b = await step();
    a + b;
  };

  public func go() : async () {
    ignore await chain();
    ignore Prim.debugPrint("done");
  };
}
