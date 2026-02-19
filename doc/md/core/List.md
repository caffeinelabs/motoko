# core/List
A mutable growable array data structure with efficient random access and dynamic resizing.
`List` provides O(1) access time and O(sqrt(n)) memory overhead. In contrast, `pure/List` is a purely functional linked list.
Can be declared `stable` for orthogonal persistence.

This implementation is adapted with permission from the `vector` Mops package created by Research AG.

Copyright: 2023 MR Research AG
Main author: Andrii Stepanov (AStepanov25)
Contributors: Timo Hanke (timohanke), Andy Gura (andygura), react0r-com

```motoko name=import
import List "mo:core/List";
```

## Type `List`
``` motoko no-repl
type List<T> = Types.List<T>
```

`List<T>` provides a mutable list of elements of type `T`.
Based on the paper "Resizable Arrays in Optimal Time and Space" by Brodnik, Carlsson, Demaine, Munro and Sedgewick (1999).
Since this is internally a two-dimensional array the access times for put and get operations
will naturally be 2x slower than Buffer and Array. However, Array is not resizable and Buffer
has `O(size)` memory waste.

The maximum number of elements in a `List` is 2^32.

## Function `empty`
``` motoko no-repl
func empty<T>() : List<T>
```

Creates a new empty List for elements of type T.

Example:
```motoko include=import
let list = List.empty<Nat>(); // Creates a new List
```

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : List<T>
```

Returns a new list with capacity and size 1, containing `element`.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.singleton<Nat>(1);
assert List.toText<Nat>(list, Nat.toText) == "List[1]";
```

Runtime: `O(1)`

Space: `O(1)`

## Function `repeat`
``` motoko no-repl
func repeat<T>(initValue : T, size : Nat) : List<T>
```

Creates a new List with `size` copies of the initial value.

Example:
```motoko include=import
let list = List.repeat<Nat>(2, 4);
assert List.toArray(list) == [2, 2, 2, 2];
```

Runtime: `O(size)`

Space: `O(size)`

## Function `fill`
``` motoko no-repl
func fill<T>(self : List<T>, value : T)
```

Fills all elements in the list with the given value.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3]);
List.fill(list, 0); // fills the list with 0
assert List.toArray(list) == [0, 0, 0];
```

Runtime: `O(size)`

Space: `O(1)`

## Function `toPure`
``` motoko no-repl
func toPure<T>(self : List<T>) : PureList.List<T>
```

Converts a mutable `List` to a purely functional `PureList`.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3]);
let pureList = List.toPure<Nat>(list); // converts to immutable PureList
```

Runtime: `O(size)`

Space: `O(size)`
@deprecated M0235

## Function `fromPure`
``` motoko no-repl
func fromPure<T>(pure : PureList.List<T>) : List<T>
```

Converts a purely functional `PureList` to a `List`.

Example:
```motoko include=import
import PureList "mo:core/pure/List";

let pureList = PureList.fromArray<Nat>([1, 2, 3]);
let list = List.fromPure<Nat>(pureList); // converts to List
```

Runtime: `O(size)`

Space: `O(size)`
@deprecated M0235

## Function `addRepeat`
``` motoko no-repl
func addRepeat<T>(self : List<T>, initValue : T, count : Nat)
```

Add to list `count` copies of the initial value.

```motoko include=import
let list = List.repeat<Nat>(2, 4); // [2, 2, 2, 2]
List.addRepeat(list, 2, 1); // [2, 2, 2, 2, 1, 1]
```

The maximum number of elements in a `List` is 2^32.

Runtime: `O(count)`

## Function `truncate`
``` motoko no-repl
func truncate<T>(self : List<T>, newSize : Nat)
```

Truncates the list to the specified size.
If the new size is larger than the current size, it will do nothing.
If the new size is equal to the current list size, after the operation list will be equal to cloned version of itself.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3, 4, 5]);
List.truncate(list, 3); // list is now [1, 2, 3]
assert List.toArray(list) == [1, 2, 3];
```

Runtime: `O(size)`

Space: `O(1)`

## Function `clear`
``` motoko no-repl
func clear<T>(self : List<T>)
```

Resets the list to size 0, de-referencing all elements.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
List.add(list, 12);
List.clear(list); // list is now empty
assert List.toArray(list) == [];
```

Runtime: `O(1)`

## Function `tabulate`
``` motoko no-repl
func tabulate<T>(size : Nat, generator : Nat -> T) : List<T>
```

Creates a list of size `size`. Each element at index i
is created by applying `generator` to i.

```motoko include=import
import Nat "mo:core/Nat";

let list = List.tabulate<Nat>(4, func i = i * 2);
assert List.toArray(list) == [0, 2, 4, 6];
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `generator` runs in O(1) time and space.

## Function `flatten`
``` motoko no-repl
func flatten<T>(self : List<List<T>>) : List<T>
```

Combines a list of lists into a single list. Retains the original
ordering of the elements.

This has better performance compared to `List.join()`.

```motoko include=import
import Nat "mo:core/Nat";

let lists = List.fromArray<List.List<Nat>>([
  List.fromArray<Nat>([0, 1, 2]), List.fromArray<Nat>([2, 3]), List.fromArray<Nat>([]), List.fromArray<Nat>([4])
]);
let flatList = List.flatten<Nat>(lists);
assert List.equal<Nat>(flatList, List.fromArray<Nat>([0, 1, 2, 2, 3, 4]), Nat.equal);
```

Runtime: O(number of elements in list)

Space: O(number of elements in list)

## Function `join`
``` motoko no-repl
func join<T>(self : Types.Iter<List<T>>) : List<T>
```

Combines an iterator of lists into a single list.
Retains the original ordering of the elements.

Consider using `List.flatten()` for better performance.

```motoko include=import
import Nat "mo:core/Nat";

let lists = [List.fromArray<Nat>([0, 1, 2]), List.fromArray<Nat>([2, 3]), List.fromArray<Nat>([]), List.fromArray<Nat>([4])];
let joinedList = List.join<Nat>(lists.values());
assert List.equal<Nat>(joinedList, List.fromArray<Nat>([0, 1, 2, 2, 3, 4]), Nat.equal);
```

Runtime: O(number of elements in list)

Space: O(number of elements in list)

## Function `clone`
``` motoko no-repl
func clone<T>(self : List<T>) : List<T>
```

Returns a copy of a List, with the same size.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 1);

let clone = List.clone(list);
assert List.toArray(clone) == [1];
```

Runtime: `O(size)`

## Function `map`
``` motoko no-repl
func map<T, R>(self : List<T>, f : T -> R) : List<R>
```

Creates a new list by applying the provided function to each element in the input list.
The resulting list has the same size as the input list.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.singleton<Nat>(123);
let textList = List.map<Nat, Text>(list, Nat.toText);
assert List.toArray(textList) == ["123"];
```

Runtime: `O(size)`

## Function `mapInPlace`
``` motoko no-repl
func mapInPlace<T>(self : List<T>, f : T -> T)
```

Applies `f` to each element of `list` in place,
retaining the original ordering of elements.
This modifies the original list.

```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([0, 1, 2, 3]);
List.mapInPlace<Nat>(list, func x = x * 3);
assert List.equal(list, List.fromArray<Nat>([0, 3, 6, 9]), Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `mapEntries`
``` motoko no-repl
func mapEntries<T, R>(self : List<T>, f : (T, Nat) -> R) : List<R>
```

Creates a new list by applying `f` to each element in `list` and its index.
Retains original ordering of elements.

```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([10, 10, 10, 10]);
let newList = List.mapEntries<Nat, Nat>(list, func (x, i) = i * x);
assert List.equal(newList, List.fromArray<Nat>([0, 10, 20, 30]), Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `mapResult`
``` motoko no-repl
func mapResult<T, R, E>(self : List<T>, f : T -> Types.Result<R, E>) : Types.Result<List<R>, E>
```

Creates a new list by applying `f` to each element in `list`.
If any invocation of `f` produces an `#err`, returns an `#err`. Otherwise
returns an `#ok` containing the new list.

```motoko include=import
import Result "mo:core/Result";

let list = List.fromArray<Nat>([4, 3, 2, 1, 0]);
// divide 100 by every element in the list
let result = List.mapResult<Nat, Nat, Text>(list, func x {
  if (x > 0) {
    #ok(100 / x)
  } else {
    #err "Cannot divide by zero"
  }
});
assert Result.isErr(result);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `filter`
``` motoko no-repl
func filter<T>(self : List<T>, predicate : T -> Bool) : List<T>
```

Returns a new list containing only the elements from `list` for which the predicate returns true.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3, 4]);
let evenNumbers = List.filter<Nat>(list, func x = x % 2 == 0);
assert List.toArray(evenNumbers) == [2, 4];
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `predicate` runs in `O(1)` time and space.

## Function `retain`
``` motoko no-repl
func retain<T>(self : List<T>, predicate : T -> Bool)
```

Retains only the elements in `list` for which the predicate returns true.
Modifies the original list in place.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3, 4]);
List.retain<Nat>(list, func x = x % 2 == 0);
assert List.toArray(list) == [2, 4];
```

Runtime: `O(size)`

Space: `O(sqrt(size))` if `list` was truncated otherwise `O(1)`

## Function `filterMap`
``` motoko no-repl
func filterMap<T, R>(self : List<T>, f : T -> ?R) : List<R>
```

Returns a new list containing all elements from `list` for which the function returns ?element.
Discards all elements for which the function returns null.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3, 4]);
let doubled = List.filterMap<Nat, Nat>(list, func x = if (x % 2 == 0) ?(x * 2) else null);
assert List.toArray(doubled) == [4, 8];
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in `O(1)` time and space.

## Function `flatMap`
``` motoko no-repl
func flatMap<T, R>(self : List<T>, k : T -> Types.Iter<R>) : List<R>
```

Creates a new list by applying `k` to each element in `list`,
and concatenating the resulting iterators in order.

```motoko include=import
import Int "mo:core/Int"

let list = List.fromArray<Nat>([1, 2, 3, 4]);
let newList = List.flatMap<Nat, Int>(list, func x = [x, -x].values());
assert List.equal(newList, List.fromArray<Int>([1, -1, 2, -2, 3, -3, 4, -4]), Int.equal);
```
Runtime: O(size)

Space: O(size)
*Runtime and space assumes that `k` runs in O(1) time and space.

## Function `size`
``` motoko no-repl
func size<T>(self : List<T>) : Nat
```

Returns the current number of elements in the list.

Example:
```motoko include=import
let list = List.empty<Nat>();
assert List.size(list) == 0
```

Runtime: `O(1)` (with some internal calculations)

## Function `add`
``` motoko no-repl
func add<T>(self : List<T>, element : T)
```

Adds a single element to the end of a List,
allocating a new internal data block if needed,
and resizing the internal index block if needed.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 0); // add 0 to list
List.add(list, 1);
List.add(list, 2);
List.add(list, 3);
assert List.toArray(list) == [0, 1, 2, 3];
```

The maximum number of elements in a `List` is 2^32.

Amortized Runtime: `O(1)`, Worst Case Runtime: `O(sqrt(n))`

## Function `removeLast`
``` motoko no-repl
func removeLast<T>(self : List<T>) : ?T
```

Removes and returns the last item in the list or `null` if
the list is empty.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
assert List.removeLast(list) == ?11;
assert List.removeLast(list) == ?10;
assert List.removeLast(list) == null;
```

Amortized Runtime: `O(1)`, Worst Case Runtime: `O(sqrt(n))`

Amortized Space: `O(1)`, Worst Case Space: `O(sqrt(n))`

## Function `at`
``` motoko no-repl
func at<T>(self : List<T>, index : Nat) : T
```

Returns the element at index `index`. Indexing is zero-based.
Traps if `index >= size`, error message may not be descriptive.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
assert List.at(list, 0) == 10;
```

Runtime: `O(1)`

## Function `get`
``` motoko no-repl
func get<T>(self : List<T>, index : Nat) : ?T
```

Returns the element at index `index` as an option.
Returns `null` when `index >= size`. Indexing is zero-based.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
assert List.get(list, 0) == ?10;
assert List.get(list, 2) == null;
```

Runtime: `O(1)`

Space: `O(1)`
@deprecated M0235

## Function `put`
``` motoko no-repl
func put<T>(self : List<T>, index : Nat, value : T)
```

Overwrites the current element at `index` with `element`.
Traps if `index` >= size, error message may not be descriptive. Indexing is zero-based.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.put(list, 0, 20); // overwrites 10 at index 0 with 20
assert List.toArray(list) == [20];
```

Runtime: `O(1)`

## Function `sortInPlace`
``` motoko no-repl
func sortInPlace<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order))
```

Sorts the elements in the list according to `compare`.
Sort is deterministic, stable, and in-place.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 3);
List.add(list, 1);
List.add(list, 2);
List.sortInPlace(list, Nat.compare);
assert List.toArray(list) == [1, 2, 3];
```

Runtime: O(size * log(size))

Space: O(size)
*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `sort`
``` motoko no-repl
func sort<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order)) : List<T>
```

Sorts the elements in the list according to `compare`.
Sort is deterministic, stable, and in-place.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 3);
List.add(list, 1);
List.add(list, 2);
let sorted = List.sort(list, Nat.compare);
assert List.toArray(sorted) == [1, 2, 3];
```

Runtime: O(size * log(size))

Space: O(size)
*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `isSorted`
``` motoko no-repl
func isSorted<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order)) : Bool
```

Checks whether the `list` is sorted.

Example:
```
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1, 2, 3]);
assert List.isSorted(list, Nat.compare);
```

Runtime: O(size)

Space: O(1)

## Function `deduplicate`
``` motoko no-repl
func deduplicate<T>(self : List<T>, equal : (implicit : (T, T) -> Bool))
```

Remove adjacent duplicates from the `list`, if the `list` is sorted all elements will be unique.

Example:
```
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1, 1, 2, 2, 3]);
List.deduplicate(list, Nat.equal);
assert List.equal(list, List.fromArray<Nat>([1, 2, 3]), Nat.equal);
```

Runtime: O(size)

Space: O(1)

## Function `indexOf`
``` motoko no-repl
func indexOf<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), element : T) : ?Nat
```

Finds the first index of `element` in `list` using equality of elements defined
by `equal`. Returns `null` if `element` is not found.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 1);
List.add(list, 2);
List.add(list, 3);
List.add(list, 4);

assert List.indexOf<Nat>(list, Nat.equal, 3) == ?2;
assert List.indexOf<Nat>(list, Nat.equal, 5) == null;
```

Runtime: `O(size)`

*Runtime and space assumes that `equal` runs in `O(1)` time and space.

## Function `nextIndexOf`
``` motoko no-repl
func nextIndexOf<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), element : T, fromInclusive : Nat) : ?Nat
```

Returns the index of the next occurence of `element` in the `list` starting from the `from` index (inclusive).

```motoko include=import
import Char "mo:core/Char";
let list = List.fromArray<Char>(['c', 'o', 'f', 'f', 'e', 'e']);
assert List.nextIndexOf<Char>(list, Char.equal, 'c', 0) == ?0;
assert List.nextIndexOf<Char>(list, Char.equal, 'f', 0) == ?2;
assert List.nextIndexOf<Char>(list, Char.equal, 'f', 2) == ?2;
assert List.nextIndexOf<Char>(list, Char.equal, 'f', 3) == ?3;
assert List.nextIndexOf<Char>(list, Char.equal, 'f', 4) == null;
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `equal` runs in O(1) time and space.

## Function `lastIndexOf`
``` motoko no-repl
func lastIndexOf<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), element : T) : ?Nat
```

Finds the last index of `element` in `list` using equality of elements defined
by `equal`. Returns `null` if `element` is not found.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1, 2, 3, 4, 2, 2]);

assert List.lastIndexOf<Nat>(list, Nat.equal, 2) == ?5;
assert List.lastIndexOf<Nat>(list, Nat.equal, 5) == null;
```

Runtime: `O(size)`

*Runtime and space assumes that `equal` runs in `O(1)` time and space.

## Function `prevIndexOf`
``` motoko no-repl
func prevIndexOf<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), element : T, fromExclusive : Nat) : ?Nat
```

Returns the index of the previous occurence of `element` in the `list` starting from the `from` index (exclusive).

```motoko include=import
import Char "mo:core/Char";
let list = List.fromArray<Char>(['c', 'o', 'f', 'f', 'e', 'e']);
assert List.prevIndexOf<Char>(list, Char.equal, 'c', List.size(list)) == ?0;
assert List.prevIndexOf<Char>(list, Char.equal, 'e', List.size(list)) == ?5;
assert List.prevIndexOf<Char>(list, Char.equal, 'e', 5) == ?4;
assert List.prevIndexOf<Char>(list, Char.equal, 'e', 4) == null;
```

Runtime: O(size)

Space: O(1)

## Function `find`
``` motoko no-repl
func find<T>(self : List<T>, predicate : T -> Bool) : ?T
```

Returns the first value in `list` for which `predicate` returns true.
If no element satisfies the predicate, returns null.

```motoko include=import
let list = List.fromArray<Nat>([1, 9, 4, 8]);
let found = List.find<Nat>(list, func(x) { x > 8 });
assert found == ?9;
```
Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `findIndex`
``` motoko no-repl
func findIndex<T>(self : List<T>, predicate : T -> Bool) : ?Nat
```

Finds the index of the first element in `list` for which `predicate` is true.
Returns `null` if no such element is found.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 1);
List.add(list, 2);
List.add(list, 3);
List.add(list, 4);

assert List.findIndex<Nat>(list, func(i) { i % 2 == 0 }) == ?1;
assert List.findIndex<Nat>(list, func(i) { i > 5 }) == null;
```

Runtime: `O(size)`

*Runtime and space assumes that `predicate` runs in `O(1)` time and space.

## Function `findLastIndex`
``` motoko no-repl
func findLastIndex<T>(self : List<T>, predicate : T -> Bool) : ?Nat
```

Finds the index of the last element in `list` for which `predicate` is true.
Returns `null` if no such element is found.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 1);
List.add(list, 2);
List.add(list, 3);
List.add(list, 4);

assert List.findLastIndex<Nat>(list, func(i) { i % 2 == 0 }) == ?3;
assert List.findLastIndex<Nat>(list, func(i) { i > 5 }) == null;
```

Runtime: `O(size)`

*Runtime and space assumes that `predicate` runs in `O(1)` time and space.

## Function `binarySearch`
``` motoko no-repl
func binarySearch<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order), element : T) : {#found : Nat; #insertionIndex : Nat}
```

Performs binary search on a sorted list to find the index of the `element`.
Returns `#found(index)` if the element is found, or `#insertionIndex(index)` with the index
where the element would be inserted according to the ordering if not found.

If there are multiple equal elements, no guarantee is made about which index is returned.
The list must be sorted in ascending order according to the `compare` function.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1, 3, 5, 7, 9, 11]);
assert List.binarySearch<Nat>(list, Nat.compare, 5) == #found(2);
assert List.binarySearch<Nat>(list, Nat.compare, 6) == #insertionIndex(3);
```

Runtime: `O(log(size))`

Space: `O(1)`

*Runtime and space assumes that `compare` runs in `O(1)` time and space.

## Function `all`
``` motoko no-repl
func all<T>(self : List<T>, predicate : T -> Bool) : Bool
```

Returns true iff every element in `list` satisfies `predicate`.
In particular, if `list` is empty the function returns `true`.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 2);
List.add(list, 3);
List.add(list, 4);

assert List.all<Nat>(list, func x { x > 1 });
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `any`
``` motoko no-repl
func any<T>(self : List<T>, predicate : T -> Bool) : Bool
```

Returns true iff some element in `list` satisfies `predicate`.
In particular, if `list` is empty the function returns `false`.

Example:
```motoko include=import
let list = List.empty<Nat>();
List.add(list, 2);
List.add(list, 3);
List.add(list, 4);

assert List.any<Nat>(list, func x { x > 3 });
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `values`
``` motoko no-repl
func values<T>(self : List<T>) : Types.Iter<T>
```

Returns an Iterator (`Iter`) over the elements of a List.
Iterator provides a single method `next()`, which returns
elements in order, or `null` when out of elements to iterate over.

```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
List.add(list, 12);

var sum = 0;
for (element in List.values(list)) {
  sum += element;
};
assert sum == 33;
```

Note: This does not create a snapshot. If the returned iterator is not consumed at once,
and instead the consumption of the iterator is interleaved with other operations on the
List, then this may lead to unexpected results.

Runtime: `O(1)`

## Function `enumerate`
``` motoko no-repl
func enumerate<T>(self : List<T>) : Types.Iter<(Nat, T)>
```

Returns an Iterator (`Iter`) over the items (index-value pairs) in the list.
Each item is a tuple of `(index, value)`. The iterator provides a single method
`next()` which returns elements in order, or `null` when out of elements.

```motoko include=import
import Iter "mo:core/Iter";

let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
List.add(list, 12);
assert Iter.toArray(List.enumerate(list)) == [(0, 10), (1, 11), (2, 12)];
```

Note: This does not create a snapshot. If the returned iterator is not consumed at once,
and instead the consumption of the iterator is interleaved with other operations on the
List, then this may lead to unexpected results.

Runtime: `O(1)`

Warning: Allocates memory on the heap to store ?(Nat, T).

## Function `reverseValues`
``` motoko no-repl
func reverseValues<T>(self : List<T>) : Types.Iter<T>
```

Returns an Iterator (`Iter`) over the elements of the list in reverse order.
The iterator provides a single method `next()` which returns elements from
last to first, or `null` when out of elements.

```motoko include=import
let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
List.add(list, 12);

var sum = 0;
for (element in List.reverseValues(list)) {
  sum += element;
};
assert sum == 33;
```

Note: This does not create a snapshot. If the returned iterator is not consumed at once,
and instead the consumption of the iterator is interleaved with other operations on the
List, then this may lead to unexpected results.

Runtime: `O(1)`

## Function `reverseEnumerate`
``` motoko no-repl
func reverseEnumerate<T>(self : List<T>) : Types.Iter<(Nat, T)>
```

Returns an Iterator (`Iter`) over the items in reverse order, i.e. pairs of index and value.
Iterator provides a single method `next()`, which returns
elements in reverse order, or `null` when out of elements to iterate over.

```motoko include=import
import Iter "mo:core/Iter";

let list = List.empty<Nat>();
List.add(list, 10);
List.add(list, 11);
List.add(list, 12);
assert Iter.toArray(List.reverseEnumerate(list)) == [(2, 12), (1, 11), (0, 10)];
```

Note: This does not create a snapshot. If the returned iterator is not consumed at once,
and instead the consumption of the iterator is interleaved with other operations on the
List, then this may lead to unexpected results.

Runtime: `O(1)`

Warning: Allocates memory on the heap to store ?(T, Nat).

## Function `keys`
``` motoko no-repl
func keys<T>(self : List<T>) : Types.Iter<Nat>
```

Returns an Iterator (`Iter`) over the indices (keys) of the list.
The iterator provides a single method `next()` which returns indices
from 0 to size-1, or `null` when out of elements.

```motoko include=import
import Iter "mo:core/Iter";

let list = List.empty<Text>();
List.add(list, "A");
List.add(list, "B");
List.add(list, "C");
Iter.toArray(List.keys(list)) // [0, 1, 2]
```

Note: This does not create a snapshot. If the returned iterator is not consumed at once,
and instead the consumption of the iterator is interleaved with other operations on the
List, then this may lead to unexpected results.

Runtime: `O(1)`

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Types.Iter<T>) : List<T>
```

Creates a new List containing all elements from the provided iterator.
Elements are added in the order they are returned by the iterator.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let array = [1, 1, 1];
let iter = array.values();

let list = List.fromIter<Nat>(iter);
assert Iter.toArray(List.values(list)) == [1, 1, 1];
```

Runtime: `O(size)`

## Function `toList`
``` motoko no-repl
func toList<T>(self : Types.Iter<T>) : List<T>
```

Convert an iterator to a new mutable List.
Elements are added in the order they are returned by the iterator.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let array = [1, 1, 1];
let iter = array.values();

let list = iter.toList<Nat>();
assert Iter.toArray(List.values(list)) == [1, 1, 1];
```

Runtime: `O(size)`

## Function `append`
``` motoko no-repl
func append<T>(self : List<T>, added : List<T>)
```

Appends all elements from `added` to the end of `list`.
Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2]);
let added = List.fromArray<Nat>([3, 4]);
List.append<Nat>(list, added);
assert List.toArray(list) == [1, 2, 3, 4];
```

Runtime: `O(size(added))`

Space: `O(size(added))`

## Function `addAll`
``` motoko no-repl
func addAll<T>(self : List<T>, iter : Types.Iter<T>)
```

Adds all elements from the provided iterator to the end of the list.
Elements are added in the order they are returned by the iterator.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let array = [1, 1, 1];
let iter = array.values();
let list = List.repeat<Nat>(2, 1);

List.addAll<Nat>(list, iter);
assert Iter.toArray(List.values(list)) == [2, 1, 1, 1];
```

The maximum number of elements in a `List` is 2^32.

Runtime: `O(size)`, where n is the size of iter.

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : List<T>) : [T]
```

Creates a new immutable array containing all elements from the list.
Elements appear in the same order as in the list.

Example:
```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3]);

assert List.toArray<Nat>(list) == [1, 2, 3];
```

Runtime: `O(size)`

## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T]) : List<T>
```

Creates a List containing elements from an Array.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let array = [2, 3];
let list = List.fromArray<Nat>(array);
assert Iter.toArray(List.values(list)) == [2, 3];
```

Runtime: `O(size)`

## Function `toVarArray`
``` motoko no-repl
func toVarArray<T>(self : List<T>) : [var T]
```

Creates a new mutable array containing all elements from the list.
Elements appear in the same order as in the list.

Example:
```motoko include=import
import Array "mo:core/Array";

let list = List.fromArray<Nat>([1, 2, 3]);

let varArray = List.toVarArray<Nat>(list);
assert Array.fromVarArray(varArray) == [1, 2, 3];
```

Runtime: `O(size)`

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray<T>(array : [var T]) : List<T>
```

Creates a new List containing all elements from the mutable array.
Elements appear in the same order as in the array.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let array = [var 2, 3];
let list = List.fromVarArray<Nat>(array);
assert Iter.toArray(List.values(list)) == [2, 3];
```

Runtime: `O(size)`

## Function `first`
``` motoko no-repl
func first<T>(self : List<T>) : ?T
```

Returns the first element of `list`, or `null` if the list is empty.

Example:
```motoko include=import
assert List.first(List.fromArray<Nat>([1, 2, 3])) == ?1;
assert List.first(List.empty<Nat>()) == null;
```

Runtime: `O(1)`

Space: `O(1)`

## Function `last`
``` motoko no-repl
func last<T>(self : List<T>) : ?T
```

Returns the last element of `list`, or `null` if the list is empty.

Example:
```motoko include=import
assert List.last(List.fromArray<Nat>([1, 2, 3])) == ?3;
assert List.last(List.empty<Nat>()) == null;
```

Runtime: `O(1)`

Space: `O(1)`

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : List<T>, f : T -> ())
```

Applies `f` to each element in `list`.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";

let list = List.fromArray<Nat>([1, 2, 3]);

List.forEach<Nat>(list, func(x) {
  Debug.print(Nat.toText(x)); // prints each element in list
});
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `forEachEntry`
``` motoko no-repl
func forEachEntry<T>(self : List<T>, f : (Nat, T) -> ())
```

Applies `f` to each item `(i, x)` in `list` where `i` is the key
and `x` is the value.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";

let list = List.fromArray<Nat>([1, 2, 3]);

List.forEachEntry<Nat>(list, func (i,x) {
  // prints each item (i,x) in list
  Debug.print(Nat.toText(i) # Nat.toText(x));
});
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `range`
``` motoko no-repl
func range<T>(self : List<T>, fromInclusive : Int, toExclusive : Int) : Types.Iter<T>
```

Returns an iterator over a slice of `list` starting at `fromInclusive` up to (but not including) `toExclusive`.

Negative indices are relative to the end of the list. For example, `-1` corresponds to the last element in the list.

If the indices are out of bounds, they are clamped to the list bounds.
If the first index is greater than the second, the function returns an empty iterator.

```motoko include=import
let list = List.fromArray<Nat>([1, 2, 3, 4, 5]);
let iter1 = List.range<Nat>(list, 3, List.size(list));
assert iter1.next() == ?4;
assert iter1.next() == ?5;
assert iter1.next() == null;

let iter2 = List.range<Nat>(list, 3, -1);
assert iter2.next() == ?4;
assert iter2.next() == null;

let iter3 = List.range<Nat>(list, 0, 0);
assert iter3.next() == null;
```

Runtime: O(1)

Space: O(1)

## Function `sliceToArray`
``` motoko no-repl
func sliceToArray<T>(self : List<T>, fromInclusive : Int, toExclusive : Int) : [T]
```

Returns a new array containing elements from `list` starting at index `fromInclusive` up to (but not including) index `toExclusive`.
If the indices are out of bounds, they are clamped to the array bounds.

```motoko include=import
let array = List.fromArray<Nat>([1, 2, 3, 4, 5]);

let slice1 = List.sliceToArray<Nat>(array, 1, 4);
assert slice1 == [2, 3, 4];

let slice2 = List.sliceToArray<Nat>(array, 1, -1);
assert slice2 == [2, 3, 4];
```

Runtime: O(toExclusive - fromInclusive)

Space: O(toExclusive - fromInclusive)

## Function `sliceToVarArray`
``` motoko no-repl
func sliceToVarArray<T>(self : List<T>, fromInclusive : Int, toExclusive : Int) : [var T]
```

Returns a new var array containing elements from `list` starting at index `fromInclusive` up to (but not including) index `toExclusive`.
If the indices are out of bounds, they are clamped to the array bounds.

```motoko include=import
import VarArray "mo:core/VarArray";
import Nat "mo:core/Nat";

let array = List.fromArray<Nat>([1, 2, 3, 4, 5]);

let slice1 = List.sliceToVarArray<Nat>(array, 1, 4);
assert VarArray.equal(slice1, [var 2, 3, 4], Nat.equal);

let slice2 = List.sliceToVarArray<Nat>(array, 1, -1);
assert VarArray.equal(slice2, [var 2, 3, 4], Nat.equal);
```

Runtime: O(toExclusive - fromInclusive)

Space: O(toExclusive - fromInclusive)

## Function `reverseForEachEntry`
``` motoko no-repl
func reverseForEachEntry<T>(self : List<T>, f : (Nat, T) -> ())
```

Like `forEachEntryRev` but iterates through the list in reverse order,
from end to beginning.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";

let list = List.fromArray<Nat>([1, 2, 3]);

List.reverseForEachEntry<Nat>(list, func (i,x) {
  // prints each item (i,x) in list
  Debug.print(Nat.toText(i) # Nat.toText(x));
});
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `reverseForEach`
``` motoko no-repl
func reverseForEach<T>(self : List<T>, f : T -> ())
```

Applies `f` to each element in `list` in reverse order.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";

let list = List.fromArray<Nat>([1, 2, 3]);

List.reverseForEach<Nat>(list, func (x) {
  Debug.print(Nat.toText(x)); // prints each element in list in reverse order
});
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `forEachInRange`
``` motoko no-repl
func forEachInRange<T>(self : List<T>, f : T -> (), fromInclusive : Nat, toExclusive : Nat)
```

Executes the closure over a slice of `list` starting at `fromInclusive` up to (but not including) `toExclusive`.

```motoko include=import
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1, 2, 3, 4, 5]);
List.forEachInRange<Nat>(list, func x = Debug.print(Nat.toText(x)), 1, 2); // prints 2 and 3
```

Runtime: `O(toExclusive - fromExclusive)`

Space: `O(1)`

## Function `contains`
``` motoko no-repl
func contains<T>(self : List<T>, equal : (implicit : (T, T) -> Bool), element : T) : Bool
```

Returns true if the list contains the specified element according to the provided
equality function. Uses the provided `equal` function to compare elements.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 2);
List.add(list, 0);
List.add(list, 3);

assert List.contains<Nat>(list, Nat.equal, 2);
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `equal` runs in O(1) time and space.

## Function `max`
``` motoko no-repl
func max<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order)) : ?T
```

Returns the greatest element in the list according to the ordering defined by `compare`.
Returns `null` if the list is empty.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 1);
List.add(list, 2);

assert List.max<Nat>(list, Nat.compare) == ?2;
assert List.max<Nat>(List.empty<Nat>(), Nat.compare) == null;
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `min`
``` motoko no-repl
func min<T>(self : List<T>, compare : (implicit : (T, T) -> Types.Order)) : ?T
```

Returns the least element in the list according to the ordering defined by `compare`.
Returns `null` if the list is empty.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.empty<Nat>();
List.add(list, 1);
List.add(list, 2);

assert List.min<Nat>(list, Nat.compare) == ?1;
assert List.min<Nat>(List.empty<Nat>(), Nat.compare) == null;
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `equal`
``` motoko no-repl
func equal<T>(self : List<T>, other : List<T>, equal : (implicit : (T, T) -> Bool)) : Bool
```

Tests if two lists are equal by comparing their elements using the provided `equal` function.
Returns true if and only if both lists have the same size and all corresponding elements
are equal according to the provided function.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list1 = List.fromArray<Nat>([1,2]);
let list2 = List.empty<Nat>();
List.add(list2, 1);
List.add(list2, 2);

assert List.equal<Nat>(list1, list2, Nat.equal);
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `equal` runs in O(1) time and space.

## Function `compare`
``` motoko no-repl
func compare<T>(self : List<T>, other : List<T>, compare : (implicit : (T, T) -> Types.Order)) : Types.Order
```

Compares two lists lexicographically using the provided `compare` function.
Elements are compared pairwise until a difference is found or one list ends.
If all elements compare equal, the shorter list is considered less than the longer list.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list1 = List.fromArray<Nat>([0, 1]);
let list2 = List.fromArray<Nat>([2]);
let list3 = List.fromArray<Nat>([0, 1, 2]);

assert List.compare<Nat>(list1, list2, Nat.compare) == #less;
assert List.compare<Nat>(list1, list3, Nat.compare) == #less;
assert List.compare<Nat>(list2, list3, Nat.compare) == #greater;
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `toText`
``` motoko no-repl
func toText<T>(self : List<T>, toText : (implicit : T -> Text)) : Text
```

Creates a textual representation of `list`, using `toText` to recursively
convert the elements into Text.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1,2,3,4]);

assert List.toText<Nat>(list, Nat.toText) == "List[1, 2, 3, 4]";
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `toText` runs in O(1) time and space.

## Function `foldLeft`
``` motoko no-repl
func foldLeft<A, T>(self : List<T>, base : A, combine : (A, T) -> A) : A
```

Collapses the elements in `list` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
left to right.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1,2,3]);

assert List.foldLeft<Text, Nat>(list, "", func (acc, x) { acc # Nat.toText(x)}) == "123";
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `combine` runs in O(1)` time and space.

## Function `foldRight`
``` motoko no-repl
func foldRight<T, A>(self : List<T>, base : A, combine : (T, A) -> A) : A
```

Collapses the elements in `list` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
right to left.

Example:
```motoko include=import
import Nat "mo:core/Nat";

let list = List.fromArray<Nat>([1,2,3]);

assert List.foldRight<Nat, Text>(list, "", func (x, acc) { Nat.toText(x) # acc }) == "123";
```

Runtime: `O(size)`

Space: `O(1)`

*Runtime and space assumes that `combine` runs in O(1)` time and space.

## Function `reverseInPlace`
``` motoko no-repl
func reverseInPlace<T>(self : List<T>)
```

Reverses the order of elements in `list` by overwriting in place.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let list = List.fromArray<Nat>([1,2,3]);

List.reverseInPlace<Nat>(list);
assert Iter.toArray(List.values(list)) == [3, 2, 1];
```

Runtime: `O(size)`

Space: `O(1)`

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : List<T>) : List<T>
```

Returns a new List with the elements from `list` in reverse order.

Example:
```motoko include=import
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let list = List.fromArray<Nat>([1,2,3]);

let rlist = List.reverse<Nat>(list);
assert Iter.toArray(List.values(rlist)) == [3, 2, 1];
```

Runtime: `O(size)`

Space: `O(1)`

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : List<T>) : Bool
```

Returns true if and only if the list is empty.

Example:
```motoko include=import
let list = List.fromArray<Nat>([2,0,3]);
assert not List.isEmpty<Nat>(list);
assert List.isEmpty<Nat>(List.empty<Nat>());
```

Runtime: `O(1)`

Space: `O(1)`

## Function `reader`
``` motoko no-repl
func reader<T>(self : List<T>, start : Nat) : () -> T
```

Unsafe iterator starting from `start`.

Example:
```
let list = List.fromArray<Nat>([1, 2, 3, 4, 5]);
let reader = List.reader<Nat>(list, 2);
assert reader() == 3;
assert reader() == 4;
assert reader() == 5;
```

Runtime: `O(1)`

Space: `O(1)`
