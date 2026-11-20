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

  // A second weak type is checked like the first

  let bad3 : (weak Blob, weak (() -> ())) = (alloc("" : Blob), alloc(func() {})); // reject, non-stable

  do {
    let p : (weak Blob, weak Text) = (alloc("" : Blob), alloc("a"));
    let bad : (weak Blob, weak [Nat]) = p // reject
  };

  do {
    let x = (alloc("" : Blob), alloc("a"));
    let y = (alloc("" : Blob), alloc([1]));
    let z = if wAlive { x } else { y };
    let bad : (weak Blob, weak Text) = z // reject, lub is (weak Blob, weak Any)
  };

  do {
    func second<T, U>(p : (weak T, weak U)) : weak U { p.1 };
    let r = second(alloc("" : Blob), alloc("a"));
    let bad : weak Nat = r // reject, U is Text
  };

  do {
    let p : [var (weak Blob, weak Text)] = [var (alloc("" : Blob), alloc("a"))];
    let bad : [var (weak Blob, weak [Nat])] = p // reject, not equal
  };

  do {
    let f : ((weak Blob, weak Text)) -> () = func(_) {};
    let g : ((weak Blob, weak [Nat])) -> () = func(_) {};
    let h = if wAlive { f } else { g };
    let bad : ((weak Blob, weak Text)) -> () = h // reject, glb is (weak Blob, weak None)
  }

};

actor b  {

  public shared func bad2(x : weak Blob) : async (weak Blob) { x }; // reject, not shared

};
