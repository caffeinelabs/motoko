import Prim "mo:prim";

actor {
    type Node = {
        var value : Nat;
        var next : ?Node;
        var test : Nat;
    };

    let node = { var value = 0; var next = null : ?Node; var test = 0 };
    node.next := ?node;

    var root = node;

    public func increase() : async () {
        root.value += 1;
        root.test := root.value;
    };

    public func check() : async () {
        Prim.debugPrint("CHECK " # debug_show (root.value));
        assert (root.value == root.test);
        switch (root.next) {
            case null Prim.trap("");
            case (?next) {
                assert (next.value == root.value);
                assert (next.value == root.test);
            };
        };
    };
};
