//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors

import Prim "mo:prim";

actor {
  Prim.debugPrint("Version 0");

  var zero : Nat = 0;

  var one : [var Nat] = [var 1, 2, 3, 4];
  var two : [var Text] = [var "1", "2", "3", "4"];

  public func check() : async () {
    Prim.debugPrint(debug_show { zero; one; two });
  };
};
