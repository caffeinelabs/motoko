//MOC-FLAG  --actor-idl idl-mo

import imported "ic:aaaaa-aa";
/*
type node = {head : Nat; tail : list};
type list = ?node;
type o = ?o;
type stream = ?{head : Nat; next : shared query () -> async stream};
type tree = {#branch : {left : tree; right : tree; val : Int}; #leaf : Int};
type s = actor {f : t; g : shared list -> async (tree, stream)};
type t = shared s -> async ();

type field = shared {_1291438163_ : Nat8; test : tree} -> async {};
*/

type expected = actor {
  field : shared { _1291438163_ : Nat8; test : Nat16 } -> async {};
  fieldnat : shared { _2_ : Int; _50_ : Nat } -> async { _0_ : Int };
  //o : shared o -> async o;
  oneway : shared Nat8 -> ();
  query_ : shared query Blob -> async Blob;
  //service : t;
  tuple : shared ((Int, Blob, Text)) -> async { _0_ : Int; _1_ : Nat8 };
  variant : shared { #A; #B; #C; #D : Float } -> async ()
};

ignore ([var imported] : [var expected]); // variable arrays are invariant, so this checks type equality.

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP drun-run
//SKIP wasm-run
