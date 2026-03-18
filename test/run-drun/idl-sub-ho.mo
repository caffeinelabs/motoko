//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// test candid subtype test with higher-order arguments
actor this {

   public func send_f0(
     f : shared (Nat) -> async Int
   ) : async () {
     Prim.debugPrint("ok");
   };

   public func send_f1(
     f : shared (?Nat) -> async ?Int
   ) : async () {
     Prim.debugPrint("ok");
   };

   public func send_f2(
     f : shared (Nat, [Nat], {f:Nat;g:Nat}, {#l:Nat; #m:Nat}) ->
           async(Int, [Int], {f:Int;g:Int}, {#l:Int; #m:Int})
   ) : async () {
     Prim.debugPrint("ok");
   };

   public func send_f3(
     f : shared () ->
           async (Null, Any, ?None)
   ) : async () {
     Prim.debugPrint("ok");
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

   public func f0_1(n : Int) : async Nat { 0 };

   public func f0_2() : async (Nat, Bool) { (0,true) };

   public func f0_3(i : Nat, on : ?Nat) : async (Int, ?Nat) { (0, null); };

   public func f0_4(ob : ?Bool) : async Int { 0 };

   public func f1_0(n : ?Nat) : async Bool { true };

   public func f2_0(n : Nat, a : [Nat], r : {f : Nat; g : Nat}, v : {#l : Nat; #m : Nat}) :
     async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Int}) {
        (1, [1], {f=1;g=1}, (#l 0))
     };

   public func f2_1(n : Int, a : [Int], r : {f : Int}, v : {#l : Int; #m : Int; #o : Int}) :
     async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, { #l : Nat}) {
        (1, [1], {f = 1; g = 2; h = 3}, (#l 0))
     };

   public func f3_0() :
     async (n : Null, a : ?None, r : Any) {
       (null, null, null)
     };

   public func f5_0({a = n : Nat; b = a : [Nat]; c = r : {f : Nat; g : Nat}; d = v : {#l : Nat; #m : Nat}}) :
     async { a : Int; b : [Int]; c : {f : Int; g : Int}; d : {#l : Int; #m : Int} } {
        { a = 1; b = [1]; c = {f =1 ; g = 1}; d = (#l 0)}
     };

   public func f5_1({a = n : Int; b = a : [Int]; c = r : {f : Int}; d = v : {#l : Int; #m : Int; #o : Int} }) :
     async { a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : { #l : Nat}} {
        { a = 1; b = [1]; c = {f = 1; g = 2; h = 3}; d = (#l 0)}
     };

   public func f6_0() :
     async {a : Null; b : ?None; c : Any} {
       { a= null; b = null; c = null}
     };

   public func go() : async () {

      let t = debug_show (Prim.principalOfActor(this));

      // vanilla subtyping on in/out args
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n:Int) -> async Nat) -> async ();
        };
        try {
          await this.send_f0(f0_1);
        }
        catch e { Prim.debugPrint "wrong_0_1"; }
      };

      // vanilla subtyping on in/out arg sequences
      do {
        let this = actor (t) : actor {
          send_f0 : (shared () -> async (Nat,Bool)) -> async ();
        };
        try {
          await this.send_f0(f0_2);
        }
        catch e { Prim.debugPrint "wrong_0_2"; }
      };

      // opt subtyping in arg and return
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (Nat, ?Nat) -> async (Int, ?Nat)) -> async ();
        };
        try {
          await this.send_f0(f0_3);
        }
        catch e { Prim.debugPrint "wrong_0_3"; }
      };

      // opt override in arg
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (?Bool) -> async Int) -> async ();
        };
        try {
          await this.send_f0(f0_4);
        }
        catch e { Prim.debugPrint "wrong_0_4"; }
      };


      // opt override in return
      do {
        let this = actor (t) : actor {
          send_f1 : (shared (?Nat) -> async Bool) -> async ();
        };
        try {
          await this.send_f1(f1_0);
        }
        catch e { Prim.debugPrint "wrong_1_0"; }
      };

      // several args
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Nat, [Nat], {f : Nat; g : Nat}, {#l : Nat; #m : Nat}) ->
              async (Int, [Int], {f : Int; g : Int}, {#l : Int; #m : Int})) ->
                async ()
        };
        try {
          await this.send_f2(f2_0);
        }
        catch e { Prim.debugPrint "wrong_2_0"; }
      };

      // several args, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f2 :
            (shared (Int, [Int], {f : Int}, {#l : Int; #m : Int; #o : Int}) ->
              async (Nat, [Nat], {f : Nat; g : Nat; h : Nat}, {#l : Nat})) ->
                async ()
        };
        try {
          await this.send_f2(f2_1);
        }
        catch e { Prim.debugPrint "wrong_2_1"; }
      };


      // null, opt and any trailing args, defaulting
      do {
        let this = actor (t) : actor {
          send_f3 :
            (shared () ->
              async (Null, ?None, Any)) ->
                async ()
        };
        try {
          await this.send_f3(f3_0);
        }
        catch e { Prim.debugPrint "wrong_3_0"; }
      };


      // record arg
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared {a : Nat; b : [Nat]; c :  {f : Nat; g : Nat}; d : {#l : Nat; #m : Nat}} ->
              async {a : Int; b : [Int]; c : {f : Int; g : Int}; d : {#l : Int; #m : Int}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_0);
        }
        catch e { Prim.debugPrint "wrong_5_0"; }
      };

      // record arg, contra-co subtyping
      do {
        let this = actor (t) : actor {
          send_f5 :
            (shared {a : Int; b : [Int]; c : {f : Int}; d : {#l : Int; #m : Int; #o : Int}} ->
              async {a : Nat; b : [Nat]; c : {f : Nat; g : Nat; h : Nat}; d : {#l : Nat}}) ->
                async ()
        };
        try {
          await this.send_f5(f5_1);
        }
        catch e { Prim.debugPrint "wrong_5_1"; }
      };


      // null, opt and any record fields, defaulting
      do {
        let this = actor (t) : actor {
          send_f6 :
            (shared () ->
              async {a : Null; b : ?None; c : Any}) ->
                async ()
        };
        try {
          await this.send_f6(f6_0);
        }
        catch e { Prim.debugPrint "wrong_6_0"; }
      };

   };

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
