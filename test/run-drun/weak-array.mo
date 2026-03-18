//MOC-FLAG --enhanced-orthogonal-persistence -A=M0194
import Prim = "mo:prim";

persistent actor {
  let w : weak[var Nat64] = Prim.allocWeakRef(Prim.Array_init<Nat64>(1, 0));
};

//SKIP run
//SKIP run-ir
//SKIP run-low
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
