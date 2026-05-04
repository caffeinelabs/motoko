//MOC-FLAG --actor-idl composite-import
//MOC-FLAG --actor-alias self rwlgt-iiaaa-aaaaa-aaaaa-cai

// this imports our own IDL, stored in actor-import

// currently hard-codes the ic-ref self id
// once we have actor aliases we can let run-test set an alias.
import Prim = "mo:⛔";
import imported1 "ic:rwlgt-iiaaa-aaaaa-aaaaa-cai";
import imported2 "canister:self";
actor {
  public func go1() : async (actor { cq : composite query () -> async () }) = async {
     imported1;
  };

  public func go2() : async (actor { cq : composite query () -> async () }) = async {
     imported2;
  };


  public composite query func cq() : async () {
    Prim.debugPrint "cq()";
    if false await imported1.cq();
    if false await imported2.cq();
  };

};
//CALL ingress go1 "DIDL\x00\x00"
//CALL ingress go2 "DIDL\x00\x00"
//CALL query cq "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low

