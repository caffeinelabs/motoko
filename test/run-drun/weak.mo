//MOC-FLAG --enhanced-orthogonal-persistence
import Prim = "mo:prim";

actor {

  let w = Prim.allocWeakRef("hello");
  Prim.debugPrint(debug_show w);
  let l = Prim.isLive(w);
  Prim.debugPrint(debug_show l);
  let c = Prim.weakGet(w);
  Prim.debugPrint(debug_show c);

};

//SKIP run
//SKIP run-ir
//SKIP run-low
