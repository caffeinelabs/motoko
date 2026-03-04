//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// test candid subtype test with optional record extension
actor this {

   public func send_f0(
     f : shared {a : Int} -> async {b : Bool})
     : async () {
     Prim.debugPrint("ok_0");
   };

   public func f0() : async () {  };

   public func f0_1_a({a : Int; n : Null}) : async {b : Bool; x : Null} { {b = true; x = null} };
   public func f0_1_b({a : Int; n : ?Null}) : async {b : Bool; x : ?Null} { {b = true; x = null} };
   public func f0_1_c({a : Int; n : Any}) : async {b : Bool; x : Any} { {b = true; x = null } };


   public func go() : async () {
      let t = debug_show (Prim.principalOfActor(this));

      // appending Null ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared {a : Int; n : Null } -> async {b : Bool; x : Null}) -> async ();
        };
        try {
          await this.send_f0(f0_1_a);
          Prim.debugPrint "ok_0_1_a";
        }
        catch e {
          Prim.debugPrint "wrong_0_1_a";
        }
      };

      // appending ?_ ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared {a : Int; n : ?Null} -> async {b : Bool; x : ?Null}) -> async ();
        };
        try {
          await this.send_f0(f0_1_b);
          Prim.debugPrint "ok_0_1_b";

        }
        catch e {
          Prim.debugPrint "wrong_0_1_b";
        }
      };

      // appending Any ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared {a : Int; n : Any} -> async {b : Bool; x : Any}) -> async ();
        };
        try {
          await this.send_f0(f0_1_c);
          Prim.debugPrint "ok_0_1_c";
        }
        catch e {
          Prim.debugPrint "wrong_0_1_c";
        }
      };

   };

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
