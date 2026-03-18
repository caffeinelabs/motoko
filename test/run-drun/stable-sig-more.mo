//MOC-FLAG -A=M0194
// test pretty printing of stable types (compiler should fail if producing unparseable stable type signature
actor this {

 public shared func f0() : async () { loop {} };
 stable let x0 = f0;

 public shared func f1(_ : Nat) : async Nat { loop {} };
 stable let x1 = f1;

 public shared func f2(_ : Nat, _ : Bool) : async (Nat, Bool) { loop {} };
 stable let x2 = f2;

 public shared func f3(_ : shared () -> async ()) : async (shared () -> async ()) { loop {} };
 stable let x3 = f3;

 public shared func f4(_ : shared () -> async ()) : async (shared () -> async ()) { loop {} };
 stable let x4 = f4;

 public shared func f5(_ : actor {}) : async (actor {}) { loop {} };
 stable let x5 = f5;

 public shared func f6(_ : shared () -> async actor {}) : async (shared () -> async actor {}) { loop {} };
 stable let x6 = f6;

 stable let v1 : Nat = 0;
 stable let v2 : Nat8 = 0;
 stable let v3 : Nat16 = 0;
 stable let v4 : Nat32 = 0;
 stable let v5 : Int = 0;
 stable let v6 : Int8 = 0;
 stable let v7 : Int16 = 0;
 stable let v8 : Int32 = 0;

 stable let p : ?Principal = null;

 stable let v9 : Char = 'a';
 stable let v10 : Text = "hello";
 stable let v11 : Blob = "hello";

 stable let n : Null = null;

 stable let o1 : ?None = null;
 stable let o2 : ??None = null;
 stable let o3 : ??(None,Any) = null;
 stable let o4 : ?(actor {}) = null;
 stable let o5 : ?{} = null;
 stable let o6 : ?(shared ()->()) = null;
 stable let o7 : ?(shared ()-> async ()) = null;
 stable let o8 : ?(shared ?()-> async ?()) = null;

 stable let t1 : () = ();
 stable let t2 : (Nat,) = (0,);
 stable let t3 : (Nat, Bool) = (0, true);
 stable let t4 : (Nat, Bool, Text) = (0, true, "oo");

 stable let a0 : [None] = [];
 stable let a1 : [Nat] = [0];
 stable let a2 : [?Nat] = [?0];

 stable let m0 : [var None] = [var];
 stable let m1 : [var Nat] = [var 0];
 stable let m2 : [var ?Nat] = [var null];
 stable let m3 : [var ?(actor{})] = [var null];

 stable let r1 : {} = {};
 stable let r2 : {a : Nat} = { a = 0 };
 stable let r3 : {a : Nat; b : Bool} = {a = 0; b = true};
 stable let r4 : {a : Nat; b : Bool; c : Text} = {a = 0; b = true; c = ""};
 stable let r5 : {a : Nat; b : Bool; c : Text; var d : Nat} = {a = 0; b = true; c = ""; var d = 0};

 stable let d1 : ?{#} = null;
 stable let d2 : {#a : Nat} = #a 0;
 stable let d3 : {#a : Nat; #b : Bool} = #a 0;
 stable let d4 : {#a : Nat; #b : Bool; #c} = #a 0;

}
