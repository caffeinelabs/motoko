// Real-world example: deriving `compare` for records AND tuples using structural synthesis.
//MOC-FLAG --package core $MOTOKO_CORE
//MOC-FLAG -W=M0223,M0236,M0237

import Array "mo:core/Array";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import Order "mo:core/Order";

// __record combiner in a module (top-level name would clash with __tuple combiner below)
// Thunks enable genuine short-circuiting: remaining fields are never evaluated.
module RecordCmp {
  public func compare(__record : [(Text, () -> Order.Order)]) : Order.Order {
    for ((_, ordThunk) in __record.vals()) {
      let ord = ordThunk();
      if (ord != #equal) return ord;
    };
    #equal;
  };
};

// __tuple combiner: same fold, but receives [() -> Order] without field names
module TupleCmp {
  public func compare(__tuple : [() -> Order.Order]) : Order.Order {
    for (ordThunk in __tuple.vals()) {
      let ord = ordThunk();
      if (ord != #equal) return ord;
    };
    #equal;
  };
};

// Generic comparison entry point — drives implicit derivation for any R
func cmp<R>(x : R, y : R, compare : (implicit : (R, R) -> Order.Order)) : Order.Order = compare(x, y);

// ── Simple two-field record ───────────────────────────────────────────────────

// Fields are sorted lexicographically in Motoko records: age before name.
// Per-field implicits resolved: Nat.compare for age, Text.compare for name.
type Person = { name : Text; age : Nat };

let alice : Person = { name = "Alice"; age = 30 };
let bob : Person = { name = "Bob"; age = 25 };
let carol : Person = { name = "Carol"; age = 30 };
let alice2 : Person = { name = "Alice"; age = 30 };

// age: 30 > 25 → #greater
assert cmp(alice, bob) == #greater;
// same age and name → #equal
assert cmp(alice, alice2) == #equal;
// age: 25 < 30 → #less
assert cmp(bob, alice) == #less;
// age tied at 30; name: "Alice" < "Carol" → #less
assert cmp(alice, carol) == #less;

// Array.sort uses (implicit : (T, T) -> Order.Order) — derived from __record (binary path)
let people : [Person] = [carol, alice, bob];
let sorted = people.sort();
// sorted by age first: bob(25), then alice(30) before carol(30) by name
assert sorted[0].name == "Bob";
assert sorted[1].name == "Alice";
assert sorted[2].name == "Carol";

// ── Derivation inside a module body (ObjBlockE) ──────────────────────────────

do {
  module PersonOps {
    public func sortPeople(people : [Person]) : [Person] = people.sort();
    public func cmpPeople(a : Person, b : Person) : Order.Order = cmp(a, b);
  };

  assert PersonOps.cmpPeople(alice, bob) == #greater;
  assert PersonOps.cmpPeople(alice, alice2) == #equal;
  let module_sorted = PersonOps.sortPeople(people);
  assert module_sorted[0].name == "Bob";
  assert module_sorted[1].name == "Alice";
  assert module_sorted[2].name == "Carol";
};

// ── Nested record: two levels of structural derivation ────────────────────────

// Fields sorted: lead before size.
// compare for Team is derived by: resolving (Person, Person) → structurally derived
// from __record binary (depth+1), then resolving (Nat, Nat) → Nat.compare.
type Team = { lead : Person; size : Nat };

let t1 : Team = { lead = alice; size = 5 };
let t2 : Team = { lead = bob; size = 10 };
let t3 : Team = { lead = alice; size = 3 };

// lead field compared first: alice(30) > bob(25) → #greater, regardless of size
assert cmp(t1, t2) == #greater;
// same lead; size: 5 > 3 → #greater
assert cmp(t1, t3) == #greater;
// fully equal
assert cmp(t1, { lead = alice; size = 5 }) == #equal;

// Sorting teams
let teams : [Team] = [t1, t2, t3];
let sorted_teams = teams.sort();
// lead age: bob(25) < alice(30), so t2 first
// among alice-led teams: size 3 < 5, so t3 before t1
assert sorted_teams[0].lead.name == "Bob";
assert sorted_teams[1].size == 3;
assert sorted_teams[2].size == 5;

// ── Tuple comparison: __tuple covers (T, T) -> Order binary holes ─────────────

// (Nat, Text) tuples compared positionally: element 0 (Nat), then element 1 (Text).
// Per-element implicits resolved: Nat.compare for pos 0, Text.compare for pos 1.
assert cmp((1 : Nat, "Alice"), (2 : Nat, "Bob")) == #less; // Nat 1 < 2
assert cmp((1 : Nat, "Alice"), (1 : Nat, "Bob")) == #less; // Nat equal; "Alice" < "Bob"
assert cmp((1 : Nat, "Alice"), (1 : Nat, "Alice")) == #equal;
assert cmp((2 : Nat, "Bob"), (1 : Nat, "Alice")) == #greater; // Nat 2 > 1

// Sorting an array of (Nat, Text) tuples
let pairs : [(Nat, Text)] = [(2 : Nat, "B"), (1 : Nat, "Z"), (1 : Nat, "A")];
let sorted_pairs = pairs.sort();
// (1,"A") < (1,"Z") < (2,"B")
assert sorted_pairs[0] == (1 : Nat, "A");
assert sorted_pairs[1] == (1 : Nat, "Z");
assert sorted_pairs[2] == (2 : Nat, "B");

// Nested: Team with a (Nat, Text) tuple field — mixes record and tuple derivation
type TaggedTeam = { key : (Nat, Text); size : Nat };
let tt1 : TaggedTeam = { key = (1 : Nat, "A"); size = 5 };
let tt2 : TaggedTeam = { key = (2 : Nat, "B"); size = 3 };
let tt3 : TaggedTeam = { key = (1 : Nat, "A"); size = 3 };
// key field compared first (record binary); key uses tuple binary internally
assert cmp(tt1, tt2) == #less; // key: (1,"A") < (2,"B")
assert cmp(tt1, tt3) == #greater; // same key; size: 5 > 3
assert cmp(tt1, { key = (1 : Nat, "A"); size = 5 }) == #equal;
