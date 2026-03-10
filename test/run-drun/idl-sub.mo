//MOC-FLAG -A=M0194
import Prim "mo:⛔";

actor this {

   public func f0() : async () {};

   public func send_f0(
     f : shared () -> async ()
   ) : async () {
     Prim.debugPrint("ok 0");
   };

   public func f1(n:Nat) : async () {};

   public func send_f1(
     f1 : shared (n:Nat) -> async ()
   ) : async () {
     Prim.debugPrint("ok 1");
   };

   public func f2(n:Nat) : async Nat { 0 };

   public func send_f2(
     f2 : shared (n:Nat) -> async Nat
   ) : async () {
     Prim.debugPrint("ok 2");
   };


   // variants
   public func f3(n:{#f:Nat}) : async {#f:Nat} { (#f 0) };

   public func send_f3(
     f3 : shared (n:{#f:Nat}) -> async {#f:Nat}
   ) : async () {
     Prim.debugPrint("ok 3");
   };


   // records
   public func f4(n:{f:Nat}) : async {f:Nat} { { f = 0 } };

   public func send_f4(
     f4 : shared (n: {f : Nat}) -> async {f : Nat}
   ) : async () {
     Prim.debugPrint("ok 4");
   };


   // option
   public func f5(n : ?Nat) : async ?Nat { null };

   public func send_f5(
     f5 : shared (n : ?Nat) -> async ?Nat
   ) : async () {
     Prim.debugPrint("ok 5");
   };

   // actor
   public func f6(n : actor {}) : async (actor {}) { n };

   public func send_f6(
     f6 : shared (n : actor {}) -> async (actor {})
   ) : async () {
     Prim.debugPrint("ok 6");
   };

   public func f7(n : Nat, b : Bool) : async (Nat,Bool) {(n,b)};

   public func send_f7(
     f7 : shared (Nat, Bool) -> async (Nat, Bool)
   ) : async () {
     Prim.debugPrint("ok 7");
   };


   // record with 2 fields
   public func f8({n : Nat; b : Bool}) : async {n : Nat; b : Bool} {{ n; b}};

   public func send_f8(
     f8 : shared {n : Nat; b : Bool} -> async {n : Nat; b : Bool}
   ) : async () {
     Prim.debugPrint("ok 8");
   };

   // arrays
   public func f9(n:[Nat]) : async [Nat] { n };

   public func send_f9(
     f9 : shared (n:[Nat]) -> async [Nat]
   ) : async () {
     Prim.debugPrint("ok 9");
   };

   // blob
   public func f10(n:Blob) : async Blob { n };

   public func send_f10(
     f10 : shared (n:Blob) -> async Blob
   ) : async () {
     Prim.debugPrint("ok 10");
   };


   // record with opt field
   public func f11({n : Nat; o : ?Bool}) : async {n : Nat; o : ?Bool} {{ n; o}};

   public func send_f11(
     f11 : shared {n : Nat; o : ?Bool} -> async {n : Nat; o : ?Bool}
   ) : async () {
     Prim.debugPrint("ok 11");
   };

   // record with opt field record
   public func f12({n : Nat; o : ?{b:Bool}}) : async {n : Nat; o : ?{b:Bool}} {{ n; o}};

   public func send_f12(
     f12 : shared {n : Nat; o : ?{b:Bool}} -> async {n : Nat; o : ?{b:Bool}}
   ) : async () {
     Prim.debugPrint("ok 12");
   };

   // Principals
   public func f13(n:Principal) : async Principal { n };

   public func send_f13(
     f13 : shared (n:Principal) -> async Principal
   ) : async () {
     Prim.debugPrint("ok 13");
   };

   type Actor = actor {
      f : () -> async ();
      p : () -> ();
      q : shared query  () ->  async ();
 //     r : shared   () -> async ();
   };

   // non-triv actor
   public func f14(n : Actor) : async Actor { n };

   public func send_f14(
     f14 : shared (n : Actor) -> async Actor
   ) : async () {
     Prim.debugPrint("ok 14");
   };

   // non-triv actor
   public query func f15() : async () {  };

   public func send_f15(
     f15 : query () -> async ()
   ) : async () {
     Prim.debugPrint("ok 15");
   };


   public func go() : async () {
      await this.send_f0(f0);
      await this.send_f1(f1);
      await this.send_f2(f2);
      await this.send_f3(f3);
      await this.send_f4(f4);
      await this.send_f5(f5);
      await this.send_f6(f6);
      await this.send_f7(f7);
      await this.send_f8(f8);
      await this.send_f9(f9);
      await this.send_f10(f10);
      await this.send_f11(f11);
      await this.send_f12(f12);
      await this.send_f13(f13);
      await this.send_f14(f14);
      await this.send_f15(f15);
   };


}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
