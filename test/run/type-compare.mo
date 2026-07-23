// compare_typ branches via LUB/GLB (if-then-else result types): Any/Non,
// Opt/Null, Variant, Obj, Mut, Tup, every Prim scalar, Con, Opt.

let _any : Any = if true (42 : Any) else (42 : Any);
let _non_opt : ?Int = null;
let _t : ?Nat = if true ?1 else null;

type V = {#a : Nat; #b : Text; #c : Bool};
func lub_var(b : Bool) : V = if b (#a 1) else (#b "x");

type R1 = {x : Nat; y : Text};
func lub_rec(b : Bool) : R1 = if b ({x = 1; y = "a"} : R1) else ({x = 2; y = "b"} : R1);

var _mu_nat : Nat = 0;

func lub_tup(b : Bool) : (Nat, Text) = if b (1, "a") else (2, "b");

let _nat  : Nat   = if true 1 else 2;
let _int  : Int   = if true 1 else 2;
let _bool : Bool  = if true true else false;
let _text : Text  = if true "a" else "b";
let _char : Char  = if true 'a' else 'b';
let _blob : Blob  = if true ("a" : Blob) else ("b" : Blob);
let _fl   : Float = if true 1.0 else 2.0;
let _n8   : Nat8  = if true (1 : Nat8) else (2 : Nat8);
let _n16  : Nat16 = if true (1 : Nat16) else (2 : Nat16);
let _n32  : Nat32 = if true (1 : Nat32) else (2 : Nat32);
let _n64  : Nat64 = if true (1 : Nat64) else (2 : Nat64);
let _i8   : Int8  = if true (1 : Int8) else (2 : Int8);
let _i16  : Int16 = if true (1 : Int16) else (2 : Int16);
let _i32  : Int32 = if true (1 : Int32) else (2 : Int32);
let _i64  : Int64 = if true (1 : Int64) else (2 : Int64);

type Pair<T> = (T, T);
let _pn : Pair<Nat> = (1, 2);
let _pt : Pair<Text> = ("a", "b");

let _opt_n : ?Nat  = if true ?(1 : Nat) else ?(2 : Nat);
let _opt_t : ?Text = if true ?("a") else ?("b");

assert (lub_tup(true) == (1, "a"));
assert (lub_tup(false) == (2, "b"));
assert (lub_rec(true).x == 1);
assert ((lub_var(true) : V) == #a 1);
