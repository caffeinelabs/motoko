//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-incremental/migrations_1

import Prim "mo:prim";

actor {
  let a : Nat;
  public func check() : async () {
    Prim.debugPrint(debug_show "V1:");
    Prim.debugPrint(debug_show { a });
  };
};
