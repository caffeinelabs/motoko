//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
// Exercises compile_enhanced.ml arms:
//   OtherPrim "subnetSelfNodeCount"
//   OtherPrim "costHttpRequestV2"
//SKIP run
//SKIP run-low
//SKIP run-ir
//SKIP drun-run

import Prim "mo:⛔";

actor {
  public func go() : async () {
    // OtherPrim "subnetSelfNodeCount": every subnet has at least one node.
    let nodes = Prim.subnetSelfNodeCount();
    Prim.debugPrint(debug_show (nodes > 0) # " -- subnet_self_node_count is positive");

    // OtherPrim "costHttpRequestV2": the argument is the Candid encoding of the
    // parameter record. Any well-formed encoding prices; the value is
    // subnet-dependent, so only its shape is asserted here.
    let params = to_candid ({
      request_bytes = 100 : Nat64;
      http_roundtrip_time_ms = 1_000 : Nat64;
      raw_response_bytes = 1_000 : Nat64;
      transformed_response_bytes = 1_000 : Nat64;
      transform_instructions = 1_000_000 : Nat64;
      outcall_type = null : ?{};
    });
    let cost = Prim.costHttpRequestV2(params);
    Prim.debugPrint(debug_show (cost > 0) # " -- cost_http_request_v2 is positive");
  };
};

//CALL ingress go "DIDL\x00\x00"
