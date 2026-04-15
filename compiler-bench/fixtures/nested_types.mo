import Prim "mo:⛔";

/// Stress nested option types and pattern matching (typechecker + lowering).
persistent actor {
  type List<A> = ?(A, List<A>);

  public func go() : async () {
    var xs : List<Nat> = null;
    var i = 0;
    while (i < 256) {
      xs := ?(i, xs);
      i += 1;
    };
    ignore Prim.debugPrint("ok");
  };
}
