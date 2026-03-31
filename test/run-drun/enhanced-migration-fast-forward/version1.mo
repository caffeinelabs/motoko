//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors

import Prim "mo:prim";

(with migration = func(old : { a : Text; b : Bool }) : { a : Text; b : Bool } { old })
actor {
  let a : Text = "";
  var b : Bool = true;

  public func check() : async () {
    Prim.debugPrint(debug_show "Fast-forwarded migrations:");
    Prim.debugPrint(debug_show { a; b });
  };
};
