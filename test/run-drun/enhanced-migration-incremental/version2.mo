//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-incremental/migrations_2

import Prim "mo:prim";

actor {
  let a : Nat;
  let b : Int;
  public func check() : async () {
    Prim.debugPrint(debug_show "V2:");
    Prim.debugPrint(debug_show { a; b });
  };
};
