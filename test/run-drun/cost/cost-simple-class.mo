import Prim "mo:⛔";

module {
  public func test() {
    Prim.debugPrint(debug_show (Prim.costCall(15, 1)) # " -- cost_call");
    Prim.debugPrint(debug_show (Prim.costCreateCanister()) # " -- cost_create_canister");
    Prim.debugPrint(debug_show (Prim.costHttpRequest(15, 2000)) # " -- cost_http_request");

    // Future work: Operation below do not work as expected in drun. Fix them once PocketIC is the new runner.
    let validKey = "test_key_1"; // This is a valid key, but drun has an empty set of keys.
    let invalidKey = "wrong_key";
    let validCurveOrAlgorithm: Nat32 = 0; // Valid argument for both ecdsa and schnorr.
    let invalidCurveOrAlgorithm: Nat32 = 42; // Just a big number which is impossible to be valid.
    testCost(Prim.costSignWithEcdsa(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, invalidCurveOrAlgorithm), " -- cost_sign_with_schnorr");

    testCost(Prim.costSignWithEcdsa(invalidKey, validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(invalidKey, validCurveOrAlgorithm), " -- cost_sign_with_schnorr");

    testCost(Prim.costSignWithEcdsa(validKey, validCurveOrAlgorithm), " -- cost_sign_with_ecdsa");
    testCost(Prim.costSignWithSchnorr(validKey, validCurveOrAlgorithm), " -- cost_sign_with_schnorr");
  };

  func testCost((resultCode : Nat32, costOrUndefined : Nat), msg: Text) {
    if resultCode == 0 {
      Prim.debugPrint(debug_show (0, costOrUndefined) # msg);
    } else {
      Prim.debugPrint(debug_show (resultCode, "undefined-cost") # msg);
    };
  };
};
