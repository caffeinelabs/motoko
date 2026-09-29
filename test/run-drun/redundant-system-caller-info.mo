//MOC-FLAG --error-format=json
import Prim "mo:prim";

// A now-redundant explicit `<system>` still compiles, with warning M0276
actor {
  public func test() : async () {
    Prim.debugPrint(debug_show Prim.callerInfoSigner<system>());
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress test 0x4449444C0000
