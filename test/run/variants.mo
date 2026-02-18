//MOC-FLAG -A=M0194
import Prim "mo:⛔";

type Tree<A> = {#leaf : A; #branch : (Tree<A>, Tree<A>)};

func size<A>(t : Tree<A>) : Nat {
  switch t {
  case (#leaf _) 1;
  case (#branch(t1, t2)) { 1 + size<A>(t1) + size<A>(t2) };
  }
};

let tt1 : Tree<Int> = #branch(#leaf 1, #leaf (-1));
let tt2 = #leaf ""; // infers type {#leaf : Text} which is a subtype of Tree<Text>

Prim.debugPrintNat(size<Int>(tt1));
Prim.debugPrintNat(size<Text>(tt2));


// subtyping

type Super = {#c : Int; #b : Char; #a : Nat};
type Sub = {#c : Nat; #a : Nat};

let ts1 : Sub = #c 25;
func ts2(v : Super) { ignore v };

let ts3 = ts2 ts1;

// type syntax variations

type A = { # };           // empty variant
type B = { #foo : Int };  // singleton variant
type C = { #foo : Int; };
type D = { #foo : Int; #bar : Char };
type E = { #foo : Int; #bar : Char; };
type F = { #foo : Int; #bar : Char; #daz : Bool };

// lightweight (enumeration-like) variants

type Weekday = { #Monday; #Tuesday; #Wednesday; #Thursday; #Friday; #Saturday; #Sunday };

func sayIcelandic (day : Weekday) : Text = switch day {
  case (#Monday) "Mánudagur";
  case (#Tuesday) "Þriðjudagur";
  case (#Wednesday) "Miðvikudagur";
  case (#Thursday) "Fimmtudagur";
  case (#Friday) "Föstudagur";
  case (#Saturday) "Laugardagur";
  case (#Sunday) "Sunnudagur"
};

assert (sayIcelandic (#Wednesday) == "Miðvikudagur");

assert (debug_show (#foo (#bar)) == "#foo(#bar)");

assert ([#Monday, #Tuesday, #Wednesday, #Thursday, #Friday, #Saturday, #Sunday].size() == 7);

// check compilation of pattern matching on constant variants
let (#const a) = #const 42;

assert a == 42;
