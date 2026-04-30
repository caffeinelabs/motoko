//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-incremental/migrations_4

import Prim "mo:prim";

actor {
  let b : Bool;
  public func check() : async () {
    Prim.debugPrint(debug_show "V4:");
    Prim.debugPrint(debug_show { b });
  };
};
