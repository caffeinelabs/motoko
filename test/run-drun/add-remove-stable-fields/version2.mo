import Prim "mo:prim";

actor {
    var secondValue = 0;

    public func increase() : async () {
        secondValue += 1;
    };

    public func show() : async () {
        Prim.debugPrint("secondValue=" # debug_show (secondValue));
    };
};
