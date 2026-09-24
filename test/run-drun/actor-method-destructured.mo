//MOC-FLAG --actor-id-alias self rwlgt-iiaaa-aaaaa-aaaaa-cai actor-method-destructured/rwlgt-iiaaa-aaaaa-aaaaa-cai.did

import IC "canister:self";
import { go } "canister:self";

// The Same<T> probe asserts the destructured method has identical type
// to the equivalent dotted access on the full-module import.

actor {
  func Same<T>(_ : T, _ : T, _ : T) {};

  public func test() : async () {
    let { go = go3 } = IC;     // general let-destructure of an actor handle
    Same(IC.go, go, go3)
  };
};

// `canister:` imports aren't resolvable in the AST interpreter (no
// actor to point at), so `run` is skipped.  `run-ir` / `run-low`
// operate on the IR after desugar, which has already routed the
// destructured import through ActorDotPrim — no I.ObjP, no breakage.
//SKIP run
