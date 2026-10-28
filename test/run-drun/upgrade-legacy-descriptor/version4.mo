import Prim "mo:prim";

actor {
   let r = {
      var a : [Nat] = [1];
      b : [var Nat] = [var 2];
      var c : [var Nat] = [var 3];
      d : [var [var Nat]] = [var [var 4]];
      var e : Char = 'e';
      f : [var Char] = [var 'f'];
      g : Char = 'g';
      h : () = ();
      i : Null = null;
      j : { #on; #off } = #on;
      k : ?(shared (Char, ()) -> async ()) = null;
      l : [[var Nat]] = [[var 8]];
   };

   public func peek() : async () {
      Prim.debugPrint(debug_show (r.a, r.b, r.c, r.d, r.e, r.f, r.g, r.h, r.i, r.j, r.l));
   };
};
