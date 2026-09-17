import Prim "mo:prim";

// Compatible upgrade
actor {
    type RecursiveArray = ?[RecursiveArray];

    var simpleArray = [var 1.0, 2.0, 3.0, 4.0];
    var nestedArray = [[1, 2, 3], [4, 5, 6]];
    var recursiveArray = ?[null, ?[], ?[?[null, null], null]] : RecursiveArray;

    public func print() : async () {
        Prim.debugPrint(debug_show (simpleArray));
        Prim.debugPrint(debug_show (nestedArray));
        Prim.debugPrint(debug_show (recursiveArray));
    };
};
