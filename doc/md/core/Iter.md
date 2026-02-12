# core/Iter
Utilities for `Iter` (iterator) values.

Iterators are a way to represent sequences of values that can be lazily produced.
They can be used to:
- Iterate over collections.
- Represent collections that are too large to fit in memory or that are produced incrementally.
- Transform collections without creating intermediate collections.

Iterators are inherently stateful. Calling `next` "consumes" a value from
the Iterator that cannot be put back, so keep that in mind when sharing
iterators between consumers.

```motoko name=import
import Iter "mo:core/Iter";
```


An iterator can be iterated over using a `for` loop:
```motoko
let iter = [1, 2, 3].values();
for (x in iter) {
  // do something with x...
}
```

Iterators can be:
- created from other collections (e.g. using `values` or `keys` function on a `Map`) or from scratch (e.g. using `empty` or `singleton`).
- transformed using `map`, `filter`, `concat`, etc. Which can be used to compose several transformations together without materializing intermediate collections.
- consumed using `forEach`, `size`, `toArray`, etc.
- combined using `concat`.

## Type `Iter`
``` motoko no-repl
type Iter<T> = Types.Iter<T>
```

An iterator that produces values of type `T`. Calling `next` returns
`null` when iteration is finished.

Iterators are inherently stateful. Calling `next` "consumes" a value from
the Iterator that cannot be put back, so keep that in mind when sharing
iterators between consumers.

An iterator `i` can be iterated over using
```motoko
let iter = [1, 2, 3].values();
for (x in iter) {
  // do something with x...
}
```

## Function `empty`
``` motoko no-repl
func empty<T>() : Iter<T>
```

Creates an empty iterator.

```motoko include=import
for (x in Iter.empty<Nat>())
  assert false; // This loop body will never run
```

## Function `singleton`
``` motoko no-repl
func singleton<T>(value : T) : Iter<T>
```

Creates an iterator that produces a single value.

```motoko include=import
var sum = 0;
for (x in Iter.singleton(3))
  sum += x;
assert sum == 3;
```

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : Iter<T>, f : (T) -> ())
```

Calls a function `f` on every value produced by an iterator and discards
the results. If you're looking to keep these results use `map` instead.

```motoko include=import
var sum = 0;
Iter.forEach<Nat>([1, 2, 3].values(), func(x) {
  sum += x;
});
assert sum == 6;
```

## Function `enumerate`
``` motoko no-repl
func enumerate<T>(self : Iter<T>) : Iter<(Nat, T)>
```

Takes an iterator and returns a new iterator that pairs each element with its index.
The index starts at 0 and increments by 1 for each element.

```motoko include=import
let iter = Iter.fromArray(["A", "B", "C"]);
let enumerated = Iter.enumerate(iter);
let result = Iter.toArray(enumerated);
assert result == [(0, "A"), (1, "B"), (2, "C")];
```

## Function `step`
``` motoko no-repl
func step<T>(self : Iter<T>, n : Nat) : Iter<T>
```

Creates a new iterator that yields every nth element from the original iterator.
If `interval` is 0, returns an empty iterator. If `interval` is 1, returns the original iterator.
For any other positive interval, returns an iterator that skips `interval - 1` elements after each yielded element.

```motoko include=import
let iter = Iter.fromArray([1, 2, 3, 4, 5, 6]);
let steppedIter = Iter.step(iter, 2); // Take every 2nd element
assert ?1 == steppedIter.next();
assert ?3 == steppedIter.next();
assert ?5 == steppedIter.next();
assert null == steppedIter.next();
```

## Function `size`
``` motoko no-repl
func size<T>(self : Iter<T>) : Nat
```

Consumes an iterator and counts how many elements were produced (discarding them in the process).
```motoko include=import
let iter = [1, 2, 3].values();
assert 3 == Iter.size(iter);
```

## Function `map`
``` motoko no-repl
func map<T, R>(self : Iter<T>, f : T -> R) : Iter<R>
```

Takes a function and an iterator and returns a new iterator that lazily applies
the function to every element produced by the argument iterator.
```motoko include=import
let iter = [1, 2, 3].values();
let mappedIter = Iter.map<Nat, Nat>(iter, func (x) = x * 2);
let result = Iter.toArray(mappedIter);
assert result == [2, 4, 6];
```

## Function `filter`
``` motoko no-repl
func filter<T>(self : Iter<T>, f : T -> Bool) : Iter<T>
```

Creates a new iterator that only includes elements from the original iterator
for which the predicate function returns true.

```motoko include=import
let iter = [1, 2, 3, 4, 5].values();
let evenNumbers = Iter.filter<Nat>(iter, func (x) = x % 2 == 0);
let result = Iter.toArray(evenNumbers);
assert result == [2, 4];
```

## Function `filterMap`
``` motoko no-repl
func filterMap<T, R>(self : Iter<T>, f : T -> ?R) : Iter<R>
```

Creates a new iterator by applying a transformation function to each element
of the original iterator. Elements for which the function returns null are
excluded from the result.

```motoko include=import
let iter = [1, 2, 3].values();
let evenNumbers = Iter.filterMap<Nat, Nat>(iter, func (x) = if (x % 2 == 0) ?x else null);
let result = Iter.toArray(evenNumbers);
assert result == [2];
```

## Function `flatten`
``` motoko no-repl
func flatten<T>(self : Iter<Iter<T>>) : Iter<T>
```

Flattens an iterator of iterators into a single iterator by concatenating the inner iterators.

Possible optimization: Use `flatMap` when you need to transform elements before calling `flatten`. Example: use `flatMap(...)` instead of `flatten(map(...))`.
```motoko include=import
let iter = Iter.flatten([[1, 2].values(), [3].values(), [4, 5, 6].values()].values());
let result = Iter.toArray(iter);
assert result == [1, 2, 3, 4, 5, 6];
```

## Function `flatMap`
``` motoko no-repl
func flatMap<T, R>(self : Iter<T>, f : T -> Iter<R>) : Iter<R>
```

Transforms every element of an iterator into an iterator and concatenates the results.
```motoko include=import
let iter = Iter.flatMap<Nat, Nat>([1, 3, 5].values(), func (x) = [x, x + 1].values());
let result = Iter.toArray(iter);
assert result == [1, 2, 3, 4, 5, 6];
```

## Function `take`
``` motoko no-repl
func take<T>(self : Iter<T>, n : Nat) : Iter<T>
```

Returns a new iterator that yields at most, first `n` elements from the original iterator.
After `n` elements have been produced or the original iterator is exhausted,
subsequent calls to `next()` will return `null`.

```motoko include=import
let iter = Iter.fromArray([1, 2, 3, 4, 5]);
let first3 = Iter.take(iter, 3);
let result = Iter.toArray(first3);
assert result == [1, 2, 3];
```

```motoko include=import
let iter = Iter.fromArray([1, 2, 3]);
let first5 = Iter.take(iter, 5);
let result = Iter.toArray(first5);
assert result == [1, 2, 3]; // only 3 elements in the original iterator
```

## Function `takeWhile`
``` motoko no-repl
func takeWhile<T>(self : Iter<T>, f : T -> Bool) : Iter<T>
```

Returns a new iterator that yields elements from the original iterator until the predicate function returns false.
The first element for which the predicate returns false is not included in the result.

```motoko include=import
let iter = Iter.fromArray([1, 2, 3, 4, 5, 4, 3, 2, 1]);
let result = Iter.takeWhile<Nat>(iter, func (x) = x < 4);
let array = Iter.toArray(result);
assert array == [1, 2, 3]; // note the difference between `takeWhile` and `filter`
```

## Function `drop`
``` motoko no-repl
func drop<T>(self : Iter<T>, n : Nat) : Iter<T>
```

Returns a new iterator that skips the first `n` elements from the original iterator.
If the original iterator has fewer than `n` elements, the result will be an empty iterator.

```motoko include=import
let iter = Iter.fromArray([1, 2, 3, 4, 5]);
let skipped = Iter.drop(iter, 3);
let result = Iter.toArray(skipped);
assert result == [4, 5];
```

## Function `dropWhile`
``` motoko no-repl
func dropWhile<T>(self : Iter<T>, f : T -> Bool) : Iter<T>
```

Returns a new iterator that skips elements from the original iterator until the predicate function returns false.
The first element for which the predicate returns false is the first element produced by the new iterator.

```motoko include=import
let iter = Iter.fromArray([1, 2, 3, 4, 5, 4, 3, 2, 1]);
let result = Iter.dropWhile<Nat>(iter, func (x) = x < 4);
let array = Iter.toArray(result);
assert array == [4, 5, 4, 3, 2, 1]; // notice that `takeWhile` and `dropWhile` are complementary
```

## Function `zip`
``` motoko no-repl
func zip<A, B>(self : Iter<A>, other : Iter<B>) : Iter<(A, B)>
```

Zips two iterators into a single iterator that produces pairs of elements.
The resulting iterator will stop producing elements when either of the input iterators is exhausted.

```motoko include=import
let iter1 = [1, 2, 3].values();
let iter2 = ["A", "B"].values();
let zipped = Iter.zip(iter1, iter2);
let result = Iter.toArray(zipped);
assert result == [(1, "A"), (2, "B")]; // note that the third element from iter1 is not included, because iter2 is exhausted
```

## Function `zip3`
``` motoko no-repl
func zip3<A, B, C>(self : Iter<A>, other1 : Iter<B>, other2 : Iter<C>) : Iter<(A, B, C)>
```

Zips three iterators into a single iterator that produces triples of elements.
The resulting iterator will stop producing elements when any of the input iterators is exhausted.

```motoko include=import
let iter1 = ["A", "B"].values();
let iter2 = ["1", "2", "3"].values();
let iter3 = ["x", "y", "z", "xd"].values();
let zipped = Iter.zip3(iter1, iter2, iter3);
let result = Iter.toArray(zipped);
assert result == [("A", "1", "x"), ("B", "2", "y")]; // note that the unmatched elements from iter2 and iter3 are not included
```

## Function `zipWith`
``` motoko no-repl
func zipWith<A, B, R>(self : Iter<A>, other : Iter<B>, f : (A, B) -> R) : Iter<R>
```

Zips two iterators into a single iterator by applying a function to zipped pairs of elements.
The resulting iterator will stop producing elements when either of the input iterators is exhausted.

```motoko include=import
let iter1 = ["A", "B"].values();
let iter2 = ["1", "2", "3"].values();
let zipped = Iter.zipWith<Text, Text, Text>(iter1, iter2, func (a, b) = a # b);
let result = Iter.toArray(zipped);
assert result == ["A1", "B2"]; // note that the third element from iter2 is not included, because iter1 is exhausted
```

## Function `zipWith3`
``` motoko no-repl
func zipWith3<A, B, C, R>(self : Iter<A>, other1 : Iter<B>, other2 : Iter<C>, f : (A, B, C) -> R) : Iter<R>
```

Zips three iterators into a single iterator by applying a function to zipped triples of elements.
The resulting iterator will stop producing elements when any of the input iterators is exhausted.

```motoko include=import
let iter1 = ["A", "B"].values();
let iter2 = ["1", "2", "3"].values();
let iter3 = ["x", "y", "z", "xd"].values();
let zipped = Iter.zipWith3<Text, Text, Text, Text>(iter1, iter2, iter3, func (a, b, c) = a # b # c);
let result = Iter.toArray(zipped);
assert result == ["A1x", "B2y"]; // note that the unmatched elements from iter2 and iter3 are not included
```

## Function `all`
``` motoko no-repl
func all<T>(self : Iter<T>, f : T -> Bool) : Bool
```

Checks if a predicate function is true for all elements produced by an iterator.
It stops consuming elements from the original iterator as soon as the predicate returns false.

```motoko include=import
assert Iter.all<Nat>([1, 2, 3].values(), func (x) = x < 4);
assert not Iter.all<Nat>([1, 2, 3].values(), func (x) = x < 3);
```

## Function `any`
``` motoko no-repl
func any<T>(self : Iter<T>, f : T -> Bool) : Bool
```

Checks if a predicate function is true for any element produced by an iterator.
It stops consuming elements from the original iterator as soon as the predicate returns true.

```motoko include=import
assert Iter.any<Nat>([1, 2, 3].values(), func (x) = x == 2);
assert not Iter.any<Nat>([1, 2, 3].values(), func (x) = x == 4);
```

## Function `find`
``` motoko no-repl
func find<T>(self : Iter<T>, f : T -> Bool) : ?T
```

Finds the first element produced by an iterator for which a predicate function returns true.
Returns `null` if no such element is found.
It stops consuming elements from the original iterator as soon as the predicate returns true.

```motoko include=import
let iter = [1, 2, 3, 4].values();
assert ?2 == Iter.find<Nat>(iter, func (x) = x % 2 == 0);
```

## Function `findIndex`
``` motoko no-repl
func findIndex<T>(self : Iter<T>, predicate : T -> Bool) : ?Nat
```

Returns the first index in `array` for which `predicate` returns true.
If no element satisfies the predicate, returns null.

```motoko include=import
let iter = ['A', 'B', 'C', 'D'].values();
let found = Iter.findIndex<Char>(iter, func(x) { x == 'C' });
assert found == ?2;
```
Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `contains`
``` motoko no-repl
func contains<T>(self : Iter<T>, equal : (implicit : (T, T) -> Bool), value : T) : Bool
```

Checks if an element is produced by an iterator.
It stops consuming elements from the original iterator as soon as the predicate returns true.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3, 4].values();
assert Iter.contains<Nat>(iter, Nat.equal, 2);
```

## Function `foldLeft`
``` motoko no-repl
func foldLeft<T, R>(self : Iter<T>, initial : R, combine : (R, T) -> R) : R
```

Reduces an iterator to a single value by applying a function to each element and an accumulator.
The accumulator is initialized with the `initial` value.
It starts applying the `combine` function starting from the `initial` accumulator value and the first elements produced by the iterator.

```motoko include=import
let iter = ["A", "B", "C"].values();
let result = Iter.foldLeft<Text, Text>(iter, "S", func (acc, x) = "(" # acc # x # ")");
assert result == "(((SA)B)C)";
```

## Function `foldRight`
``` motoko no-repl
func foldRight<T, R>(self : Iter<T>, initial : R, combine : (T, R) -> R) : R
```

Reduces an iterator to a single value by applying a function to each element in reverse order and an accumulator.
The accumulator is initialized with the `initial` value and it is first combined with the last element produced by the iterator.
It starts applying the `combine` function starting from the last elements produced by the iterator.

**Performance note**: Since this function needs to consume the entire iterator to reverse it,
it has to materialize the entire iterator in memory to get to the last element to start applying the `combine` function.
**Use `foldLeft` or `reduce` when possible to avoid the extra memory overhead**.

```motoko include=import
let iter = ["A", "B", "C"].values();
let result = Iter.foldRight<Text, Text>(iter, "S", func (x, acc) = "(" # x # acc # ")");
assert result == "(A(B(CS)))";
```

## Function `reduce`
``` motoko no-repl
func reduce<T>(self : Iter<T>, combine : (T, T) -> T) : ?T
```

Reduces an iterator to a single value by applying a function to each element, starting with the first elements.
The accumulator is initialized with the first element produced by the iterator.
When the iterator is empty, it returns `null`.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3].values();
assert ?6 == Iter.reduce<Nat>(iter, Nat.add);
```

## Function `scanLeft`
``` motoko no-repl
func scanLeft<T, R>(self : Iter<T>, initial : R, combine : (R, T) -> R) : Iter<R>
```

Produces an iterator containing cumulative results of applying the `combine` operator going left to right, including the `initial` value.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3].values();
let scanned = Iter.scanLeft<Nat, Nat>(iter, 0, Nat.add);
let result = Iter.toArray(scanned);
assert result == [0, 1, 3, 6];
```

## Function `scanRight`
``` motoko no-repl
func scanRight<T, R>(self : Iter<T>, initial : R, combine : (T, R) -> R) : Iter<R>
```

Produces an iterator containing cumulative results of applying the `combine` operator going right to left, including the `initial` value.

**Performance note**: Since this function needs to consume the entire iterator to reverse it,
it has to materialize the entire iterator in memory to get to the last element to start applying the `combine` function.
**Use `scanLeft` when possible to avoid the extra memory overhead**.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3].values();
let scanned = Iter.scanRight<Nat, Nat>(iter, 0, Nat.add);
let result = Iter.toArray(scanned);
assert result == [0, 3, 5, 6];
```

## Function `unfold`
``` motoko no-repl
func unfold<T, S>(initial : S, step : S -> ?(T, S)) : Iter<T>
```

Creates an iterator that produces elements using the `step` function starting from the `initial` value.
The `step` function takes the current state and returns the next element and the next state, or `null` if the iteration is finished.

```motoko include=import
let iter = Iter.unfold<Nat, Nat>(1, func (x) = if (x <= 3) ?(x, x + 1) else null);
let result = Iter.toArray(iter);
assert result == [1, 2, 3];
```

## Function `max`
``` motoko no-repl
func max<T>(self : Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : ?T
```

Consumes an iterator and returns the first maximum element produced by the iterator.
If the iterator is empty, it returns `null`.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3].values();
assert ?3 == Iter.max<Nat>(iter, Nat.compare);
```

## Function `min`
``` motoko no-repl
func min<T>(self : Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : ?T
```

Consumes an iterator and returns the first minimum element produced by the iterator.
If the iterator is empty, it returns `null`.

```motoko include=import
import Nat "mo:core/Nat";

let iter = [1, 2, 3].values();
assert ?1 == Iter.min<Nat>(iter, Nat.compare);
```

## Function `infinite`
``` motoko no-repl
func infinite<T>(item : T) : Iter<T>
```

Creates an iterator that produces an infinite sequence of `x`.
```motoko include=import
let iter = Iter.infinite(10);
assert ?10 == iter.next();
assert ?10 == iter.next();
assert ?10 == iter.next();
// ...
```

## Function `concat`
``` motoko no-repl
func concat<T>(self : Iter<T>, other : Iter<T>) : Iter<T>
```

Takes two iterators and returns a new iterator that produces
elements from the original iterators sequentally.
```motoko include=import
let iter1 = [1, 2].values();
let iter2 = [5, 6, 7].values();
let concatenatedIter = Iter.concat(iter1, iter2);
let result = Iter.toArray(concatenatedIter);
assert result == [1, 2, 5, 6, 7];
```

## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T]) : Iter<T>
```

Creates an iterator that produces the elements of an Array in ascending index order.
```motoko include=import
let iter = Iter.fromArray([1, 2, 3]);
assert ?1 == iter.next();
assert ?2 == iter.next();
assert ?3 == iter.next();
assert null == iter.next();
```
@deprecated M0235

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray<T>(array : [var T]) : Iter<T>
```

Like `fromArray` but for Arrays with mutable elements. Captures
the elements of the Array at the time the iterator is created, so
further modifications won't be reflected in the iterator.
@deprecated M0235

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : Iter<T>) : [T]
```

Consumes an iterator and collects its produced elements in an Array.
```motoko include=import
let iter = [1, 2, 3].values();
assert [1, 2, 3] == Iter.toArray(iter);
```

## Function `toVarArray`
``` motoko no-repl
func toVarArray<T>(self : Iter<T>) : [var T]
```

Like `toArray` but for Arrays with mutable elements.

## Function `sort`
``` motoko no-repl
func sort<T>(self : Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : Iter<T>
```

Sorted iterator.  Will iterate over *all* elements to sort them, necessarily.

## Function `repeat`
``` motoko no-repl
func repeat<T>(item : T, count : Nat) : Iter<T>
```

Creates an iterator that produces a given item a specified number of times.
```motoko include=import
let iter = Iter.repeat<Nat>(3, 2);
assert ?3 == iter.next();
assert ?3 == iter.next();
assert null == iter.next();
```

Runtime: O(1)

Space: O(1)

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : Iter<T>) : Iter<T>
```

Creates a new iterator that produces elements from the original iterator in reverse order.
Note: This function needs to consume the entire iterator to reverse it.
```motoko include=import
let iter = Iter.fromArray([1, 2, 3]);
let reversed = Iter.reverse(iter);
assert ?3 == reversed.next();
assert ?2 == reversed.next();
assert ?1 == reversed.next();
assert null == reversed.next();
```

Runtime: O(n) where n is the number of elements in the iterator

Space: O(n) where n is the number of elements in the iterator
