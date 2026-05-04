//MOC-FLAG --actor-idl actor-import
//MOC-FLAG --actor-alias self rwlgt-iiaaa-aaaaa-aaaaa-cai

// this imports our own IDL, stored in actor-import

// currently hard-codes the `drun` self id
// once we have actor aliases we can let run-test set an alias.

import imported1 "ic:rwlgt-iiaaa-aaaaa-aaaaa-cai";
import imported2 "canister:self";
actor {
  public func go() : async (actor {}) = async imported1;
  public func go2() : async (actor {}) = async await (imported1.go());
  public func go3() : async (actor {}) = async await (imported2.go());
};
//CALL ingress go "DIDL\x00\x00"
//CALL ingress go2 "DIDL\x00\x00"
//CALL ingress go3 "DIDL\x00\x00"


//SKIP run
//SKIP run-ir
//SKIP run-low

