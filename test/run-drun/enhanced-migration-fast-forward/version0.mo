//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-fast-forward/migrations

import Prim "mo:prim";

actor {
  let a : Text;
  var b : Bool;

  public func check() : async () {
    Prim.debugPrint(debug_show "Fast-forwarded migrations:");
    Prim.debugPrint(debug_show { a; b });
  };
};
