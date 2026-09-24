import Prim "mo:prim";

actor {
    let instance = [
        var {
            secondField = 1;
        }
    ];

    let alias = [
        var {
            firstField = 1;
            secondField = 1;
            thirdField = 1;
        }
    ];

    public func test() : async () {
        // This would break type safety if memory compatibility check does not prevent upgrade.
        instance[0] := { secondField = 2 };
        // alias[0] would no longer have the firstField and thirdField.

        Prim.debugPrint("secondField=" # debug_show (instance[0].secondField));
        Prim.debugPrint("firstField=" # debug_show (alias[0].firstField));
        Prim.debugPrint("secondField=" # debug_show (alias[0].secondField));
        Prim.debugPrint("thirdField=" # debug_show (alias[0].thirdField));
    };
};
