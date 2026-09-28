import Prim "mo:prim";

actor {
    let instance = [
        var {
            firstField = 0;
            secondField = 0;
            thirdField = 0;
        }
    ];

    let alias = instance;

    public func test() : async () {
        Prim.debugPrint("firstField=" # debug_show (instance[0].firstField));
        Prim.debugPrint("secondField=" # debug_show (instance[0].secondField));
        Prim.debugPrint("thirdField=" # debug_show (instance[0].thirdField));
        Prim.debugPrint("firstField=" # debug_show (alias[0].firstField));
        Prim.debugPrint("secondField=" # debug_show (alias[0].secondField));
        Prim.debugPrint("thirdField=" # debug_show (alias[0].thirdField));
    };
};
