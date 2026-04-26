import Prim "mo:prim";
import Cycles = "../cycles/cycles";
import ReducedService "reduced-service";

actor this {
    type OriginalActor = actor {
        test1() : async ();
        test2() : async ();
    };

    type ReducedActor = actor {
        test1() : async ();
    };

    stable var instance : ?ReducedActor = null;

    public func initialize() : async () {
        Cycles.add<system>(2_000_000_000_000);
        instance := ?(await ReducedService.ReducedActor());
    };

    public func test() : async () {
        let inst = instance ?? Prim.trap("Null");
        await inst.test1();
    };
};
