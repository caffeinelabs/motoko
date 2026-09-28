import Prim "mo:prim";
import Cycles = "../cycles/cycles";
import OriginalService "original-service";

actor this {
  type OriginalActor = actor {
    test0() : async ();
    test1() : async ();
    test2() : async ();
    test3() : async ();
  };

  var instance = [var null : ?OriginalActor];
  var alias = instance;

  public func initialize() : async () {
    await Cycles.provisional_top_up_actor(this, 100_000_000_000_000);
    Cycles.add<system>(2_000_000_000_000);
    instance[0] := ?(await OriginalService.OriginalActor());
  };

  public func test() : async () {
    switch instance[0] {
      case ?instance {
        await instance.test0();
        await instance.test1();
        await instance.test2();
        await instance.test3();
      }
      case null { Prim.trap("Null") }
    };
    switch alias[0] {
      case ?alias {
        await alias.test0();
        await alias.test1();
        await alias.test2();
        await alias.test3();
      }
      case null { Prim.trap("Null") }
    };
  };
};
