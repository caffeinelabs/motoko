//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/comparators
import Prim "mo:⛔";

type Order = { #less; #equal; #greater };

func ascending(x : Nat64, y : Nat64) : Order {
  if (x < y) #less else if (x == y) #equal else #greater
};
func integers(x : Int, y : Int) : Order {
  if (x < y) #less else if (x == y) #equal else #greater
};
func invoke<T>(compare : (T, T) -> Order, x : T, y : T) : Int {
  Prim.fixtureAutoCompare<T>(compare, x, y)
};

// These canonical-looking Text/Blob wrappers reference later declarations.
// An unavailable compile-time proof must silently preserve the callback.
func forwardText(x : Text, y : Text) : Order {
  let c = laterText(x, y);
  if (c < 0) #less else if (c == 0) #equal else #greater
};
func forwardBlob(x : Blob, y : Blob) : Order {
  let c = laterBlob(x, y);
  if (c < 0) #less else if (c == 0) #equal else #greater
};
func laterText(x : Text, y : Text) : Int8 { Prim.textCompare(x, y) };
func laterBlob(x : Blob, y : Blob) : Int8 { Prim.blobCompare(x, y) };
assert invoke<Text>(forwardText, "a", "b") == -1;
assert invoke<Blob>(forwardBlob, "\ff", "\00") == 1;

// Neither familiar names nor familiar signatures prove comparator behavior.
let deceptive = module {
  public func compare(x : Nat64, y : Nat64) : Order {
    if (x < y) #greater else if (x == y) #equal else #less
  };
};
func reversedOperands(x : Nat64, y : Nat64) : Order {
  if (y < x) #less else if (y == x) #equal else #greater
};
func allEquivalent(_x : Nat64, _y : Nat64) : Order { #equal };
assert invoke<Nat64>(deceptive.compare, 1, 9) == 1;
assert invoke<Nat64>(reversedOperands, 1, 9) == 1;
assert invoke<Nat64>(allEquivalent, 1, 9) == 0;

var effects = 0;
func counted(x : Nat64, y : Nat64) : Order {
  effects += 1;
  if (x < y) #less else if (x == y) #equal else #greater
};
assert invoke<Nat64>(counted, 1, 9) == -1;
assert effects == 1;
assert Prim.fixtureAutoComparePair<Nat64>(counted, 9, 1) == (1, 9);
assert effects == 2;
Prim.fixtureAutoCompareUnit<Nat64>(counted, 3, 3);
assert effects == 3;
assert Prim.fixtureNestedCompare<Nat64>(counted, 9, 1) == 1;
assert effects == 4;

func captured(direction : Int, increment : Nat) : (Nat64, Nat64) -> Order {
  func (x : Nat64, y : Nat64) : Order {
    effects += increment;
    if (direction < 0) reversedOperands(x, y) else ascending(x, y)
  }
};
assert invoke<Nat64>(captured(-1, 7), 1, 9) == 1;
assert effects == 11;
assert invoke<Nat64>(captured(1, 3), 1, 9) == -1;
assert effects == 14;

// The same call site sees standard and custom closures through fields and
// mutable variables. A previous fast call must not change later dispatches.
let aliases = { normal = ascending; reverse = deceptive.compare };
var selected : (Nat64, Nat64) -> Order = aliases.normal;
assert invoke<Nat64>(selected, 0, 18_446_744_073_709_551_615) == -1;
selected := aliases.reverse;
assert invoke<Nat64>(selected, 0, 18_446_744_073_709_551_615) == 1;
selected := aliases.normal;
assert invoke<Nat64>(selected, 18_446_744_073_709_551_615, 0) == 1;

// Comparator arguments are contravariant: Int.compare can order Nat keys.
assert invoke<Nat>(integers, 0, 18_446_744_073_709_551_616) == -1;
assert invoke<Int>(integers, -18_446_744_073_709_551_616, 0) == -1;

var events = "";
func logging(x : Nat64, y : Nat64) : Order {
  events #= "callback;";
  if (x < y) #less else if (x == y) #equal else #greater
};
let intrinsic = Prim.fixtureAutoCompare;
assert (do { events #= "callee;"; intrinsic })<Nat64>(
  do { events #= "comparator;"; logging },
  do { events #= "left;"; 3 },
  do { events #= "right;"; 5 }
) == -1;
assert events == "callee;comparator;left;right;callback;";
events := "";
assert intrinsic<Nat64>(
  do { events #= "comparator;"; ascending },
  do { events #= "left;"; 3 },
  do { events #= "right;"; 5 }
) == -1;
assert events == "comparator;left;right;";

// Nullable values and reference objects keep the ordinary callback path.
func optional(x : ?Nat, y : ?Nat) : Order {
  effects += 1;
  switch (x, y) {
    case (null, null) #equal;
    case (null, _) #less;
    case (_, null) #greater;
    case (?a, ?b) { if (a < b) #less else if (a == b) #equal else #greater };
  }
};
assert invoke<?Nat>(optional, null, ?0) == -1;
assert invoke<?Nat>(optional, ?0, null) == 1;
assert effects == 16;
type Cell = { var key : Nat };
let left = { var key = 4 };
let right = { var key = 8 };
func objects(a : Cell, b : Cell) : Order {
  a.key += 1;
  if (a.key < b.key) #less else if (a.key == b.key) #equal else #greater
};
assert invoke<Cell>(objects, left, right) == -1;
assert left.key == 5;
