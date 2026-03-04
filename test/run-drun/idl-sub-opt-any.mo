//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// test candid subtype test with optional argument extension
actor this {

   public func send_f0(
     f : shared (a : Int) -> async Bool)
     : async () {
     Prim.debugPrint("ok_0");
   };

   public func f0() : async () {  };

   public func f0_1_a(a : Int, n : Null) : async (Bool, Null) { (true, null) };
   public func f0_1_b(a : Int, n : ?Null) : async (Bool, ?Null) { (true, null) };
   public func f0_1_c(a : Int, n : Any) : async (Bool, Any) { (true, null) };
   public func f0_1_d(n : ?Null, a : Int) : async (?Null, Bool) { (null, true) };
   public func f0_1_e(n : Any, a : Int) : async (Any, Bool) { (null, true) };


   public func go() : async () {
      let t = debug_show (Prim.principalOfActor(this));

      // appending Null ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (a : Int, n : Null) -> async (Bool, Null)) -> async ();
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
          send_f0 : (shared (a : Int, n : ?Null) -> async (Bool, ?Null)) -> async ();
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
          send_f0 : (shared (a : Int, n : Any) -> async (Bool, Any)) -> async ();
        };
        try {
          await this.send_f0(f0_1_c);
          Prim.debugPrint "ok_0_1_c";
        }
        catch e {
          Prim.debugPrint "wrong_0_1_c";
        }
      };

      // prepending Any not ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n : ?Null, a : Int) -> async (?Null, Bool)) -> async ();
        };
        try {
          await this.send_f0(f0_1_d);
          Prim.debugPrint "wrong_0_1_d";
        }
        catch e {
          Prim.debugPrint "ok_0_1_d";
        }
      };

      // prepending ?_ not ok
      do {
        let this = actor (t) : actor {
          send_f0 : (shared (n : Any, a : Int) -> async (Any, Bool)) -> async ();
        };
        try {
          await this.send_f0(f0_1_e);
          Prim.debugPrint "wrong_0_1_e";

        }
        catch e {
          Prim.debugPrint "ok_0_1_e";
        }
      };

   };

}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
