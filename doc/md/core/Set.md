# core/Set
Imperative (mutable) sets based on order/comparison of elements.
A set is a collection of elements without duplicates.
The set data structure type is stable and can be used for orthogonal persistence.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter([3, 1, 2, 3].values(), Nat.compare);
  assert Set.size(set) == 3;
  assert not Set.contains(set, Nat.compare, 4);
  let diff = Set.difference(set, set, Nat.compare);
  assert Set.isEmpty(diff);
}
```

These sets are implemented as B-trees with order 32, a balanced search tree of ordered elements.

Performance:
* Runtime: `O(log(n))` worst case cost per insertion, removal, and retrieval operation.
* Space: `O(n)` for storing the entire tree,
where `n` denotes the number of elements stored in the set.

## Type `Set`
``` motoko no-repl
type Set<T> = Types.Set.Set<T>
```


## Function `toPure`
``` motoko no-repl
func toPure<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : PureSet.Set<T>
```

Convert the mutable set to an immutable, purely functional set.

Example:
```motoko
import Set "mo:core/Set";
import PureSet "mo:core/pure/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter<Nat>([0, 2, 1].values(), Nat.compare);
  let pureSet = Set.toPure(set, Nat.compare);
  assert Iter.toArray(PureSet.values(pureSet)) == Iter.toArray(Set.values(set));
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)` retained memory plus garbage, see the note below.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

Note: Creates `O(n * log(n))` temporary objects that will be collected as garbage.
@deprecated M0235

## Function `fromPure`
``` motoko no-repl
func fromPure<T>(set : PureSet.Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Convert an immutable, purely functional set to a mutable set.

Example:
```motoko
import PureSet "mo:core/pure/Set";
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let pureSet = PureSet.fromIter([3, 1, 2].values(), Nat.compare);
  let set = Set.fromPure(pureSet, Nat.compare);
  assert Iter.toArray(Set.values(set)) == Iter.toArray(PureSet.values(pureSet));
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)`.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.
@deprecated M0235

## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T], compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```


## Function `clone`
``` motoko no-repl
func clone<T>(self : Set<T>) : Set<T>
```

Create a copy of the mutable set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let originalSet = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let clonedSet = Set.clone(originalSet);
  Set.add(originalSet, Nat.compare, 4);
  assert Set.size(clonedSet) == 3;
  assert Set.size(originalSet) == 4;
}
```

Runtime: `O(n)`.
Space: `O(n)`.
where `n` denotes the number of elements stored in the set.

## Function `empty`
``` motoko no-repl
func empty<T>() : Set<T>
```

Create a new empty mutable set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  assert Set.size(set) == 0;
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : Set<T>
```

Create a new mutable set with a single element.

Example:
```motoko
import Set "mo:core/Set";

persistent actor {
  let cities = Set.singleton<Text>("Zurich");
  assert Set.size(cities) == 1;
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `clear`
``` motoko no-repl
func clear<T>(self : Set<T>)
```

Remove all the elements from the set.

Example:
```motoko
import Set "mo:core/Set";
import Text "mo:core/Text";

persistent actor {
  let cities = Set.empty<Text>();
  Set.add(cities, Text.compare, "Zurich");
  Set.add(cities, Text.compare, "San Francisco");
  Set.add(cities, Text.compare, "London");
  assert Set.size(cities) == 3;

  Set.clear(cities);
  assert Set.size(cities) == 0;
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : Set<T>) : Bool
```

Determines whether a set is empty.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  Set.add(set, Nat.compare, 3);

  assert not Set.isEmpty(set);
  Set.clear(set);
  assert Set.isEmpty(set);
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `size`
``` motoko no-repl
func size<T>(self : Set<T>) : Nat
```

Return the number of elements in a set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  Set.add(set, Nat.compare, 3);

  assert Set.size(set) == 3;
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `equal`
``` motoko no-repl
func equal<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Types.Order)) : Bool
```

Test whether two imperative sets are equal.
Both sets have to be constructed by the same comparison function.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set1 = Set.fromIter([1, 2].values(), Nat.compare);
  let set2 = Set.fromIter([2, 1].values(), Nat.compare);
  let set3 = Set.fromIter([2, 1, 0].values(), Nat.compare);
  assert Set.equal(set1, set2, Nat.compare);
  assert not Set.equal(set1, set3, Nat.compare);
}
```

Runtime: `O(n)`.
Space: `O(1)`.

## Function `contains`
``` motoko no-repl
func contains<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : Bool
```

Tests whether the set contains the provided element.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  Set.add(set, Nat.compare, 3);

  assert Set.contains(set, Nat.compare, 1);
  assert not Set.contains(set, Nat.compare, 4);
}
```

Runtime: `O(log(n))`.
Space: `O(1)`.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

## Function `add`
``` motoko no-repl
func add<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T)
```

Add a new element to a set.
No effect if the element already exists in the set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.empty<Nat>();
  Set.add(set, Nat.compare, 2);
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  assert Iter.toArray(Set.values(set)) == [1, 2];
}
```

Runtime: `O(log(n))`.
Space: `O(log(n))`.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

## Function `insert`
``` motoko no-repl
func insert<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : Bool
```

Insert a new element in the set.
Returns true if the element is new, false if the element was already contained in the set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.empty<Nat>();
  assert Set.insert(set, Nat.compare, 2);
  assert Set.insert(set, Nat.compare, 1);
  assert not Set.insert(set, Nat.compare, 2);
  assert Iter.toArray(Set.values(set)) == [1, 2];
}
```

Runtime: `O(log(n))`.
Space: `O(log(n))`.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.
@deprecated M0235

## Function `remove`
``` motoko no-repl
func remove<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : ()
```

Deletes an element from a set.
No effect if the element is not contained in the set.

```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([1, 2, 3].values(), Nat.compare);

  Set.remove(set, Nat.compare, 2);
  assert not Set.contains(set, Nat.compare, 2);

  Set.remove(set, Nat.compare, 4);
  assert not Set.contains(set, Nat.compare, 4);

  assert Iter.toArray(Set.values(set)) == [1, 3];
}
```

Runtime: `O(log(n))`.
Space: `O(log(n))` including garbage, see below.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

Note: Creates `O(log(n))` objects that will be collected as garbage.

## Function `delete`
``` motoko no-repl
func delete<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : Bool
```

Deletes an element from a set.
Returns true if the element was contained in the set, false if not.

```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([1, 2, 3].values(), Nat.compare);

  assert Set.delete(set, Nat.compare, 2);
  assert not Set.contains(set, Nat.compare, 2);

  assert not Set.delete(set, Nat.compare, 4);
  assert not Set.contains(set, Nat.compare, 4);
  assert Iter.toArray(Set.values(set)) == [1, 3];
}
```

Runtime: `O(log(n))`.
Space: `O(log(n))` including garbage, see below.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

Note: Creates `O(log(n))` objects that will be collected as garbage.
@deprecated M0235

## Function `max`
``` motoko no-repl
func max<T>(self : Set<T>) : ?T
```

Retrieves the maximum element from the set.
If the set is empty, returns `null`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  assert Set.max(set) == null;
  Set.add(set, Nat.compare, 3);
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  assert Set.max(set) == ?3;
}
```

Runtime: `O(log(n))`.
Space: `O(1)`.
where `n` denotes the number of elements stored in the set.

## Function `min`
``` motoko no-repl
func min<T>(self : Set<T>) : ?T
```

Retrieves the minimum element from the set.
If the set is empty, returns `null`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.empty<Nat>();
  assert Set.min(set) == null;
  Set.add(set, Nat.compare, 1);
  Set.add(set, Nat.compare, 2);
  Set.add(set, Nat.compare, 3);
  assert Set.min(set) == ?1;
}
```

Runtime: `O(log(n))`.
Space: `O(1)`.
where `n` denotes the number of elements stored in the set.

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : Set<T>) : [T]
```


## Function `values`
``` motoko no-repl
func values<T>(self : Set<T>) : Types.Iter<T>
```

Returns an iterator over the elements in the set,
traversing the elements in the ascending order.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter([0, 2, 3, 1].values(), Nat.compare);

  var tmp = "";
  for (number in Set.values(set)) {
     tmp #= " " # Nat.toText(number);
  };
  assert tmp == " 0 1 2 3";
}
```
Cost of iteration over all elements:
Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `valuesFrom`
``` motoko no-repl
func valuesFrom<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : Types.Iter<T>
```

Returns an iterator over the elements in the set,
starting from a given element in ascending order.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([0, 3, 1].values(), Nat.compare);
  assert Iter.toArray(Set.valuesFrom(set, Nat.compare, 1)) == [1, 3];
  assert Iter.toArray(Set.valuesFrom(set, Nat.compare, 2)) == [3];
}
```
Cost of iteration over all elements:
Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of key-value entries stored in the map.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `reverseValues`
``` motoko no-repl
func reverseValues<T>(self : Set<T>) : Types.Iter<T>
```

Returns an iterator over the elements in the set,
traversing the elements in the descending order.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter([0, 2, 3, 1].values(), Nat.compare);

  var tmp = "";
  for (number in Set.reverseValues(set)) {
     tmp #= " " # Nat.toText(number);
  };
  assert tmp == " 3 2 1 0";
}
```
Cost of iteration over all elements:
Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `reverseValuesFrom`
``` motoko no-repl
func reverseValuesFrom<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), element : T) : Types.Iter<T>
```

Returns an iterator over the elements in the set,
starting from a given element in descending order.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([0, 1, 3].values(), Nat.compare);
  assert Iter.toArray(Set.reverseValuesFrom(set, Nat.compare, 0)) == [0];
  assert Iter.toArray(Set.reverseValuesFrom(set, Nat.compare, 2)) == [1, 0];
}
```
Cost of iteration over all elements:
Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Types.Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Create a mutable set with the elements obtained from an iterator.
Potential duplicate elements in the iterator are ignored, i.e.
multiple occurrence of an equal element only occur once in the set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter<Nat>([3, 1, 2, 1].values(), Nat.compare);
  assert Iter.toArray(Set.values(set)) == [1, 2, 3];
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)`.
where `n` denotes the number of elements returned by the iterator and
assuming that the `compare` function implements an `O(1)` comparison.

## Function `toSet`
``` motoko no-repl
func toSet<T>(self : Types.Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Convert an iterator of elements to a mutable set.
Potential duplicate elements in the iterator are ignored, i.e.
multiple occurrence of an equal element only occur once in the set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  transient let iter = [3, 1, 2, 1].values();

  let set = iter.toSet(Nat.compare);

  assert Iter.toArray(Set.values(set)) == [1, 2, 3];
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)`.
where `n` denotes the number of elements returned by the iterator and
assuming that the `compare` function implements an `O(1)` comparison.

## Function `isSubset`
``` motoko no-repl
func isSubset<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Bool
```

Test whether `set1` is a sub-set of `set2`, i.e. each element in `set1` is
also contained in `set2`. Returns `true` if both sets are equal.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set1 = Set.fromIter([1, 2].values(), Nat.compare);
  let set2 = Set.fromIter([2, 1, 0].values(), Nat.compare);
  let set3 = Set.fromIter([3, 4].values(), Nat.compare);
  assert Set.isSubset(set1, set2, Nat.compare);
  assert not Set.isSubset(set1, set3, Nat.compare);
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements stored in the sets `set1` and `set2`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `union`
``` motoko no-repl
func union<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Returns a new set that is the union of `set1` and `set2`,
i.e. a new set that all the elements that exist in at least on of the two sets.
Potential duplicates are ignored, i.e. if the same element occurs in both `set1`
and `set2`, it only occurs once in the returned set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set1 = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let set2 = Set.fromIter([3, 4, 5].values(), Nat.compare);
  let union = Set.union(set1, set2, Nat.compare);
  assert Iter.toArray(Set.values(union)) == [1, 2, 3, 4, 5];
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements stored in the sets `set1` and `set2`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `intersection`
``` motoko no-repl
func intersection<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Returns a new set that is the intersection of `set1` and `set2`,
i.e. a new set that contains all the elements that exist in both sets.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set1 = Set.fromIter([0, 1, 2].values(), Nat.compare);
  let set2 = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let intersection = Set.intersection(set1, set2, Nat.compare);
  assert Iter.toArray(Set.values(intersection)) == [1, 2];
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements stored in the sets `set1` and `set2`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `difference`
``` motoko no-repl
func difference<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Returns a new set that is the difference between `set1` and `set2` (`set1` minus `set2`),
i.e. a new set that contains all the elements of `set1` that do not exist in `set2`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set1 = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let set2 = Set.fromIter([3, 4, 5].values(), Nat.compare);
  let difference = Set.difference(set1, set2, Nat.compare);
  assert Iter.toArray(Set.values(difference)) == [1, 2];
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements stored in the sets `set1` and `set2`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `addAll`
``` motoko no-repl
func addAll<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), iter : Types.Iter<T>)
```

Adds all elements from `iter` to the specified `set`.
This is equivalent to `Set.union()` but modifies the set in place.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([1, 2, 3].values(), Nat.compare);
  Set.addAll(set, Nat.compare, [3, 4, 5].values());
  assert Iter.toArray(Set.values(set)) == [1, 2, 3, 4, 5];
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements in `set` and `iter`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `deleteAll`
``` motoko no-repl
func deleteAll<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), iter : Types.Iter<T>) : Bool
```

Deletes all values in `iter` from the specified `set`.
Returns `true` if any value was present in the set, otherwise false.
The return value indicates whether the size of the set has changed.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([0, 1, 2].values(), Nat.compare);
  assert Set.deleteAll(set, Nat.compare, [0, 2].values());
  assert Iter.toArray(Set.values(set)) == [1];
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements in `set` and `iter`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.
@deprecated M0235

## Function `insertAll`
``` motoko no-repl
func insertAll<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), iter : Types.Iter<T>) : Bool
```

Inserts all values in `iter` into `set`.
Returns true if any value was not contained in the original set, otherwise false.
The return value indicates whether the size of the set has changed.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([0, 1, 2].values(), Nat.compare);
  assert Set.insertAll(set, Nat.compare, [0, 2, 3].values());
  assert Iter.toArray(Set.values(set)) == [0, 1, 2, 3];
  assert not Set.insertAll(set, Nat.compare, [0, 1, 2].values()); // no change
}
```

Runtime: `O(m * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `m` and `n` denote the number of elements in `set` and `iter`, respectively,
and assuming that the `compare` function implements an `O(1)` comparison.
@deprecated M0235

## Function `retainAll`
``` motoko no-repl
func retainAll<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), predicate : T -> Bool) : Bool
```

Removes all values in `set` that do not satisfy the given predicate.
Returns `true` if and only if the size of the set has changed.
Modifies the set in place.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set = Set.fromIter([3, 1, 2].values(), Nat.compare);

  let sizeChanged = Set.retainAll<Nat>(set, Nat.compare, func n { n % 2 == 0 });
  assert Iter.toArray(Set.values(set)) == [2];
  assert sizeChanged;
}
```

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : Set<T>, operation : T -> ())
```

Apply an operation on each element contained in the set.
The operation is applied in ascending order of the elements.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let numbers = Set.fromIter([0, 3, 1, 2].values(), Nat.compare);

  var tmp = "";
  Set.forEach<Nat>(numbers, func (element) {
    tmp #= " " # Nat.toText(element)
  });
  assert tmp == " 0 1 2 3";
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `filter`
``` motoko no-repl
func filter<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order), criterion : T -> Bool) : Set<T>
```

Filter elements in a new set.
Create a copy of the mutable set that only contains the elements
that fulfil the criterion function.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let numbers = Set.fromIter([0, 3, 1, 2].values(), Nat.compare);

  let evenNumbers = Set.filter<Nat>(numbers, Nat.compare, func (number) {
    number % 2 == 0
  });
  assert Iter.toArray(Set.values(evenNumbers)) == [0, 2];
}
```

Runtime: `O(n)`.
Space: `O(n)`.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

## Function `map`
``` motoko no-repl
func map<T1, T2>(self : Set<T1>, compare : (implicit : (T2, T2) -> Order.Order), project : T1 -> T2) : Set<T2>
```

Project all elements of the set in a new set.
Apply a mapping function to each element in the set and
collect the mapped elements in a new mutable set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

persistent actor {
  let numbers = Set.fromIter([3, 1, 2].values(), Nat.compare);

  let textNumbers =
    Set.map<Nat, Text>(numbers, Text.compare, Nat.toText);
  assert Iter.toArray(Set.values(textNumbers)) == ["1", "2", "3"];
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set and
assuming that the `compare` function implements an `O(1)` comparison.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `filterMap`
``` motoko no-repl
func filterMap<T1, T2>(self : Set<T1>, compare : (implicit : (T2, T2) -> Order.Order), project : T1 -> ?T2) : Set<T2>
```

Filter all elements in the set by also applying a projection to the elements.
Apply a mapping function `project` to all elements in the set and collect all
elements, for which the function returns a non-null new element. Collect all
non-discarded new elements in a new mutable set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

persistent actor {
  let numbers = Set.fromIter([3, 0, 2, 1].values(), Nat.compare);

  let evenTextNumbers = Set.filterMap<Nat, Text>(numbers, Text.compare, func (number) {
    if (number % 2 == 0) {
       ?Nat.toText(number)
    } else {
       null // discard odd numbers
    }
  });
  assert Iter.toArray(Set.values(evenTextNumbers)) == ["0", "2"];
}
```

Runtime: `O(n * log(n))`.
Space: `O(n)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `foldLeft`
``` motoko no-repl
func foldLeft<T, A>(self : Set<T>, base : A, combine : (A, T) -> A) : A
```

Iterate all elements in ascending order,
and accumulate the elements by applying the combine function, starting from a base value.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter([0, 3, 2, 1].values(), Nat.compare);

  let text = Set.foldLeft<Nat, Text>(
     set,
     "",
     func (accumulator, element) {
       accumulator # " " # Nat.toText(element)
     }
  );
  assert text == " 0 1 2 3";
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `foldRight`
``` motoko no-repl
func foldRight<T, A>(self : Set<T>, base : A, combine : (T, A) -> A) : A
```

Iterate all elements in descending order,
and accumulate the elements by applying the combine function, starting from a base value.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter([0, 3, 2, 1].values(), Nat.compare);

  let text = Set.foldRight<Nat, Text>(
     set,
     "",
     func (element, accumulator) {
        accumulator # " " # Nat.toText(element)
     }
  );
  assert text == " 3 2 1 0";
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `join`
``` motoko no-repl
func join<T>(setIterator : Types.Iter<Set<T>>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Construct the union of a series of sets, i.e. all elements of
each set are included in the result set.
Any duplicates are ignored, i.e. if an element occurs
in several of the iterated sets, it only occurs once in the result set.

Assumes all sets are ordered by `compare`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let set1 = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let set2 = Set.fromIter([3, 4, 5].values(), Nat.compare);
  let set3 = Set.fromIter([5, 6, 7].values(), Nat.compare);
  let combined = Set.join([set1, set2, set3].values(), Nat.compare);
  assert Iter.toArray(Set.values(combined)) == [1, 2, 3, 4, 5, 6, 7];
}
```

Runtime: `O(n * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `n` denotes the number of elements stored in the iterated sets,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `flatten`
``` motoko no-repl
func flatten<T>(self : Set<Set<T>>, compare : (implicit : (T, T) -> Order.Order)) : Set<T>
```

Construct the union of a set of element sets, i.e. all elements of
each element set are included in the result set.
Any duplicates are ignored, i.e. if the same element occurs in multiple element sets,
it only occurs once in the result set.

Assumes all sets are ordered by `compare`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Order "mo:core/Order";
import Iter "mo:core/Iter";

persistent actor {
  func setCompare(first: Set.Set<Nat>, second: Set.Set<Nat>) : Order.Order {
     Set.compare(first, second, Nat.compare)
  };

  let set1 = Set.fromIter([1, 2, 3].values(), Nat.compare);
  let set2 = Set.fromIter([3, 4, 5].values(), Nat.compare);
  let set3 = Set.fromIter([5, 6, 7].values(), Nat.compare);
  let setOfSets = Set.fromIter([set1, set2, set3].values(), setCompare);
  let flatSet = Set.flatten(setOfSets, Nat.compare);
  assert Iter.toArray(Set.values(flatSet)) == [1, 2, 3, 4, 5, 6, 7];
}
```

Runtime: `O(n * log(n))`.
Space: `O(1)` retained memory plus garbage, see the note below.
where `n` denotes the number of elements stored in all the sub-sets,
and assuming that the `compare` function implements an `O(1)` comparison.

## Function `all`
``` motoko no-repl
func all<T>(self : Set<T>, predicate : T -> Bool) : Bool
```

Check whether all elements in the set satisfy a predicate, i.e.
the `predicate` function returns `true` for all elements in the set.
Returns `true` for an empty set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter<Nat>([0, 3, 1, 2].values(), Nat.compare);

  let belowTen = Set.all<Nat>(set, func (number) {
    number < 10
  });
  assert belowTen;
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `any`
``` motoko no-repl
func any<T>(self : Set<T>, predicate : T -> Bool) : Bool
```

Check whether at least one element in the set satisfies a predicate, i.e.
the `predicate` function returns `true` for at least one element in the set.
Returns `false` for an empty set.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter<Nat>([0, 3, 1, 2].values(), Nat.compare);

  let aboveTen = Set.any<Nat>(set, func (number) {
    number > 10
  });
  assert not aboveTen;
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `assertValid`
``` motoko no-repl
func assertValid<T>(self : Set<T>, compare : (implicit : (T, T) -> Order.Order))
```

Internal sanity check function.
Can be used to check that elements have been inserted with a consistent comparison function.
Traps if the internal set structure is invalid.
@deprecated M0235

## Function `toText`
``` motoko no-repl
func toText<T>(self : Set<T>, toText : (implicit : T -> Text)) : Text
```

Generate a textual representation of all the elements in the set.
Primarily to be used for testing and debugging.
The elements are formatted according to `elementFormat`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set = Set.fromIter<Nat>([0, 3, 1, 2].values(), Nat.compare);

  assert Set.toText(set, Nat.toText) == "Set{0, 1, 2, 3}"
}
```

Runtime: `O(n)`.
Space: `O(n)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set and
assuming that `elementFormat` has runtime and space costs of `O(1)`.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.

## Function `compare`
``` motoko no-repl
func compare<T>(self : Set<T>, other : Set<T>, compare : (implicit : (T, T) -> Order.Order)) : Order.Order
```

Compare two sets by comparing the elements.
Both sets must have been created by the same comparison function.
The two sets are iterated by the ascending order of their creation and
order is determined by the following rules:
Less:
`set1` is less than `set2` if:
 * the pairwise iteration hits an element pair `element1` and `element2` where
   `element1` is less than `element2` and all preceding elements are equal, or,
 * `set1` is  a strict prefix of `set2`, i.e. `set2` has more elements than `set1`
    and all elements of `set1` occur at the beginning of iteration `set2`.
Equal:
`set1` and `set2` have same series of equal elements by pairwise iteration.
Greater:
`set1` is neither less nor equal `set2`.

Example:
```motoko
import Set "mo:core/Set";
import Nat "mo:core/Nat";

persistent actor {
  let set1 = Set.fromIter([0, 1].values(), Nat.compare);
  let set2 = Set.fromIter([0, 2].values(), Nat.compare);

  assert Set.compare(set1, set2, Nat.compare) == #less;
  assert Set.compare(set1, set1, Nat.compare) == #equal;
  assert Set.compare(set2, set1, Nat.compare) == #greater;
}
```

Runtime: `O(n)`.
Space: `O(1)` retained memory plus garbage, see below.
where `n` denotes the number of elements stored in the set and
assuming that `compare` has runtime and space costs of `O(1)`.

Note: Creates `O(log(n))` temporary objects that will be collected as garbage.
