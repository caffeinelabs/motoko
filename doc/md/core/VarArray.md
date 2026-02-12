# core/VarArray
Provides extended utility functions on mutable Arrays (`[var]`).

Note the difference between mutable (`[var]`) and immutable (`[]`) arrays.
Mutable arrays allow their elements to be modified after creation, while
immutable arrays are fixed once created.

WARNING: If you are looking for a list that can grow and shrink in size,
it is recommended you use `List` for those purposes.
Arrays must be created with a fixed size.

Import from the core package to use this module.
```motoko name=import
import VarArray "mo:core/VarArray";
```

## Function `empty`
``` motoko no-repl
func empty<T>() : [var T]
```

Creates an empty mutable array (equivalent to `[var]`).

```motoko include=import
let array = VarArray.empty<Text>();
assert array.size() == 0;
```

Runtime: O(1)

Space: O(1)

## Function `repeat`
``` motoko no-repl
func repeat<T>(item : T, size : Nat) : [var T]
```

Creates a mutable array containing `item` repeated `size` times.

```motoko include=import
import Text "mo:core/Text";

let array = VarArray.repeat<Text>("Echo", 3);
assert VarArray.equal(array, [var "Echo", "Echo", "Echo"], Text.equal);
```

Runtime: O(size)

Space: O(size)

## Function `clone`
``` motoko no-repl
func clone<T>(self : [var T]) : [var T]
```

Duplicates `array`, returning a shallow copy of the original.

```motoko include=import
import Nat "mo:core/Nat";

let array1 = [var 1, 2, 3];
let array2 = VarArray.clone<Nat>(array1);
array2[0] := 0;
assert VarArray.equal(array1, [var 1, 2, 3], Nat.equal);
assert VarArray.equal(array2, [var 0, 2, 3], Nat.equal);
```

Runtime: O(size)

Space: O(size)

## Function `tabulate`
``` motoko no-repl
func tabulate<T>(size : Nat, generator : Nat -> T) : [var T]
```

Creates a mutable array of size `size`. Each element at index i
is created by applying `generator` to i.

```motoko include=import
import Nat "mo:core/Nat";

let array : [var Nat] = VarArray.tabulate<Nat>(4, func i = i * 2);
assert VarArray.equal(array, [var 0, 2, 4, 6], Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `generator` runs in O(1) time and space.

## Function `equal`
``` motoko no-repl
func equal<T>(self : [var T], other : [var T], equal : (implicit : (T, T) -> Bool)) : Bool
```

Tests if two arrays contain equal values (i.e. they represent the same
list of elements). Uses `equal` to compare elements in the arrays.

```motoko include=import
// Use the equal function from the Nat module to compare Nats
import Nat "mo:core/Nat";

let array1 = [var 0, 1, 2, 3];
let array2 = [var 0, 1, 2, 3];
assert VarArray.equal(array1, array2, Nat.equal);
```

Runtime: O(size1 + size2)

Space: O(1)

*Runtime and space assumes that `equal` runs in O(1) time and space.

## Function `find`
``` motoko no-repl
func find<T>(self : [var T], predicate : T -> Bool) : ?T
```

Returns the first value in `array` for which `predicate` returns true.
If no element satisfies the predicate, returns null.

```motoko include=import
let array = [var 1, 9, 4, 8];
let found = VarArray.find<Nat>(array, func x = x > 8);
assert found == ?9;
```
Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `findIndex`
``` motoko no-repl
func findIndex<T>(self : [var T], predicate : T -> Bool) : ?Nat
```

Returns the first index in `array` for which `predicate` returns true.
If no element satisfies the predicate, returns null.

```motoko include=import
let array = [var 'A', 'B', 'C', 'D'];
let found = VarArray.findIndex<Char>(array, func(x) { x == 'C' });
assert found == ?2;
```
Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `concat`
``` motoko no-repl
func concat<T>(self : [var T], other : [var T]) : [var T]
```

Create a new mutable array by concatenating the values of `array1` and `array2`.
Note that `VarArray.concat` copies its arguments and has linear complexity.

```motoko include=import
import Nat "mo:core/Nat";

let array1 = [var 1, 2, 3];
let array2 = [var 4, 5, 6];
let result = VarArray.concat<Nat>(array1, array2);
assert VarArray.equal(result, [var 1, 2, 3, 4, 5, 6], Nat.equal);
```
Runtime: O(size1 + size2)

Space: O(size1 + size2)

## Function `sort`
``` motoko no-repl
func sort<T>(self : [var T], compare : (implicit : (T, T) -> Order.Order)) : [var T]
```

Creates a new sorted copy of the mutable array according to `compare`.
Sort is deterministic and stable.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 4, 2, 6];
let sorted = VarArray.sort(array, Nat.compare);
assert VarArray.equal(sorted, [var 2, 4, 6], Nat.equal);
```
Runtime: O(size * log(size))

Space: O(size)
*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `sortInPlace`
``` motoko no-repl
func sortInPlace<T>(self : [var T], compare : (implicit : (T, T) -> Order.Order)) : ()
```

Sorts the elements in a mutable array in place according to `compare`.
Sort is deterministic and stable. This modifies the original array.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 4, 2, 6];
VarArray.sortInPlace(array, Nat.compare);
assert VarArray.equal(array, [var 2, 4, 6], Nat.equal);
```
Runtime: O(size * log(size))

Space: O(size)
*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : [var T]) : [var T]
```

Creates a new mutable array by reversing the order of elements in `array`.
The original array is not modified.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 10, 11, 12];
let reversed = VarArray.reverse(array);
assert VarArray.equal(reversed, [var 12, 11, 10], Nat.equal);
```

Runtime: O(size)

Space: O(1)

## Function `reverseInPlace`
``` motoko no-repl
func reverseInPlace<T>(self : [var T]) : ()
```

Reverses the order of elements in a mutable array in place.
This modifies the original array.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 10, 11, 12];
VarArray.reverseInPlace(array);
assert VarArray.equal(array, [var 12, 11, 10], Nat.equal);
```

Runtime: O(size)

Space: O(1)

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : [var T], f : T -> ())
```

Calls `f` with each element in `array`.
Retains original ordering of elements.

```motoko include=import
var sum = 0;
let array = [var 0, 1, 2, 3];
VarArray.forEach<Nat>(array, func(x) {
  sum += x;
});
assert sum == 6;
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `map`
``` motoko no-repl
func map<T, R>(self : [var T], f : T -> R) : [var R]
```

Creates a new mutable array by applying `f` to each element in `array`. `f` "maps"
each element it is applied to of type `T` to an element of type `R`.
Retains original ordering of elements.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 0, 1, 2, 3];
let array2 = VarArray.map<Nat, Nat>(array, func x = x * 2);
assert VarArray.equal(array2, [var 0, 2, 4, 6], Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `mapInPlace`
``` motoko no-repl
func mapInPlace<T>(self : [var T], f : T -> T)
```

Applies `f` to each element of `array` in place,
retaining the original ordering of elements.
This modifies the original array.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 0, 1, 2, 3];
VarArray.mapInPlace<Nat>(array, func x = x * 3);
assert VarArray.equal(array, [var 0, 3, 6, 9], Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `filter`
``` motoko no-repl
func filter<T>(self : [var T], f : T -> Bool) : [var T]
```

Creates a new mutable array by applying `predicate` to every element
in `array`, retaining the elements for which `predicate` returns true.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 4, 2, 6, 1, 5];
let evenElements = VarArray.filter<Nat>(array, func x = x % 2 == 0);
assert VarArray.equal(evenElements, [var 4, 2, 6], Nat.equal);
```
Runtime: O(size)

Space: O(size)
*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `filterMap`
``` motoko no-repl
func filterMap<T, R>(self : [var T], f : T -> ?R) : [var R]
```

Creates a new mutable array by applying `f` to each element in `array`,
and keeping all non-null elements. The ordering is retained.

```motoko include=import
import Nat "mo:core/Nat";
import Text "mo:core/Text";

let array = [var 4, 2, 0, 1];
let newArray =
  VarArray.filterMap<Nat, Text>( // mapping from Nat to Text values
    array,
    func x = if (x == 0) { null } else { ?Nat.toText(100 / x) } // can't divide by 0, so return null
  );
assert VarArray.equal(newArray, [var "25", "50", "100"], Text.equal);
```
Runtime: O(size)

Space: O(size)
*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `mapResult`
``` motoko no-repl
func mapResult<T, R, E>(self : [var T], f : T -> Result.Result<R, E>) : Result.Result<[var R], E>
```

Creates a new mutable array by applying `f` to each element in `array`.
If any invocation of `f` produces an `#err`, returns an `#err`. Otherwise
returns an `#ok` containing the new array.

```motoko include=import
import Result "mo:core/Result";

let array = [var 4, 3, 2, 1, 0];
// divide 100 by every element in the array
let result = VarArray.mapResult<Nat, Nat, Text>(array, func x {
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
@deprecated M0235

## Function `mapEntries`
``` motoko no-repl
func mapEntries<T, R>(self : [var T], f : (T, Nat) -> R) : [var R]
```

Creates a new array by applying `f` to each element in `array` and its index.
Retains original ordering of elements.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 10, 10, 10, 10];
let newArray = VarArray.mapEntries<Nat, Nat>(array, func (x, i) = i * x);
assert VarArray.equal(newArray, [var 0, 10, 20, 30], Nat.equal);
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `flatMap`
``` motoko no-repl
func flatMap<T, R>(self : [var T], k : T -> Types.Iter<R>) : [var R]
```

Creates a new mutable array by applying `k` to each element in `array`,
and concatenating the resulting arrays in order.

```motoko include=import
import Int "mo:core/Int"

let array = [var 1, 2, 3, 4];
let newArray = VarArray.flatMap<Nat, Int>(array, func x = [x, -x].vals());
assert VarArray.equal(newArray, [var 1, -1, 2, -2, 3, -3, 4, -4], Int.equal);
```
Runtime: O(size)

Space: O(size)
*Runtime and space assumes that `k` runs in O(1) time and space.

## Function `foldLeft`
``` motoko no-repl
func foldLeft<T, A>(self : [var T], base : A, combine : (A, T) -> A) : A
```

Collapses the elements in `array` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
left to right.

```motoko include=import
import {add} "mo:core/Nat";

let array = [var 4, 2, 0, 1];
let sum =
  VarArray.foldLeft<Nat, Nat>(
    array,
    0, // start the sum at 0
    func(sumSoFar, x) = sumSoFar + x // this entire function can be replaced with `add`!
  );
assert sum == 7;
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `combine` runs in O(1) time and space.

## Function `foldRight`
``` motoko no-repl
func foldRight<T, A>(self : [var T], base : A, combine : (T, A) -> A) : A
```

Collapses the elements in `array` into a single value by starting with `base`
and progessively combining elements into `base` with `combine`. Iteration runs
right to left.

```motoko include=import
import {toText} "mo:core/Nat";

let array = [var 1, 9, 4, 8];
let bookTitle = VarArray.foldRight<Nat, Text>(array, "", func(x, acc) = toText(x) # acc);
assert bookTitle == "1948";
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `combine` runs in O(1) time and space.

## Function `join`
``` motoko no-repl
func join<T>(self : Types.Iter<[var T]>) : [var T]
```

Combines an iterator of mutable arrays into a single mutable array.
Retains the original ordering of the elements.

Consider using `VarArray.flatten()` for better performance.

```motoko include=import
import Nat "mo:core/Nat";

let arrays : [[var Nat]] = [[var 0, 1, 2], [var 2, 3], [var], [var 4]];
let joinedArray = VarArray.join<Nat>(arrays.vals());
assert VarArray.equal(joinedArray, [var 0, 1, 2, 2, 3, 4], Nat.equal);
```

Runtime: O(number of elements in array)

Space: O(number of elements in array)

## Function `flatten`
``` motoko no-repl
func flatten<T>(self : [var [var T]]) : [var T]
```

Combines a mutable array of mutable arrays into a single mutable array. Retains the original
ordering of the elements.

This has better performance compared to `VarArray.join()`.

```motoko include=import
import Nat "mo:core/Nat";

let arrays : [var [var Nat]] = [var [var 0, 1, 2], [var 2, 3], [var], [var 4]];
let flatArray = VarArray.flatten<Nat>(arrays);
assert VarArray.equal(flatArray, [var 0, 1, 2, 2, 3, 4], Nat.equal);
```

Runtime: O(number of elements in array)

Space: O(number of elements in array)

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : [var T]
```

Create an array containing a single value.

```motoko include=import
import Nat "mo:core/Nat";

let array = VarArray.singleton<Nat>(2);
assert VarArray.equal(array, [var 2], Nat.equal);
```

Runtime: O(1)

Space: O(1)

## Function `size`
``` motoko no-repl
func size<T>(self : [var T]) : Nat
```

Returns the size of a mutable array. Equivalent to `array.size()`.

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : [var T]) : Bool
```

Returns whether a mutable array is empty, i.e. contains zero elements.

## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T]) : [var T]
```

Transforms an immutable array into a mutable array.

```motoko include=import
let array = [0, 1, 2];
let varArray = VarArray.fromArray<Nat>(array);
assert varArray.size() == 3;
```

Runtime: O(size)

Space: O(1)
@deprecated M0235

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Types.Iter<T>) : [var T]
```

Converts an iterator to a mutable array.

## Function `keys`
``` motoko no-repl
func keys<T>(self : [var T]) : Types.Iter<Nat>
```

Returns an iterator (`Iter`) over the indices of `array`.
An iterator provides a single method `next()`, which returns
indices in order, or `null` when out of index to iterate over.

NOTE: You can also use `array.keys()` instead of this function. See example
below.

```motoko include=import
let array = [var 10, 11, 12];

var sum = 0;
for (element in array.keys()) {
  sum += element;
};
assert sum == 3; // 0 + 1 + 2
```

Runtime: O(1)

Space: O(1)

## Function `values`
``` motoko no-repl
func values<T>(self : [var T]) : Types.Iter<T>
```

Iterator provides a single method `next()`, which returns
elements in order, or `null` when out of elements to iterate over.

Note: You can also use `array.values()` instead of this function. See example
below.

```motoko include=import
let array = [var 10, 11, 12];

var sum = 0;
for (element in array.values()) {
  sum += element;
};
assert sum == 33; // 10 + 11 + 12
```

Runtime: O(1)

Space: O(1)

## Function `enumerate`
``` motoko no-repl
func enumerate<T>(self : [var T]) : Types.Iter<(Nat, T)>
```

Returns an iterator that provides pairs of (index, element) in order, or `null`
when out of elements to iterate over.

```motoko include=import
let array = [var 10, 11, 12];

var sum = 0;
for ((index, element) in VarArray.enumerate(array)) {
  sum += element;
};
assert sum == 33;
```

Runtime: O(1)

Space: O(1)

## Function `all`
``` motoko no-repl
func all<T>(self : [var T], predicate : T -> Bool) : Bool
```

Returns true if all elements in `array` satisfy the predicate function.

```motoko include=import
let array = [var 1, 2, 3, 4];
assert VarArray.all<Nat>(array, func x = x > 0);
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `any`
``` motoko no-repl
func any<T>(self : [var T], predicate : T -> Bool) : Bool
```

Returns true if any element in `array` satisfies the predicate function.

```motoko include=import
let array = [var 1, 2, 3, 4];
assert VarArray.any<Nat>(array, func x = x > 3);
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `indexOf`
``` motoko no-repl
func indexOf<T>(self : [var T], equal : (implicit : (T, T) -> Bool), element : T) : ?Nat
```

Returns the index of the first `element` in the `array`.

```motoko include=import
import Char "mo:core/Char";

let array = [var 'c', 'o', 'f', 'f', 'e', 'e'];
assert VarArray.indexOf<Char>(array, Char.equal, 'c') == ?0;
assert VarArray.indexOf<Char>(array, Char.equal, 'f') == ?2;
assert VarArray.indexOf<Char>(array, Char.equal, 'g') == null;
```

Runtime: O(array.size())

Space: O(1)

## Function `nextIndexOf`
``` motoko no-repl
func nextIndexOf<T>(self : [var T], equal : (implicit : (T, T) -> Bool), element : T, fromInclusive : Nat) : ?Nat
```

Returns the index of the next occurence of `element` in the `array` starting from the `from` index (inclusive).

```motoko include=import
import Char "mo:core/Char";

let array = [var 'c', 'o', 'f', 'f', 'e', 'e'];
assert VarArray.nextIndexOf<Char>(array, Char.equal, 'c', 0) == ?0;
assert VarArray.nextIndexOf<Char>(array, Char.equal, 'f', 0) == ?2;
assert VarArray.nextIndexOf<Char>(array, Char.equal, 'f', 2) == ?2;
assert VarArray.nextIndexOf<Char>(array, Char.equal, 'f', 3) == ?3;
assert VarArray.nextIndexOf<Char>(array, Char.equal, 'f', 4) == null;
```

Runtime: O(array.size())

Space: O(1)

## Function `lastIndexOf`
``` motoko no-repl
func lastIndexOf<T>(self : [var T], equal : (implicit : (T, T) -> Bool), element : T) : ?Nat
```

Returns the index of the last `element` in the `array`.

```motoko include=import
import Char "mo:core/Char";

let array = [var 'c', 'o', 'f', 'f', 'e', 'e'];
assert VarArray.lastIndexOf<Char>(array, Char.equal, 'c') == ?0;
assert VarArray.lastIndexOf<Char>(array, Char.equal, 'f') == ?3;
assert VarArray.lastIndexOf<Char>(array, Char.equal, 'e') == ?5;
assert VarArray.lastIndexOf<Char>(array, Char.equal, 'g') == null;
```

Runtime: O(array.size())

Space: O(1)

## Function `prevIndexOf`
``` motoko no-repl
func prevIndexOf<T>(self : [var T], equal : (implicit : (T, T) -> Bool), element : T, fromExclusive : Nat) : ?Nat
```

Returns the index of the previous occurence of `element` in the `array` starting from the `from` index (exclusive).

```motoko include=import
import Char "mo:core/Char";
let array = [var 'c', 'o', 'f', 'f', 'e', 'e'];
assert VarArray.prevIndexOf<Char>(array, Char.equal, 'c', array.size()) == ?0;
assert VarArray.prevIndexOf<Char>(array, Char.equal, 'e', array.size()) == ?5;
assert VarArray.prevIndexOf<Char>(array, Char.equal, 'e', 5) == ?4;
assert VarArray.prevIndexOf<Char>(array, Char.equal, 'e', 4) == null;
```

Runtime: O(array.size());
Space: O(1);

## Function `range`
``` motoko no-repl
func range<T>(self : [var T], fromInclusive : Int, toExclusive : Int) : Types.Iter<T>
```

Returns an iterator over a slice of `array` starting at `fromInclusive` up to (but not including) `toExclusive`.

Negative indices are relative to the end of the array. For example, `-1` corresponds to the last element in the array.

If the indices are out of bounds, they are clamped to the array bounds.
If the first index is greater than the second, the function returns an empty iterator.

```motoko include=import
let array = [var 1, 2, 3, 4, 5];
let iter1 = VarArray.range<Nat>(array, 3, array.size());
assert iter1.next() == ?4;
assert iter1.next() == ?5;
assert iter1.next() == null;

let iter2 = VarArray.range<Nat>(array, 3, -1);
assert iter2.next() == ?4;
assert iter2.next() == null;

let iter3 = VarArray.range<Nat>(array, 0, 0);
assert iter3.next() == null;
```

Runtime: O(1)

Space: O(1)

## Function `sliceToArray`
``` motoko no-repl
func sliceToArray<T>(self : [var T], fromInclusive : Int, toExclusive : Int) : [T]
```

Returns a new array containing elements from `array` starting at index `fromInclusive` up to (but not including) index `toExclusive`.
If the indices are out of bounds, they are clamped to the array bounds.

```motoko include=import
let array = [var 1, 2, 3, 4, 5];

let slice1 = VarArray.sliceToArray<Nat>(array, 1, 4);
assert slice1 == [2, 3, 4];

let slice2 = VarArray.sliceToArray<Nat>(array, 1, -1);
assert slice2 == [2, 3, 4];
```

Runtime: O(toExclusive - fromInclusive)

Space: O(toExclusive - fromInclusive)

## Function `sliceToVarArray`
``` motoko no-repl
func sliceToVarArray<T>(self : [var T], fromInclusive : Int, toExclusive : Int) : [var T]
```

Returns a new mutable array containing elements from `array` starting at index `fromInclusive` up to (but not including) index `toExclusive`.
If the indices are out of bounds, they are clamped to the array bounds.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 1, 2, 3, 4, 5];

let slice1 = VarArray.sliceToVarArray<Nat>(array, 1, 4);
assert VarArray.equal(slice1, [var 2, 3, 4], Nat.equal);

let slice2 = VarArray.sliceToVarArray<Nat>(array, 1, -1);
assert VarArray.equal(slice2, [var 2, 3, 4], Nat.equal);
```

Runtime: O(toExclusive - fromInclusive)

Space: O(toExclusive - fromInclusive)

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : [var T]) : [T]
```

Transforms a mutable array into an immutable array.

```motoko include=import
let varArray = [var 0, 1, 2];
varArray[2] := 3;
let array = VarArray.toArray<Nat>(varArray);
assert array == [0, 1, 3];
```

Runtime: O(size)

Space: O(1)

## Function `toText`
``` motoko no-repl
func toText<T>(self : [var T], f : (implicit : (toText : T -> Text))) : Text
```

Converts the mutable array to its textual representation using `f` to convert each element to `Text`.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 1, 2, 3];
assert VarArray.toText<Nat>(array, Nat.toText) == "[var 1, 2, 3]";
```

Runtime: O(size)

Space: O(size)

*Runtime and space assumes that `f` runs in O(1) time and space.

## Function `compare`
``` motoko no-repl
func compare<T>(self : [var T], other : [var T], compare : (implicit : (T, T) -> Order.Order)) : Order.Order
```

Compares two mutable arrays using the provided comparison function for elements.
Returns #less, #equal, or #greater if `array1` is less than, equal to,
or greater than `array2` respectively.

If arrays have different sizes but all elements up to the shorter length are equal,
the shorter array is considered #less than the longer array.

```motoko include=import
import Nat "mo:core/Nat";
let array1 = [var 1, 2, 3];
let array2 = [var 1, 2, 4];
assert VarArray.compare<Nat>(array1, array2, Nat.compare) == #less;

let array3 = [var 1, 2];
let array4 = [var 1, 2, 3];
assert VarArray.compare<Nat>(array3, array4, Nat.compare) == #less;
```

Runtime: O(min(size1, size2))

Space: O(1)

*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `binarySearch`
``` motoko no-repl
func binarySearch<T>(self : [var T], compare : (implicit : (T, T) -> Order.Order), element : T) : {#found : Nat; #insertionIndex : Nat}
```

Performs binary search on a sorted mutable array to find the index of the `element`.
Returns `#found(index)` if the element is found, or `#insertionIndex(index)` with the index

If there are multiple equal elements, no guarantee is made about which index is returned.
The array must be sorted in ascending order according to the `compare` function.

```motoko include=import
import Nat "mo:core/Nat";

let sorted = [var 1, 3, 5, 7, 9, 11];
assert VarArray.binarySearch<Nat>(sorted, Nat.compare, 5) == #found(2);
assert VarArray.binarySearch<Nat>(sorted, Nat.compare, 6) == #insertionIndex(3);
```

Runtime: O(log(size))

Space: O(1)

*Runtime and space assumes that `compare` runs in O(1) time and space.

## Function `isSorted`
``` motoko no-repl
func isSorted<T>(self : [var T], compare : (implicit : (T, T) -> Order.Order)) : Bool
```

Checks whether the mutable `array` is sorted according to the `compare` function.

```motoko include=import
import Nat "mo:core/Nat";

let array = [var 1, 2, 3];
assert VarArray.isSorted<Nat>(array, Nat.compare);
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `compare` runs in O(1) time and space.
