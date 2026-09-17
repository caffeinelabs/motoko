import Prim "mo:prim";
import Cycles = "../cycles/cycles";
import OriginalService "original-service";

actor this {
    type OriginalActor = actor {
        test1() : async ();
        test2() : async ();
    };

    var instance : ?OriginalActor = null;

    public func initialize() : async () {
        await Cycles.provisional_top_up_actor(this, 100_000_000_000_000);
        Cycles.add<system>(2_000_000_000_000);
        instance := ?(await OriginalService.OriginalActor());
    };

    public func test() : async () {
        switch instance {
            case (?instance) {
                await instance.test1();
                await instance.test2();
            };
            case null Prim.trap("Null");
        };
    };
};
