import Lib "try-finally-trap-stop/class";
import Prim "mo:⛔";

// tests that a canister with a leaky callback table
// (due to a trapping finally block) preventing upgrades,
// can still be upgraded after being stopped.
actor {

  let ic00 = actor "aaaaa-aa" : actor {
    stop_canister : { canister_id : Principal } -> async ();
    start_canister : { canister_id : Principal } -> async ();
  };

  public func go() : async () {
    let pre = Prim.cyclesBalance();
    // instantiate Burner actor
    Prim.cyclesAdd<system>(pre / 2);
    var a = await Lib.C();
    let p = Prim.principalOfActor(a);
    a := await (system Lib.C) (#upgrade a) ();
    await a.show();
    try {
      await a.leak(); // leaks a callback in a's callback table
    } catch _ {};
    await a.show();
    Prim.debugPrint("canister running");
    try {
      a := await (system Lib.C) (#upgrade a) ();
      Prim.debugPrint("canister upgraded");
      assert false;
    } catch e {
      Prim.debugPrint(Prim.errorMessage (e));
    };
    try {
      await ic00.stop_canister({ canister_id = p });
    }
    catch e {
      Prim.debugPrint(Prim.errorMessage (e));
      assert false;
    };
    Prim.debugPrint("canister stopped");
    try {
      a := await (system Lib.C) (#upgrade a) ();
      Prim.debugPrint("canister upgraded");
    } catch e {
      Prim.debugPrint(Prim.errorMessage (e));
      assert false;
    };

    try {
      await ic00.start_canister({ canister_id = p });
    }
    catch e {
      Prim.debugPrint(Prim.errorMessage (e));
      assert false;
    };
    await a.show();
    Prim.debugPrint("done");

  }

}

//SKIP run-low
//SKIP run-ir
//CALL ingress go "DIDL\x00\x00"
