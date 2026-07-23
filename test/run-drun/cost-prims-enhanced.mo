//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
// Exercises compile_enhanced.ml arms:
//   SystemCyclesBurnPrim   (Cycles.burn)
//   OtherPrim "costCall"
//   OtherPrim "costCreateCanister"
//   OtherPrim "costHttpRequest"
//   OtherPrim "costSignWithEcdsa"
//   OtherPrim "costSignWithSchnorr"
//SKIP run
//SKIP run-low
//SKIP run-ir
//SKIP drun-run

import Prim "mo:⛔";

actor {
  func testCost((resultCode : Nat32, costOrUndefined : Nat), msg : Text) {
    if (resultCode == 0) {
      Prim.debugPrint(debug_show (0, costOrUndefined) # msg);
    } else {
      Prim.debugPrint(debug_show (resultCode, "undefined-cost") # msg);
    };
  };

  public func go() : async () {
    // SystemCyclesBurnPrim: burning 0 is safe on any subnet
    let burned = Prim.cyclesBurn<system>(0);
    Prim.debugPrint(debug_show burned # " -- cycles_burn");

    // OtherPrim "costCall"
    Prim.debugPrint(debug_show (Prim.costCall(15, 1)) # " -- cost_call");

    // OtherPrim "costCreateCanister"
    Prim.debugPrint(debug_show (Prim.costCreateCanister()) # " -- cost_create_canister");

    // OtherPrim "costHttpRequest"
    Prim.debugPrint(debug_show (Prim.costHttpRequest(15, 2000)) # " -- cost_http_request");

    // OtherPrim "costSignWithEcdsa" / "costSignWithSchnorr"
    // drun has empty key sets; all queries produce error codes
    let validKey = "test_key_1";
    let invalidCurveOrAlgorithm : Nat32 = 42;
    let validCurveOrAlgorithm : Nat32 = 0;
    testCost(Prim.costSignWithEcdsa(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_schnorr");
    testCost(Prim.costSignWithEcdsa("wrong_key", validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr("wrong_key", validCurveOrAlgorithm), " -- cost_sign_with_schnorr");
    testCost(Prim.costSignWithEcdsa(validKey, validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, validCurveOrAlgorithm), " -- cost_sign_with_schnorr");
  };
};

//CALL ingress go "DIDL\x00\x00"
