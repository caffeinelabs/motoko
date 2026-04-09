//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-incremental/migrations_3

import Prim "mo:prim";

actor {
  let a : Nat;
  let b : Bool;
  public func check() : async () {
    Prim.debugPrint(debug_show "V3:");
    Prim.debugPrint(debug_show { a; b });
  };
};
