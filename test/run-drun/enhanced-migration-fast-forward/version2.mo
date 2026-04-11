//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors

import Prim "mo:prim";

actor {
  let a : Text = "";
  var b : Bool = true;

  public func check() : async () {
    Prim.debugPrint(debug_show "Fast-forwarded migrations:");
    Prim.debugPrint(debug_show { a; b });
  };
};
