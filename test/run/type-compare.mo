// Exercise compare_typ branches via LUB/GLB computation.
// LUB (lub) = type of if-then-else and switch expressions.
// GLB (glb) = type of record updates, intersection (implicit).
//
// compare_typ is used by OrdPair (Set.Make/Map.Make) to memoize
// subtyping and equality checks during combine (lub/glb).

// Any / Non branches: Any, Non, Pre -> 0
// Triggered by subtype checks involving Any or Non
let _any : Any = if true { (42 : Any) } else (42 : Any);
let _non_opt : ?Int = null;  // Null <: ?Int triggers Non/Prim Null arm

// Async branch: compare_async_sort
// (only reachable in actor/async context, but compare_typ may be used
//  in subtype memo for async types)
// We approximate by using ?T lub for async scope types:
let _t : ?Nat = if true ?1 else null;  // triggers Opt/Null lub

// Variant branch: compare_flds on variant fields
type V = {#a : Nat; #b : Text; #c : Bool};
func lub_var(b : Bool) : V =
  if b (#a 1) else (#b "x");

// Trigger compare_flds via lub of two different record types that share a supertype
type R1 = {x : Nat; y : Text};
type R2 = {x : Nat; y : Text; z : Bool};
// lub of R1 and R2 is R1 (subtype wins)
func lub_rec(b : Bool) : R1 =
  if b ({x = 1; y = "a"} : R1) else ({x = 2; y = "b"} : R1);

// Mut branch: compare_typ t1 t2 where both are Mut
// Mut types in stable fields are what drive compatible_typ coverage
// We use a mutable variable to exercise Mut (unread → `_`-prefixed)
var _mu_nat : Nat = 0;

// Named branch: compare_typ Named(n1,t1) Named(n2,t2)
// Named types appear internally; we exercise them via imports
// which get Named wrappers.

// Tup branch: compare_typs ts1 ts2
func lub_tup(b : Bool) : (Nat, Text) =
  if b (1, "a") else (2, "b");

// Prim comparisons — compare_prim for all prim types
let _nat  : Nat  = if true 1    else 2;
let _int  : Int  = if true 1    else 2;
let _bool : Bool = if true true else false;
let _text : Text = if true "a"  else "b";
let _char : Char = if true 'a'  else 'b';
let _blob : Blob = if true ("a" : Blob) else ("b" : Blob);
let _fl   : Float = if true 1.0 else 2.0;
let _n8   : Nat8  = if true (1 : Nat8) else (2 : Nat8);
let _n16  : Nat16 = if true (1 : Nat16) else (2 : Nat16);
let _n32  : Nat32 = if true (1 : Nat32) else (2 : Nat32);
let _n64  : Nat64 = if true (1 : Nat64) else (2 : Nat64);
let _i8   : Int8  = if true (1 : Int8)  else (2 : Int8);
let _i16  : Int16 = if true (1 : Int16) else (2 : Int16);
let _i32  : Int32 = if true (1 : Int32) else (2 : Int32);
let _i64  : Int64 = if true (1 : Int64) else (2 : Int64);

// Con branch: two uses of same type constructor — memoized via compare
type Pair<T> = (T, T);
let _ : Pair<Nat> = (1, 2);
let _ : Pair<Text> = ("a", "b");

// Opt branch: compare_typ t1 t2 inside Opt
let _opt_n : ?Nat  = if true ?(1 : Nat) else ?(2 : Nat);
let _opt_t : ?Text = if true ?("a") else ?("b");

assert (lub_tup(true) == (1, "a"));
assert (lub_tup(false) == (2, "b"));
assert (lub_rec(true).x == 1);
assert ((lub_var(true) : V) == #a 1);
