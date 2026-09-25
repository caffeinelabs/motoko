import Prim "mo:prim";

// Incompatible change (mutable invariance).
actor {
  type Node = {
    var value : Nat;
    var next : ?Node;
  };

  var root = { var value = 0; var next = null : ?Node };

  public func increase() : async () {
    root.value += 1;
  };

  public func check() : async () {
    Prim.debugPrint("CHECK " # debug_show (root.value));
    switch root.next {
      case null { Prim.trap("") }
      case ?next {
        assert (next.value == root.value);
      }
    };
  };
};
