//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// test candid subtype test with higher-order arguments
actor this {

   public func send_f0(
     f : shared (Nat) -> async Int
   ) : async () {
     Prim.debugPrint("wrong_0");
   };

   public func send_f2(
     f : shared (Nat, [Nat], {f:Nat;g:Nat}, {#l:Nat; #m:Nat}) ->
           async(Int, [Int], {f:Int;g:Int}, {#l:Int; #m:Int})
   ) : async () {
     Prim.debugPrint("wrong_2");
   };

   public func send_f3(
     f : shared () ->
           async (Null, Any, ?None, Nat)
   ) : async () {
     Prim.debugPrint("wrong_3");
   };

   public func send_f5(
     f : shared {a : Nat; b : [Nat]; c : {f:Nat;g:Nat}; d : {#l:Nat; #m:Nat}} ->
           async {a : Int; b : [Int]; c : {f:Int;g:Int}; d : {#l:Int; #m:Int}}
   ) : async () {
     Prim.debugPrint("ok");
   };

   public func send_f6(
     f : shared () ->
           async {a : Null; b : Any; c : ?None}
   ) : async () {
     Prim.debugPrint("ok");
   };


   public func f0(n : Nat) : async Int { 0 };

   public func f0_1_a(n : None) : async Int { 0 };
   public func f0_1_b(n : Nat) : async Any { () };

   public func f0_2_a() : async (Any, Bool) { ((), true) };
   public func f0_2_b(n : Nat, a : Bool ) : async (Int, Bool) { (1, true) };

   public func f0_3_a(on : ?Nat, i : Nat) : async (Int, ?Nat) { (0, null); };
   public func f0_3_b(i : Nat, on : ?Nat) : async (?Nat, Int) { (null, 0); };

   public func f0_4(ob : ?Bool, n : Nat) : async Int { 0 };

   public func f1_0(n : ?Nat) : async (Bool, Nat) { (true, 0) };

   public func f2_0_a(n : Nat, a : [Nat], r : {f : Nat; g : Nat}, v : {#l : Nat; #m : None}) :
     async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Int}) {
        (1, [1], {f=1;g=1}, (#l 0))
     };

   public func f2_0_b(n : Nat, a : [Nat], r : {f : Nat; g : Nat}, v : {#l : Nat; #m : Nat}) :
     async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Any}) {
        (1, [1], {f=1;g=1}, (#l 0))
     };


   public func f2_1_a(n : Int, a : [Int], r : {f : Int}, v : {#l : Int; #m : None; #o : Int}) :
     async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, { #l : Nat}) {
        (1, [1], {f = 1; g = 2; h = 3}, (#l 0))
     };

   public func f2_1_b(n : Int, a : [Int], r : {f : Int}, v : {#l : Int; #m : Int; #o : Int}) :
     async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, { #l : Any}) {
        (1, [1], {f = 1; g = 2; h = 3}, (#l 0))
     };


   public func f3_0_a(x : Nat) :
     async (n : Null, a : ?None, r : Any) {
       (null, null, null)
     };

   public func f3_0_b() :
     async (n : Null, a : ?None, r : Any) {
       (null, null, null)
     };

   public func f5_0_a({a = n : Nat; b = a : [Nat]; c = r : {f : Nat; g : Nat}; d = v : {#l : Nat; #m : None}}) :
     async { a : Int; b : [Int]; c : {f : Int; g : Int}; d : {#l : Int; #m : Int} } {
        { a = 1; b = [1]; c = {f =1 ; g = 1}; d = (#l 0)}
     };

   public func f5_0_b({a = n : Nat; b = a : [Nat]; c = r : {f : Nat; g : Nat}; d = v : {#l : Nat; #m : Nat}}) :
     async { a : Int; b : [Int]; c : {f : Int; g : Int}; d : {#l : Int; #m : Any} } {
        { a = 1; b = [1]; c = {f =1 ; g = 1}; d = (#l 0)}
     };

   public func f5_1_a({a = n : Int; b = a : [Int]; c = r : {f : Int}; d = v : {#l : Int; #m : None; #o : Int} }) :
     async { a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : { #l : Nat}} {
        { a = 1; b = [1]; c = {f = 1; g = 2; h = 3}; d = (#l 0)}
     };

   public func f5_1_b({a = n : Int; b = a : [Int]; c = r : {f : Int}; d = v : {#l : Int; #m : Int; #o : Int} }) :
     async { a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : { #l : Any}} {
        { a = 1; b = [1]; c = {f = 1; g = 2; h = 3}; d = (#l 0)}
     };


   public func f6_0_a({x = _:Nat}) :
     async {a : Null; b : ?None; c : Any} {
       { a= null; b = null; c = null}
     };

   public func f6_0_b({}) :
     async {a : Null; b : ?None; c : Any} {
       { a= null; b = null; c = null}
     };

   public func go() : async () {
      let t = debug_show (Prim.principalOfActor(this));

      // vanilla subtyping on in/out args
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n:None) -> async Int) -> async ();
        };
        try {
          await this.send_f0(f0_1_a);
          Prim.debugPrint "wrong_0_1_a";
        }
        catch e {
          Prim.debugPrint "ok_0_1_a";
        }
      };

      // vanilla subtyping on in/out args
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n:Nat) -> async Any) -> async ();
        };
        try {
          await this.send_f0(f0_1_b);
          Prim.debugPrint "wrong_0_1_b";
        }
        catch e {
          Prim.debugPrint "ok_0_1_b";
        }
      };

     // negative vanilla subtyping on in/out arg sequences
     do {
        let this = actor (t) : actor {
          send_f0 : (shared () -> async (Any, Bool)) -> async ();
        };
        try {
          await this.send_f0(f0_2_a);
          Prim.debugPrint "wrong_0_2_a";
        }
        catch e {
          Prim.debugPrint "ok_0_2_a";
        }
      };

     // negative vanilla subtyping on in/out arg sequences
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n : Nat, a : Bool) -> async (Int, Bool)) -> async ();
        };
        try {
          await this.send_f0(f0_2_b);
          Prim.debugPrint "wrong_0_2_b";
        }
        catch e {
          Prim.debugPrint "ok_0_2_a";
        }
      };

      // negative opt subtyping in arg and return
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (?Nat, Nat) -> async (Int, ?Nat)) -> async ();
        };
        try {
          await this.send_f0(f0_3_a);
          Prim.debugPrint "wrong_0_3_a";
        }
        catch e {
          Prim.debugPrint "ok_0_3_a";
        }
      };

      // negative opt subtyping in arg and return
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (Nat, ?Nat) -> async (?Nat, Int)) -> async ();
        };
        try {
          await this.send_f0(f0_3_b);
          Prim.debugPrint "wrong_0_3_b";
        }
        catch e {
          Prim.debugPrint "ok_0_3_b";
        }
      };

      // negative opt override in arg
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (?Bool, Nat) -> async Int) -> async ();
        };
        try {
          await this.send_f0(f0_4);
          Prim.debugPrint "wrong_0_4";
        }
        catch e {
          Prim.debugPrint "ok 0_4"; }
      };


      // negative several args
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Nat, [Nat], {f : Nat; g : Nat}, {#l : Nat; #m : None}) ->
              async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Int})) ->
                async ()
        };
        try {
          await this.send_f2(f2_0_a);
          Prim.debugPrint "wrong_2_0_a";
        }
        catch e {
          Prim.debugPrint "ok 2_0_a"; }
      };

      // several args
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Nat, [Nat], {f : Nat; g : Nat}, {#l : Nat; #m : Nat}) ->
              async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Any})) ->
                async ()
        };
        try {
          await this.send_f2(f2_0_b);
          Prim.debugPrint "wrong_2_0_b";
        }
        catch e {
          Prim.debugPrint "ok 2_0_b"; }
      };

      // negative several args, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Int, [Int], {f : Int}, {#l : Int; #m : None; #o : Int}) ->
              async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, {#l : Nat})) ->
                async ()
        };
        try {
          await this.send_f2(f2_1_a);
          Prim.debugPrint "wrong 2_1_a";
        }
        catch e { Prim.debugPrint "ok 2_1_a"; }
      };

      // negative several args, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Int, [Int], {f : Int}, {#l : Int; #m : Int; #o : Int}) ->
              async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, {#l : Any})) ->
                async ()
        };
        try {
          await this.send_f2(f2_1_b);
          Prim.debugPrint "wrong 2_1_b";
        }
        catch e {
          Prim.debugPrint "ok 2_1_b";}
      };


      // negative null, opt and any trailing args, defaulting
      do {
        let this = actor (t) : actor {
          send_f3 :
            (shared Nat ->
              async (Null, ?None, Any)) ->
                async ()
        };
        try {
          await this.send_f3(f3_0_a);
          Prim.debugPrint "wrong 3_0_a";
        }
        catch e {
          Prim.debugPrint "ok 3_0_a";
        }
      };

      // negative null, opt and any trailing args, defaulting
      do {
        let this = actor (t) : actor {
          send_f3 :
            (shared () ->
              async (Null, ?None, Any)) ->
                async ()
        };
        try {
          await this.send_f3(f3_0_b);
          Prim.debugPrint "wrong 3_0_b";
        }
        catch e {
          Prim.debugPrint "ok 3_0_b";
        }
      };

      // negative several fields
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared {a : Nat; b : [Nat]; c : {f : Nat; g : Nat}; d : {#l : Nat; #m : None}} ->
              async {a : Int; b : [Int]; c : {f : Int; g : Int}; d : {#l : Int; #m : Int}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_0_a);
          Prim.debugPrint "wrong 5_0_a";
        }
        catch e {
          Prim.debugPrint "ok 5_0_a"; }
      };

      // negative several fields
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared { a : Nat; b : [Nat]; c : {f : Nat; g : Nat}; d : {#l : Nat; #m : Nat}} ->
              async { a : Int; b : [Int]; c :  {f : Int; g : Int}; d : {#l : Int; #m : Any}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_0_b);
          Prim.debugPrint "wrong 5_0_b";
        }
        catch e {
          Prim.debugPrint "ok 5_0_b"; }
      };

      // negative several fields, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared { a : Int; b : [Int]; c : {f : Int}; d : {#l : Int; #m : None; #o : Int}} ->
              async { a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : {#l : Nat}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_1_a);
          Prim.debugPrint "wrong 5_1_a";
        }
        catch e { Prim.debugPrint "ok 5_1_a"; }
      };

      // negative several args, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared { a : Int; b : [Int]; c : {f : Int}; d : {#l : Int; #m : Int; #o : Int}} ->
              async { a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : {#l : Any}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_1_b);
          Prim.debugPrint "wrong 5_1_b";
        }
        catch e {
          Prim.debugPrint "ok 5_1_b";}
      };


      // negative null, opt and any fields, defaulting
      do {
        let this = actor (t) : actor {
          send_f6 :
            (shared {x : Nat} ->
              async { a : Null; b : ?None; c : Any}) ->
                async ()
        };
        try {
          await this.send_f6(f6_0_a);
          Prim.debugPrint "wrong 6_0_a";
        }
        catch e {
          Prim.debugPrint "ok 6_0_a";
        }
      };

      // negative null, opt and any fields, defaulting
      do {
        let this = actor (t) : actor {
          send_f6 :
            (shared {} ->
              async {a : Null; b : ?None; c : Any}) ->
                async ()
        };
        try {
          await this.send_f6(f6_0_b);
          Prim.debugPrint "wrong 6_0_b";
        }
        catch e {
          Prim.debugPrint "ok 6_0_b";
        }
      };

   };

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
