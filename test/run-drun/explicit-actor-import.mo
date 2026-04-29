//MOC-FLAG --actor-idl actor-import
//MOC-FLAG --actor-alias self lg264-qjkae

// this imports our own IDL, stored in actor-import

// currently hard-codes the ic-ref self id
// once we have actor aliases we can let run-test set an alias.

import imported1 "ic:lg264-qjkae";
import { go = imported1_go } "ic:lg264-qjkae";
import { go = imported2_go } "canister:self";

actor a {
  public func go() : async (actor {}) = async imported1;
  public func go2() : async (actor {}) = async await (imported1_go());
  public func go3() : async (actor {}) = async await (imported2_go());
};
//CALL ingress go "DIDL\x00\x00"
//CALL ingress go2 "DIDL\x00\x00"
//CALL ingress go3 "DIDL\x00\x00"


//SKIP run
//SKIP run-ir
//SKIP run-low

// Skip running on drun for now; hard to pass a `--actor-alias` that works for
// both drun and ic-ref-run
//SKIP comp

