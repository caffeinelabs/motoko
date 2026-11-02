//MOC-FLAG -A=M0194
// test pretty printing of stable types (compiler should fail if producing unparseable stable type signature
actor this {

 public shared func f0() : async () { loop {} };
 let x0 = f0;

 public shared func f1(_ : Nat) : async Nat { loop {} };
 let x1 = f1;

 public shared func f2(_ : Nat, _ : Bool) : async (Nat, Bool) { loop {} };
 let x2 = f2;

 public shared func f3(_ : shared () -> async ()) : async (shared () -> async ()) { loop {} };
 let x3 = f3;

 public shared func f4(_ : shared () -> async ()) : async (shared () -> async ()) { loop {} };
 let x4 = f4;

 public shared func f5(_ : actor {}) : async (actor {}) { loop {} };
 let x5 = f5;

 public shared func f6(_ : shared () -> async actor {}) : async (shared () -> async actor {}) { loop {} };
 let x6 = f6;

 let v1 : Nat = 0;
 let v2 : Nat8 = 0;
 let v3 : Nat16 = 0;
 let v4 : Nat32 = 0;
 let v5 : Int = 0;
 let v6 : Int8 = 0;
 let v7 : Int16 = 0;
 let v8 : Int32 = 0;

 let p : ?Principal = null;

 let v9 : Char = 'a';
 let v10 : Text = "hello";
 let v11 : Blob = "hello";

 let n : Null = null;

 let o1 : ?None = null;
 let o2 : ??None = null;
 let o3 : ??(None,Any) = null;
 let o4 : ?(actor {}) = null;
 let o5 : ?{} = null;
 let o6 : ?(shared ()->()) = null;
 let o7 : ?(shared ()-> async ()) = null;
 let o8 : ?(shared ?()-> async ?()) = null;

 let t1 : () = ();
 let t2 : (Nat,) = (0,);
 let t3 : (Nat, Bool) = (0, true);
 let t4 : (Nat, Bool, Text) = (0, true, "oo");

 let a0 : [None] = [];
 let a1 : [Nat] = [0];
 let a2 : [?Nat] = [?0];

 let m0 : [var None] = [var];
 let m1 : [var Nat] = [var 0];
 let m2 : [var ?Nat] = [var null];
 let m3 : [var ?(actor{})] = [var null];

 let r1 : {} = {};
 let r2 : {a : Nat} = { a = 0 };
 let r3 : {a : Nat; b : Bool} = {a = 0; b = true};
 let r4 : {a : Nat; b : Bool; c : Text} = {a = 0; b = true; c = ""};
 let r5 : {a : Nat; b : Bool; c : Text; var d : Nat} = {a = 0; b = true; c = ""; var d = 0};

 let d1 : ?{#} = null;
 let d2 : {#a : Nat} = #a 0;
 let d3 : {#a : Nat; #b : Bool} = #a 0;
 let d4 : {#a : Nat; #b : Bool; #c} = #a 0;

}
