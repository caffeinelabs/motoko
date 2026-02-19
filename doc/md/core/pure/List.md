# core/pure/List
Purely-functional, singly-linked list data structure.
This module provides immutable lists with efficient prepend and traversal operations.

A list of type `List<T>` is either `null` or an optional pair of a value of type `T` and a tail, itself of type `List<T>`.

To use this library, import it using:

```motoko name=import
import List "mo:core/pure/List";
```

## Type `List`
``` motoko no-repl
type List<T> = Types.Pure.List<T>
```

@deprecated M0235

## Function `empty`
``` motoko no-repl
func empty<T>() : List<T>
```

Create an empty list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  assert List.empty<Nat>() == null;
}
```

Runtime: O(1)

Space: O(1)

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : List<T>) : Bool
```

Check whether a list is empty and return true if the list is empty.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  assert List.isEmpty(null);
  assert not List.isEmpty(?(1, null));
}
```

Runtime: O(1)

Space: O(1)

## Function `size`
``` motoko no-repl
func size<T>(self : List<T>) : Nat
```

Return the length of the list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, null));
  assert List.size(list) == 2;
}
```

Runtime: O(size)

Space: O(1)

## Function `contains`
``` motoko no-repl
func contains<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), item : T) : Bool
```

Check whether the list contains a given value. Uses the provided equality function to compare values.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.contains(list, Nat.equal, 2);
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `equal` runs in O(1) time and space.

## Function `at`
``` motoko no-repl
func at<T>(self : List<T>, n : Nat) : T
```

Access any item in a list by index (zero-based). Traps if the index is out of bounds.

NOTE: Indexing into a list is a linear operation, and usually an
indication that a list might not be the best data structure
to use.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, null));
  assert List.at(list, 1) == 1;
}
```

Runtime: O(size)

Space: O(1)

## Function `get`
``` motoko no-repl
func get<T>(self : List<T>, n : Nat) : ?T
```

Optional access to any item in a list by index (zero-based). Returns `null` if the index is out of bounds.

NOTE: Indexing into a list is a linear operation, and usually an
indication that a list might not be the best data structure
to use.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, null));
  assert List.get(list, 1) == ?1;
}
```

Runtime: O(size)

Space: O(1)

## Function `pushFront`
``` motoko no-repl
func pushFront<T>(self : List<T>, item : T) : List<T>
```

Add `item` to the head of `list`, and return the new list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  assert List.pushFront(null, 0) == ?(0, null);
}
```

Runtime: O(1)

Space: O(1)

## Function `last`
``` motoko no-repl
func last<T>(self : List<T>) : ?T
```

Return the last element of the list, if present.
Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, null));
  assert List.last(list) == ?1;
}
```

Runtime: O(size)

Space: O(1)

## Function `popFront`
``` motoko no-repl
func popFront<T>(self : List<T>) : (?T, List<T>)
```

Remove the head of the list, returning the optioned head and the tail of the list in a pair.
Returns `(null, null)` if the list is empty.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, null));
  assert List.popFront(list) == (?0, ?(1, null));
}
```

Runtime: O(1)

Space: O(1)

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : List<T>) : List<T>
```

Reverses the list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.reverse(list) == ?(2, ?(1, ?(0, null)));
}
```

Runtime: O(size)

Space: O(size)

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : List<T>, f : T -> ())
```

Call the given function for its side effect, with each list element in turn.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  var sum = 0;
  List.forEach<Nat>(list, func n = sum += n);
  assert sum == 3;
}
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `map`
``` motoko no-repl
func map<T1, T2>(self : List<T1>, f : T1 -> T2) : List<T2>
```

Call the given function `f` on each list element and collect the results
in a new list.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.map(list, Nat.toText) == ?("0", ?("1", ?("2", null)));
}
```

Runtime: O(size)

Space: O(size)
*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `filter`
``` motoko no-repl
func filter<T>(self : List<T>, f : T -> Bool) : List<T>
```

Create a new list with only those elements of the original list for which
the given function (often called the _predicate_) returns true.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.filter<Nat>(list, func n = n != 1) == ?(0, ?(2, null));
}
```

Runtime: O(size)

Space: O(size)

## Function `filterMap`
``` motoko no-repl
func filterMap<T, R>(self : List<T>, f : T -> ?R) : List<R>
```

Call the given function on each list element, and collect the non-null results
in a new list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.filterMap<Nat, Nat>(
    list,
    func n = if (n > 1) ?(n * 2) else null
  ) == ?(4, ?(6, null));
}
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `mapResult`
``` motoko no-repl
func mapResult<T, R, E>(self : List<T>, f : T -> Result.Result<R, E>) : Result.Result<List<R>, E>
```

Maps a `Result`-returning function `f` over a `List` and returns either
the first error or a list of successful values.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.mapResult<Nat, Nat, Text>(
    list,
    func n = if (n > 0) #ok(n * 2) else #err "Some element is zero"
  ) == #ok(?(2, ?(4, ?(6, null))));
}
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `partition`
``` motoko no-repl
func partition<T>(self : List<T>, f : T -> Bool) : (List<T>, List<T>)
```

Create two new lists from the results of a given function (`f`).
The first list only includes the elements for which the given
function `f` returns true and the second list only includes
the elements for which the function returns false.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.partition<Nat>(list, func n = n != 1) == (?(0, ?(2, null)), ?(1, null));
}
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `concat`
``` motoko no-repl
func concat<T>(self : List<T>, other : List<T>) : List<T>
```

Append the elements from one list to another list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list1 = ?(0, ?(1, ?(2, null)));
  let list2 = ?(3, ?(4, ?(5, null)));
  assert List.concat(list1, list2) == ?(0, ?(1, ?(2, ?(3, ?(4, ?(5, null))))));
}
```

Runtime: O(size(l))

Space: O(size(l))

## Function `join`
``` motoko no-repl
func join<T>(iter : Iter.Iter<List<T>>) : List<T>
```

Flatten, or repatedly concatenate, an iterator of lists as a list.

Example:
```motoko
import List "mo:core/pure/List";
import Iter "mo:core/Iter";

persistent actor {
  let lists = [ ?(0, ?(1, ?(2, null))),
                ?(3, ?(4, ?(5, null))) ];
  assert List.join(lists |> Iter.fromArray(_)) == ?(0, ?(1, ?(2, ?(3, ?(4, ?(5, null))))));
}
```

Runtime: O(size*size)

Space: O(size*size)

## Function `flatten`
``` motoko no-repl
func flatten<T>(self : List<List<T>>) : List<T>
```

Flatten, or repatedly concatenate, a list of lists as a list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let lists = ?(?(0, ?(1, ?(2, null))),
               ?(?(3, ?(4, ?(5, null))),
                 null));
  assert List.flatten(lists) == ?(0, ?(1, ?(2, ?(3, ?(4, ?(5, null))))));
}
```

Runtime: O(size*size)

Space: O(size*size)

## Function `take`
``` motoko no-repl
func take<T>(self : List<T>, n : Nat) : List<T>
```

Returns the first `n` elements of the given list.
If the given list has fewer than `n` elements, this function returns
a copy of the full input list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.take(list, 2) == ?(0, ?(1, null));
}
```

Runtime: O(n)

Space: O(n)

## Function `drop`
``` motoko no-repl
func drop<T>(self : List<T>, n : Nat) : List<T>
```

Drop the first `n` elements from the given list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.drop(list, 2) == ?(2, null);
}
```

Runtime: O(n)

Space: O(1)

## Function `foldLeft`
``` motoko no-repl
func foldLeft<T, A>(self : List<T>, base : A, combine : (A, T) -> A) : A
```

Collapses the elements in `list` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
left to right.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.foldLeft<Nat, Text>(
    list,
    "",
    func (acc, x) = acc # Nat.toText(x)
  ) == "123";
}
```

Runtime: O(size(list))

Space: O(1) heap, O(1) stack

*Runtime and space assumes that `combine` runs in O(1) time and space.

## Function `foldRight`
``` motoko no-repl
func foldRight<T, A>(self : List<T>, base : A, combine : (T, A) -> A) : A
```

Collapses the elements in `buffer` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
right to left.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.foldRight<Nat, Text>(
    list,
    "",
    func (x, acc) = Nat.toText(x) # acc
  ) == "123";
}
```

Runtime: O(size(list))

Space: O(1) heap, O(size(list)) stack

*Runtime and space assumes that `combine` runs in O(1) time and space.

## Function `find`
``` motoko no-repl
func find<T>(self : List<T>, f : T -> Bool) : ?T
```

Return the first element for which the given predicate `f` is true,
if such an element exists.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.find<Nat>(list, func n = n > 1) == ?2;
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `findIndex`
``` motoko no-repl
func findIndex<T>(self : List<T>, f : T -> Bool) : ?Nat
```

Return the first index for which the given predicate `f` is true.
If no element satisfies the predicate, returns null.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.fromArray(['A', 'B', 'C', 'D']);
  let found = List.findIndex<Char>(list, func(x) { x == 'C' });
  assert found == ?2;
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `all`
``` motoko no-repl
func all<T>(self : List<T>, f : T -> Bool) : Bool
```

Return true if the given predicate `f` is true for all list
elements.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert not List.all<Nat>(list, func n = n > 1);
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `any`
``` motoko no-repl
func any<T>(self : List<T>, f : T -> Bool) : Bool
```

Return true if there exists a list element for which
the given predicate `f` is true.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.any<Nat>(list, func n = n > 1);
}
```

Runtime: O(size(list))

Space: O(1)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `merge`
``` motoko no-repl
func merge<T>(self : List<T>, other : List<T>, compare : (implicit : (T, T) -> Order.Order)) : List<T>
```

Merge two ordered lists into a single ordered list.
This function requires both list to be ordered as specified
by the given relation `compare`.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list1 = ?(1, ?(2, ?(4, null)));
  let list2 = ?(2, ?(4, ?(6, null)));
  assert List.merge(list1, list2, Nat.compare) == ?(1, ?(2, ?(2, ?(4, ?(4, ?(6, null))))));
}
```

Runtime: O(size(l1) + size(l2))

Space: O(size(l1) + size(l2))

*Runtime and space assumes that `lessThanOrEqual` runs in O(1) time and space.

## Function `equal`
``` motoko no-repl
func equal<T>(self : List<T>, other : List<T>, equalItem : (implicit : (equal : (T, T) -> Bool))) : Bool
```

Check if two lists are equal using the given equality function to compare elements.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list1 = ?(1, ?(2, null));
  let list2 = ?(1, ?(2, null));
  assert List.equal(list1, list2, Nat.equal);
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `equalItem` runs in O(1) time and space.

## Function `compare`
``` motoko no-repl
func compare<T>(self : List<T>, other : List<T>, compareItem : (implicit : (compare : (T, T) -> Order.Order))) : Order.Order
```

Compare two lists using lexicographic ordering specified by argument function `compareItem`.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list1 = ?(1, ?(2, null));
  let list2 = ?(3, ?(4, null));
  assert List.compare(list1, list2, Nat.compare) == #less;
}
```

Runtime: O(size(l1))

Space: O(1)

*Runtime and space assumes that argument `compare` runs in O(1) time and space.

## Function `tabulate`
``` motoko no-repl
func tabulate<T>(n : Nat, f : Nat -> T) : List<T>
```

Generate a list based on a length and a function that maps from
a list index to a list element.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.tabulate<Nat>(3, func n = n * 2);
  assert list == ?(0, ?(2, ?(4, null)));
}
```

Runtime: O(n)

Space: O(n)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `singleton`
``` motoko no-repl
func singleton<T>(item : T) : List<T>
```

Create a list with exactly one element.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  assert List.singleton(0) == ?(0, null);
}
```

Runtime: O(1)

Space: O(1)

## Function `repeat`
``` motoko no-repl
func repeat<T>(item : T, n : Nat) : List<T>
```

Create a list of the given length with the same value in each position.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.repeat('a', 3);
  assert list == ?('a', ?('a', ?('a', null)));
}
```

Runtime: O(n)

Space: O(n)

## Function `zip`
``` motoko no-repl
func zip<T, U>(self : List<T>, other : List<U>) : List<(T, U)>
```

Create a list of pairs from a pair of lists.

If the given lists have different lengths, then the created list will have a
length equal to the length of the smaller list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list1 = ?(0, ?(1, ?(2, null)));
  let list2 = ?("0", ?("1", null));
  assert List.zip(list1, list2) == ?((0, "0"), ?((1, "1"), null));
}
```

Runtime: O(min(size(xs), size(ys)))

Space: O(min(size(xs), size(ys)))

## Function `zipWith`
``` motoko no-repl
func zipWith<T, U, V>(self : List<T>, other : List<U>, f : (T, U) -> V) : List<V>
```

Create a list in which elements are created by applying function `f` to each pair `(x, y)` of elements
occuring at the same position in list `xs` and list `ys`.

If the given lists have different lengths, then the created list will have a
length equal to the length of the smaller list.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";
import Char "mo:core/Char";

persistent actor {
  let list1 = ?(0, ?(1, ?(2, null)));
  let list2 = ?('a', ?('b', null));
  assert List.zipWith<Nat, Char, Text>(
    list1,
    list2,
    func (n, c) = Nat.toText(n) # Char.toText(c)
  ) == ?("0a", ?("1b", null));
}
```

Runtime: O(min(size(xs), size(ys)))

Space: O(min(size(xs), size(ys)))

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `split`
``` motoko no-repl
func split<T>(self : List<T>, n : Nat) : (List<T>, List<T>)
```

Split the given list at the given zero-based index.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, null)));
  assert List.split(list, 2) == (?(0, ?(1, null)), ?(2, null));
}
```

Runtime: O(n)

Space: O(n)

## Function `chunks`
``` motoko no-repl
func chunks<T>(self : List<T>, n : Nat) : List<List<T>>
```

Split the given list into chunks of length `n`.
The last chunk will be shorter if the length of the given list
does not divide by `n` evenly. Traps if `n` = 0.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = ?(0, ?(1, ?(2, ?(3, ?(4, null)))));
  assert List.chunks(list, 2) == ?(?(0, ?(1, null)), ?(?(2, ?(3, null)), ?(?(4, null), null)));
}
```

Runtime: O(size)

Space: O(size)

## Function `values`
``` motoko no-repl
func values<T>(self : List<T>) : Iter.Iter<T>
```

Returns an iterator to the elements in the list.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = List.fromArray([3, 1, 4]);
  var text = "";
  for (item in List.values(list)) {
    text #= Nat.toText(item);
  };
  assert text == "314";
}
```

## Function `enumerate`
``` motoko no-repl
func enumerate<T>(self : List<T>) : Iter.Iter<(Nat, T)>
```

Returns an iterator to the `(index, element)` pairs in the list.

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = List.fromArray([3, 1, 4]);
  var text = "";
  for ((index, element) in List.enumerate(list)) {
    text #= Nat.toText(index);
  };
  assert text == "012";
}
```

## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T]) : List<T>
```

Convert an array into a list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.fromArray([0, 1, 2, 3, 4]);
  assert list == ?(0, ?(1, ?(2, ?(3, ?(4, null)))));
}
```

Runtime: O(size)

Space: O(size)

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray<T>(array : [var T]) : List<T>
```

Convert a mutable array into a list.

Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.fromVarArray([var 0, 1, 2, 3, 4]);
  assert list == ?(0, ?(1, ?(2, ?(3, ?(4, null)))));
}
```

Runtime: O(size)

Space: O(size)

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : List<T>) : [T]
```

Create an array from a list.
Example:
```motoko
import List "mo:core/pure/List";
import Array "mo:core/Array";
import Nat "mo:core/Nat";

persistent actor {
  let array = List.toArray(?(0, ?(1, ?(2, ?(3, ?(4, null))))));
  assert Array.equal(array, [0, 1, 2, 3, 4], Nat.equal);
}
```

Runtime: O(size)

Space: O(size)

## Function `toVarArray`
``` motoko no-repl
func toVarArray<T>(self : List<T>) : [var T]
```

Create a mutable array from a list.
Example:
```motoko
import List "mo:core/pure/List";
import Array "mo:core/Array";
import Nat "mo:core/Nat";

persistent actor {
  let array = List.toVarArray<Nat>(?(0, ?(1, ?(2, ?(3, ?(4, null))))));
  assert Array.equal(Array.fromVarArray(array), [0, 1, 2, 3, 4], Nat.equal);
}
```

Runtime: O(size)

Space: O(size)

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Iter.Iter<T>) : List<T>
```

Create a list from an iterator, consuming the iterator.
Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  let list = List.fromIter([0, 1, 2, 3, 4].values());
  assert list == ?(0, ?(1, ?(2, ?(3, ?(4, null)))));
}
```

Runtime: O(size)

Space: O(size)

## Function `toList`
``` motoko no-repl
func toList<T>(self : Iter.Iter<T>) : List<T>
```

Convert an iterator to a list, consuming the iterator.
Example:
```motoko
import List "mo:core/pure/List";

persistent actor {
  transient let iter = [0, 1, 2, 3, 4].values();

  let list = iter.toList();

  assert list == ?(0, ?(1, ?(2, ?(3, ?(4, null)))));
}
```

Runtime: O(size)

Space: O(size)

## Function `toText`
``` motoko no-repl
func toText<T>(self : List<T>, f : (implicit : T -> Text)) : Text
```

Convert a list to a text representation using the provided function to convert each element to text.
The resulting text will be in the format "[element1, element2, ...]".

Example:
```motoko
import List "mo:core/pure/List";
import Nat "mo:core/Nat";

persistent actor {
  let list = ?(1, ?(2, ?(3, null)));
  assert List.toText(list, Nat.toText) == "PureList[1, 2, 3]";
}
```

Runtime: O(size)

Space: O(size)
