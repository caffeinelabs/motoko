//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:⛔";
actor a {

  type w = weak Blob; // type formation

  transient let alloc = Prim.allocWeakRef;

  let w : weak Blob = Prim.allocWeakRef("" : Blob); // accept, stable with eop
  let wAlive = Prim.isLive(w);

  let bad1 : weak(() -> ()) = Prim.allocWeakRef(func() {}); // reject,  non-stable

  let bad1Alive = Prim.isLive(bad1);

  do {
    let bad : weak Any = w ; // weak T invariant, accept
  };

  do {
    let bad = [alloc "hello", alloc {} ]; // weak T invariant, accept
  };

};

actor b  {

  public shared func bad2(x : weak Blob) : async (weak Blob) { x }; // reject, not shared

};
