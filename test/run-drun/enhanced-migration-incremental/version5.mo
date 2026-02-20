//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-incremental/migrations_5

import Prim "mo:prim";

actor {
  let a : Text;
  var b : Bool;

  public func check() : async () {
    Prim.debugPrint(debug_show "V5:");
    Prim.debugPrint(debug_show { a; b });
  };
};
