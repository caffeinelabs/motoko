// Type predicates/relations: eq_con / eq_binds (type fields, generics, alias
// chains), inhabited_typ arms, and has_no_subtypes_or_supertypes (via bi_match).

class Container() {
  public type Elem = Nat;
  public var count : Nat = 0;
  public func add() { count += 1 };
};
let ctr = Container();
ctr.add(); ctr.add();
assert (ctr.count == 2);

module M { public type T = Nat; public let x : T = 42; };
type HasType = {type T = Nat; n : Nat};
func poly<A, B>(a : A, b : B) : (A, B) = (a, b);
let _pa = poly(1, "x");
let _pb = poly(true, 0.5);
type Alias1 = Nat;
type Alias2 = Alias1;
let _al : Alias2 = 42;
type Id<T> = T;
let _id : Id<Nat> = 7;

type Color = {#red; #green; #blue};
func check_variant(c : Color) : Text =
  switch c { case (#red) "red"; case (#green) "green"; case (#blue) "blue" };
type MyNat = Nat;
func check_con(n : MyNat) : MyNat = n;
type Unit_ = {#unit};
func check_unit(u : Unit_) : Bool = switch u { case (#unit) true };
type Wrapper = {#some : Nat; #none};
func check_wrapper(w : Wrapper) : Nat = switch w { case (#some n) n; case (#none) 0 };
func check_tup(p : (Nat, Bool)) : Nat = switch p { case (n, _) n };
func check_obj(o : {x : Nat; y : Text}) : Nat = switch o { case {x; y = _} x };
func check_nested(p : (Nat, (Bool, Text))) : Nat = switch p { case (n, (_, _)) n };
func check_obj2(o : {a : Nat; b : Bool; c : Text}) : Bool = switch o { case {a = _; b; c = _} b };
assert (check_variant(#red) == "red");
assert (check_con(99) == 99);
assert (check_unit(#unit) == true);
assert (check_wrapper(#some 3) == 3);
assert (check_tup((7, true)) == 7);
assert (check_obj({x = 5; y = "hi"}) == 5);
assert (check_nested((1, (true, "x"))) == 1);
assert (check_obj2({a = 1; b = false; c = "z"}) == false);

func upper<T>(_consume : T -> ()) : [var T] = [var];
func lower<T>(_produce : () -> T) : [var T] = [var];
let _u1 = upper(func (_ : Bool) {});
let _u2 = lower(func () : Bool = true);
let _u3 = upper(func (_ : Nat8) {});
let _u4 = lower(func () : Nat64 = 0);
let _u5 = upper(func (_ : Int8) {});
let _u6 = lower(func () : Int64 = 0);
let _u7 = upper(func (_ : Float) {});
let _u8 = lower(func () : Char = 'a');
let _u9 = upper(func (_ : Text) {});
let _u10 = lower(func () : Blob = "");
let _u11 = upper(func (_ : Nat) {});
let _u12 = lower(func () : Int = 0);
let _u13 = upper(func (_ : Null) {});
let _u14 = lower(func () : ?Nat8 = ?(0 : Nat8));
let _u15 = upper(func (_ : (Nat8, Bool)) {});
let _u16 = upper(func (_ : [Nat8]) {});
let _u17 = upper(func (_ : [var Nat8]) {});
let _u18 = upper(func (_ : Nat8 -> Bool) {});
let _u19 = lower(func () : Nat8 -> Bool = func (n : Nat8) : Bool = n == 0);
type MyNat8 = Nat8;
let _u20 = upper(func (_ : MyNat8) {});
