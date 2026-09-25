import Prim "mo:⛔";
import Cycles = "cycles/cycles";
actor a {

  let wasm_mod : Blob = "\00\61\73\6D\01\00\00\00";
  let empty_arg : Blob = "";

  public func go() : async() {
    // To get lots of cycles in drun
    if Cycles.balance() == 0
      { await Cycles.provisional_top_up_actor(a, 100_000_000_000_000) };

    try {
      Prim.debugPrint("Installing actor:");
      Cycles.add<system>(2_000_000_000_000);
      let principal = await Prim.createActor(wasm_mod, empty_arg);
      let id = debug_show principal;
      Prim.debugPrint(id);
      let a = actor (id) : actor {};
    } catch e {
      Prim.debugPrint("Exception: " # debug_show (Prim.errorMessage(e)));
    }
  };
}
//CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
