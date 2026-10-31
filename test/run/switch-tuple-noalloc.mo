import Prim "mo:⛔";

// Switching on a tuple, or projecting from one, should not allocate the
// tuple on the heap when it is not needed as a whole (#6462).

func pair(x : Nat) : (Nat, Nat) = (x, x + 1);

func triple(x : Nat) : (Nat, Nat, Nat) = (x, x + 1, x + 2);

func switch_lit(a : Nat, b : Nat) : Nat {
  switch (a, b) {
    case (0, _) { 10 };
    case (_, 0) { 20 };
    case (x, y) { x + y };
  }
};

func switch_call(x : Nat) : Nat {
  switch (pair(x)) {
    case (0, y) { y };
    case (x, y) { x * y };
  }
};

func switch_triple_wild(x : Nat) : Nat {
  switch (triple(x)) {
    case (_, y, _) { y };
  }
};

func proj_first(x : Nat) : Nat = pair(x).0;

func proj_last(x : Nat) : Nat = pair(x).1;

func proj_middle(x : Nat) : Nat = triple(x).1;

// Controls: these still allocate, since a case binds the whole tuple, uses
// an or-pattern, or the scrutinee contains a nested tuple. They must still
// work.
func switch_whole(a : Nat, b : Nat) : (Nat, Nat) {
  switch (a, b) {
    case (0, _) { (0, 0) };
    case t { t };
  }
};

func switch_or(a : Nat, b : Nat) : Nat {
  switch (a, b) {
    case ((0, x) or (x, 0)) { x };
    case _ { 42 };
  }
};

func switch_nested(a : Nat, b : Nat, c : Nat) : Nat {
  switch (a, (b, c)) {
    case (0, (y, z)) { y + z };
    case (x, (0, _)) { x };
    case (x, _) { x + 1 };
  }
};

func switch_nested_call(a : Nat, b : Nat) : Nat {
  switch (a, pair(b)) {
    case (0, (y, z)) { y + z };
    case (x, (_, z)) { x + z };
  }
};

func no_alloc(name : Text, f : Nat -> Nat, arg : Nat, expected : Nat) {
  ignore f(arg); // warm-up
  let a0 = Prim.rts_total_allocation();
  let r = f(arg);
  let a1 = Prim.rts_total_allocation();
  if (r != expected) {
    Prim.debugPrint(name # ": unexpected result " # debug_show r);
    assert false;
  };
  if (a1 != a0) {
    Prim.debugPrint(name # ": unexpected allocation " # debug_show (a1 - a0 : Nat));
    assert false;
  };
};

no_alloc("switch_lit 0", func x = switch_lit(0, x), 5, 10);
no_alloc("switch_lit 1", func x = switch_lit(x, 0), 5, 20);
no_alloc("switch_lit 2", func x = switch_lit(x, 3), 5, 8);
no_alloc("switch_call 0", switch_call, 0, 1);
no_alloc("switch_call 1", switch_call, 3, 12);
no_alloc("switch_triple_wild", switch_triple_wild, 3, 4);
no_alloc("proj_first", proj_first, 3, 3);
no_alloc("proj_last", proj_last, 3, 4);
no_alloc("proj_middle", proj_middle, 3, 4);

// The control cases still work
assert (switch_whole(0, 1) == (0, 0));
assert (switch_whole(2, 1) == (2, 1));
assert (switch_or(0, 7) == 7);
assert (switch_or(7, 0) == 7);
assert (switch_or(7, 1) == 42);
assert (switch_nested(0, 3, 2) == 5);
assert (switch_nested(3, 0, 2) == 3);
assert (switch_nested(3, 1, 2) == 4);
assert (switch_nested_call(0, 3) == 7);
assert (switch_nested_call(3, 1) == 5);

// Evaluation order of the scrutinee and the projected tuple is preserved
var log = "";
func t(s : Text, n : Nat) : Nat { log #= s; n };

switch (t("a", 1), t("b", 2), t("c", 3)) {
  case (1, 2, 3) { log #= "!" };
  case _ { assert false };
};
assert (log == "abc!");

log := "";
switch (t("a", 1), (t("b", 2), t("c", 3))) {
  case (_, (2, _)) { log #= "!" };
  case _ { assert false };
};
assert (log == "abc!");

log := "";
assert ((t("a", 1), t("b", 2), t("c", 3)).1 == 2);
assert (log == "abc");

// First matching case wins, also with refutable later components
func first_match(a : Nat, b : Nat) : Text {
  switch (a, b) {
    case (1, 1) { "both" };
    case (1, _) { "first" };
    case (_, 1) { "second" };
    case (_, _) { "none" };
  }
};
assert (first_match(1, 1) == "both");
assert (first_match(1, 2) == "first");
assert (first_match(2, 1) == "second");
assert (first_match(2, 2) == "none");
