import Prim "mo:prim";
import Cycles = "../cycles/cycles";
import ReducedService "reduced-service";

actor this {
    type OriginalActor = actor {
        test0() : async ();
        test1() : async ();
        test2() : async ();
        test3() : async ();
    };

    type ReducedActor = actor {
        test1() : async ();
        test3() : async ();
    };

    var instance = [var null : ?ReducedActor];
    var alias = [var null : ?OriginalActor];

    public func initialize() : async () {
        Cycles.add<system>(2_000_000_000_000);
        instance[0] := ?(await ReducedService.ReducedActor());
    };

    public func test() : async () {
        switch (instance[0]) {
            case (?instance) {
                await instance.test1();
                await instance.test3();
            };
            case null Prim.trap("Null");
        };
        switch (alias[0]) {
            case (?alias) {
                await alias.test0();
                await alias.test1();
                await alias.test2();
                await alias.test3();
            };
            case null Prim.trap("Null");
        };
    };
};
