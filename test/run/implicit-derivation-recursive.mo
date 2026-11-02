//MOC-FLAG --package core $MOTOKO_CORE
import { Tuple2; Tuple4 } "mo:core/Tuples";
import Option "mo:core/Option";
import Nat "mo:core/Nat";
import { type Order } "mo:core/Order";
import Int "mo:core/Int";

// --- Single recursion: MyList = ?(Nat, MyList) ---

type MyList = ?(Nat, MyList);

func compareMyLists(
  a : MyList,
  b : MyList,
  compare : (implicit : (MyList, MyList) -> Order),
) : Order { compare(a, b) };

assert compareMyLists(null, null) == #equal;
assert compareMyLists(null, ?(1, null)) == #less;
assert compareMyLists(?(1, null), null) == #greater;
assert compareMyLists(?(1, null), ?(1, null)) == #equal;
assert compareMyLists(?(1, null), ?(2, null)) == #less;
assert compareMyLists(?(2, null), ?(1, null)) == #greater;

assert compareMyLists(
  ?(1, ?(2, ?(3, null))),
  ?(1, ?(2, ?(3, null))),
) == #equal;

assert compareMyLists(
  ?(1, ?(2, ?(3, null))),
  ?(1, ?(2, ?(4, null))),
) == #less;

assert compareMyLists(
  ?(1, ?(2, ?(3, null))),
  ?(1, ?(2, null)),
) == #greater;

// Deep nesting
assert compareMyLists(
  ?(1, ?(2, ?(3, ?(4, ?(5, ?(6, ?(7, ?(8, ?(9, ?(10, ?(11, ?(12, null)))))))))))),
  ?(1, ?(2, ?(3, ?(4, ?(5, ?(6, ?(7, ?(8, ?(9, ?(10, ?(11, ?(12, null)))))))))))),
) == #equal;

assert compareMyLists(
  ?(1, ?(2, ?(3, ?(4, ?(5, ?(6, ?(7, ?(8, ?(9, ?(10, ?(11, ?(12, null)))))))))))),
  ?(1, ?(2, ?(3, ?(4, ?(5, ?(6, ?(7, ?(8, ?(9, ?(10, ?(11, ?(99, null)))))))))))),
) == #less;

// --- Mutual recursion: A = ?(Nat, B), B = ?(Nat, A) ---

type A = ?(Nat, B);
type B = ?(Nat, A);

func compareAs(
  a1 : A,
  a2 : A,
  compare : (implicit : (A, A) -> Order),
) : Order { compare(a1, a2) };

assert compareAs(null, null) == #equal;
assert compareAs(null, ?(1, null)) == #less;
assert compareAs(?(1, null), ?(1, null)) == #equal;
assert compareAs(?(1, ?(2, null)), ?(1, ?(2, null))) == #equal;
assert compareAs(?(1, ?(2, null)), ?(1, ?(3, null))) == #less;

// Deep mutual recursion
assert compareAs(
  ?(1, ?(2, ?(3, ?(4, null)))),
  ?(1, ?(2, ?(3, ?(4, null)))),
) == #equal;

assert compareAs(
  ?(1, ?(2, ?(3, ?(4, null)))),
  ?(1, ?(2, ?(3, ?(5, null)))),
) == #less;

// --- Recursive type with multiple non-recursive fields ---

type Tree = ?(Nat, Nat, Tree, Tree);

func compareTrees(
  a : Tree,
  b : Tree,
  compare : (implicit : (Tree, Tree) -> Order),
) : Order { compare(a, b) };

assert compareTrees(null, null) == #equal;
assert compareTrees(null, ?(1, 2, null, null)) == #less;
assert compareTrees(?(1, 2, null, null), ?(1, 2, null, null)) == #equal;
assert compareTrees(?(1, 2, null, null), ?(1, 3, null, null)) == #less;

assert compareTrees(
  ?(1, 2, ?(3, 4, null, null), null),
  ?(1, 2, ?(3, 4, null, null), null),
) == #equal;

assert compareTrees(
  ?(1, 2, ?(3, 4, null, null), null),
  ?(1, 2, ?(3, 5, null, null), null),
) == #less;

// Left vs right subtree ordering
assert compareTrees(
  ?(1, 2, ?(3, 4, null, null), ?(5, 6, null, null)),
  ?(1, 2, ?(3, 4, null, null), ?(5, 7, null, null)),
) == #less;

// --- Recursive type used in a larger expression ---

type IntList = ?(Int, IntList);

func sortedInsert(
  x : Int,
  xs : IntList,
  compare : (implicit : (Int, Int) -> Order),
) : IntList {
  switch xs {
    case null ?(x, null);
    case (?(h, t)) {
      switch (compare(x, h)) {
        case (#less) ?(x, xs);
        case (#equal) ?(x, xs);
        case (#greater) ?(h, sortedInsert(x, t));
      }
    }
  }
};

// Not testing the derivation of IntList compare here,
// just that recursive types with implicits work in broader contexts

func compareIntLists(
  a : IntList,
  b : IntList,
  compare : (implicit : (IntList, IntList) -> Order),
) : Order { compare(a, b) };

let list1 = sortedInsert(3, sortedInsert(1, sortedInsert(2, null)));
let list2 = sortedInsert(3, sortedInsert(1, sortedInsert(2, null)));
let list3 = sortedInsert(4, sortedInsert(1, sortedInsert(2, null)));

assert compareIntLists(list1, list2) == #equal;
assert compareIntLists(list1, list3) == #less;
