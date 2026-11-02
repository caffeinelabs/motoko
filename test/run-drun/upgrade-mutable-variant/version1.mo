import Prim "mo:prim";

actor {
    type OriginalVariant = {
        #Option1;
        #Option2;
    };

    type ExtendedVariant = {
        #Option0;
        #Option1;
        #Option2;
    };

    let instance = [ var #Option1: OriginalVariant ];
    let alias = [ var #Option1: ExtendedVariant ];

    public func test() : async () {
        // Would break type safety if not rejected by the memory compatibility check of the upgrade.
        alias[0] := #Option0;
        // instance would observe invalid enum value.

        Prim.debugPrint("instance=" # debug_show (instance));
        Prim.debugPrint("alias=" # debug_show (alias));
    };
};
