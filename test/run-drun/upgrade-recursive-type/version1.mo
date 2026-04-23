import Prim "mo:prim";

// Incompatible change (mutable invariance).
actor {
    type Node = {
        var value : Nat;
        var next : ?Node;
    };

    stable var root = { var value = 0; var next = null : ?Node };

    public func increase() : async () {
        root.value += 1;
    };

    public func check() : async () {
        Prim.debugPrint("CHECK " # debug_show (root.value));
        let next = root.next ?? Prim.trap("");
        assert (next.value == root.value);
    };
};
