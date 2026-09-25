import Prim "mo:⛔";
// This test checks that the IDL decoder can
// do the subtyping from null to option
actor {
  public query func any(o : ?Text) : async () {
    switch o {
      case null { Prim.debugPrint ("ok: null") }
      case ?x { Prim.debugPrint ("ok: " # x) }
    }
  };
}

//CALL query any 0x4449444C00017f
//CALL query any 0x4449444C016e71010000
//CALL query any 0x4449444C016e7101000103466F6F

//SKIP run
//SKIP run-ir
//SKIP run-low
