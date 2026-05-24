//MOC-FLAG --actor-idl actor-method-destructured
//MOC-FLAG --actor-alias self rwlgt-iiaaa-aaaaa-aaaaa-cai

// gabor/actor-method-destructured:
//   destructuring import of an actor — method projection via desugar's
//   ActorDotPrim path, sidestepping the IR-level ObjP lowering that
//   would corrupt the actor blob at runtime.
//
// The Same<T> probe asserts the destructured method has identical type
// to the equivalent dotted access on the full-module import.

import IC "canister:self";
import { go } "canister:self";

actor {
  func Same<T>(_ : T, _ : T) {};

  public func test() : async () {
    Same(IC.go, go)
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low
