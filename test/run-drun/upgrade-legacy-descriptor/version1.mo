import Prim "mo:prim";

actor {
   let r = {
      a : [var Nat] = [var];
      var b : [Nat] = [];
      var c : [var Nat] = [var];
      d : [var [var Nat]] = [var];
      var e : Char = 'e';
      f : [var Char] = [var];
      g : Nat32 = 0;
      h : () = ();
      i : Null = null;
      j : { #on; #off } = #on;
      k : ?(shared (Char, ()) -> async ()) = null;
      l : [[var Nat]] = [];
   };

   public func peek() : async () {
      Prim.debugPrint(debug_show (r.a, r.b, r.c, r.d, r.e, r.f, r.g, r.h, r.i, r.j, r.l));
   };
};
