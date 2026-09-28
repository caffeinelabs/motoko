//MOC-FLAG --enhanced-orthogonal-persistence
import Prim = "mo:prim";

actor {

  do {
    let v = "hello";
    let w = Prim.allocWeakRef(?v);
    Prim.debugPrint(debug_show w);
    Prim.debugPrint(debug_show (Prim.weakGet(w)));
    assert (Prim.weakGet(w) == ??v);
  };

  do {
    let v = null;
    let w = Prim.allocWeakRef(?v);
    Prim.debugPrint(debug_show w);
    Prim.debugPrint(debug_show (Prim.weakGet(w)));
    assert (Prim.weakGet(w) == ??v);
  };

  do {
    let v = "hello";
    let w = Prim.allocWeakRef(?v);
    Prim.debugPrint(debug_show w);
    Prim.debugPrint(debug_show (Prim.weakGet(w)));
    assert (Prim.weakGet(w) == ??v);
  };

  do {
    let v = ?"hello";
    let w = Prim.allocWeakRef(?v);
    Prim.debugPrint(debug_show w);
    Prim.debugPrint(debug_show (Prim.weakGet(w)));
    assert (Prim.weakGet(w) == ??v);
  };

  do {
    let v = ?null;
    let w = Prim.allocWeakRef(?v);
    Prim.debugPrint(debug_show w);
    Prim.debugPrint(debug_show (Prim.weakGet(w)));
    assert (Prim.weakGet(w) == ??v);
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
