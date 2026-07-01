// Exercise inhabited_typ: various type shapes trigger inhabitation checks in coverage.
// inhabited_typ branches: Tup, Obj, Variant, Con(Def), Con(Abs)

// Tup arm: tuple pattern — List.for_all on ts in inhabited_typ
func check_tup(p : (Nat, Bool)) : Nat {
  switch p { case (n, _) n }
};

// Obj arm: object pattern — List.for_all on tfs in inhabited_typ
func check_obj(o : {x : Nat; y : Text}) : Nat {
  switch o { case {x; y = _} x }
};

// Variant arm: variant pattern — List.exists on tfs in inhabited_typ
type Color = {#red; #green; #blue};
func check_variant(c : Color) : Text {
  switch c {
    case (#red) "red";
    case (#green) "green";
    case (#blue) "blue"
  }
};

// Con(Def) arm: type alias resolved via open_ in inhabited_typ
type MyNat = Nat;
func check_con(n : MyNat) : MyNat {
  switch n { case _ n }
};

// Con(Abs) arm: type parameter — inhabited_typ co t' via Abs branch.
// A type alias of a Con (Abs) kind is checked by inhabited_typ via Con(Abs) arm.
type Id<T> = T;

// Variant with only one tag
type Unit_ = {#unit};
func check_unit_var(u : Unit_) : Bool {
  switch u { case (#unit) true }
};

// Wrapper variant with payload
type Wrapper = {#some : Nat; #none};
func check_wrapper(w : Wrapper) : Nat {
  switch w {
    case (#some n) n;
    case (#none) 0
  }
};

// Nested tuple — exercises List.for_all recursively in inhabited_typ Tup arm
func check_nested(p : (Nat, (Bool, Text))) : Nat {
  switch p { case (n, (_, _)) n }
};

// Obj with multiple fields — exercises List.for_all (inhabited_field) in Obj arm
func check_obj2(o : {a : Nat; b : Bool; c : Text}) : Bool {
  switch o { case {a = _; b; c = _} b }
};

assert (check_tup((7, true)) == 7);
assert (check_obj({x = 5; y = "hi"}) == 5);
assert (check_variant(#red) == "red");
assert (check_con(99) == 99);
assert (check_unit_var(#unit) == true);
assert (check_wrapper(#some 3) == 3);
assert (check_nested((1, (true, "x"))) == 1);
assert (check_obj2({a = 1; b = false; c = "z"}) == false);
