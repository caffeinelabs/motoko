// Exercises compile.ml arms:
//   SystemCyclesBurnPrim   (Cycles.burn)
//   OtherPrim "costCall"
//   OtherPrim "costCreateCanister"
//   OtherPrim "costHttpRequest"
//   OtherPrim "costHttpRequestV2"
//   OtherPrim "subnetSelfNodeCount"
//   OtherPrim "costSignWithEcdsa"
//   OtherPrim "costSignWithSchnorr"
//   OtherPrim "costVetkdDeriveKey"
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

    // OtherPrim "costHttpRequestV2": the argument is the Candid encoding of the
    // parameter record; the price is subnet-dependent, so only its sign is shown
    let params = to_candid ({
      request_bytes = 100 : Nat64;
      http_roundtrip_time_ms = 1_000 : Nat64;
      raw_response_bytes = 1_000 : Nat64;
      transformed_response_bytes = 1_000 : Nat64;
      transform_instructions = 1_000_000 : Nat64;
      outcall_type = null : ?{};
    });
    Prim.debugPrint(debug_show (Prim.costHttpRequestV2(params) > 0) # " -- cost_http_request_v2");

    // OtherPrim "subnetSelfNodeCount": every subnet has at least one node
    Prim.debugPrint(debug_show (Prim.subnetSelfNodeCount() > 0) # " -- subnet_self_node_count");

    // OtherPrim "costSignWithEcdsa" / "costSignWithSchnorr" / "costVetkdDeriveKey"
    // drun has empty key sets; all queries produce error codes
    let validKey = "test_key_1";
    let invalidCurveOrAlgorithm : Nat32 = 42;
    let validCurveOrAlgorithm : Nat32 = 0;
    testCost(Prim.costSignWithEcdsa(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_schnorr");
    testCost(Prim.costVetkdDeriveKey(validKey, invalidCurveOrAlgorithm), " -- cost_vetkd_derive_key");
    testCost(Prim.costSignWithEcdsa("wrong_key", validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr("wrong_key", validCurveOrAlgorithm), " -- cost_sign_with_schnorr");
    testCost(Prim.costVetkdDeriveKey("wrong_key", validCurveOrAlgorithm), " -- cost_vetkd_derive_key");
    testCost(Prim.costSignWithEcdsa(validKey, validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, validCurveOrAlgorithm), " -- cost_sign_with_schnorr");
    testCost(Prim.costVetkdDeriveKey(validKey, validCurveOrAlgorithm), " -- cost_vetkd_derive_key");
  };
};

//CALL ingress go "DIDL\x00\x00"
