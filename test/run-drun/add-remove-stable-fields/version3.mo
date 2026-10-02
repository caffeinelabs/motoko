import Prim "mo:prim";

// drops `secondValue` while adding `thirdValue`, whose field hash sorts between
// `firstValue` and `secondValue`
actor {
    var firstValue = 0;
    var thirdValue = 0;

    public func increase() : async () {
        firstValue += 1;
        thirdValue += 1;
    };

    public func show() : async () {
        Prim.debugPrint("firstValue=" # debug_show (firstValue));
        Prim.debugPrint("thirdValue=" # debug_show (thirdValue));
    };
};
