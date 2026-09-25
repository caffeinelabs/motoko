import Prim "mo:⛔";
import Cycles "cycles/cycles";
import Lib "clone/cloneable";

actor Cloner {

  // Calls Lib.Cloneable to construct a new Clonable,
  // passing itself as first argument, using available funds
  public shared func makeCloneable(init : Nat): async Lib.Cloneable {
    let accepted = Cycles.accept<system>(Cycles.available());
    Prim.debugPrint(debug_show { accepted });
    await (with cycles = accepted) Lib.Cloneable(makeCloneable, init);
  };

  public shared func test() : async () {
    // get some cycles
    if Cycles.balance() == 0
    { await Cycles.provisional_top_up_actor(Cloner, 100_000_000_000_000) };

    // create the original Cloneable object
    let c0 : Lib.Cloneable = await (with cycles = 10_000_000_000_000) makeCloneable(0);
    await (with cycles = 42_000_000) c0.someMethod(); // prints 1
    Prim.debugPrint(debug_show(Prim.principalOfActor c0));

    // create some proper clones
    let c1 = await c0.clone(1); // clone!
    await c1.someMethod(); // prints 2
    Prim.debugPrint(debug_show(Prim.principalOfActor c1));

    let c2 = await c1.clone(2); // clone!
    await c2.someMethod(); // prints 3
    Prim.debugPrint(debug_show(Prim.principalOfActor c2));

    await c0.someMethod(); // prints 2
  }

};

// testing
//SKIP run
//SKIP run-ir
//SKIP run-low
ignore Cloner.test(); //OR-CALL ingress test "DIDL\x00\x00"
