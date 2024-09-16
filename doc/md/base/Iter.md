# Iter
Iterators

## Type `Iter`
``` motoko no-repl
type Iter<T> = { next : () -> ?T }
```

An iterator that produces values of type `T`. Calling `next` returns
`null` when iteration is finished.

Iterators are inherently stateful. Calling `next` "consumes" a value from
the Iterator that cannot be put back, so keep that in mind when sharing
iterators between consumers.

An iterater `i` can be iterated over using
```
for (x in i) {
  …do something with x…
}
```

## Class `range`

``` motoko no-repl
class range(x : Nat, y : Int)
```

Creates an iterator that produces all `Nat`s from `x` to `y` including
both of the bounds.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.range(1, 3);
assert(?1 == iter.next());
assert(?2 == iter.next());
assert(?3 == iter.next());
assert(null == iter.next());
```

### Function `next`
``` motoko no-repl
func next() : ?Nat
```


## Class `revRange`

``` motoko no-repl
class revRange(x : Int, y : Int)
```

Like `range` but produces the values in the opposite
order.

### Function `next`
``` motoko no-repl
func next() : ?Int
```


## Function `iterate`
``` motoko no-repl
func iterate<A>(xs : Iter<A>, f : (A, Nat) -> ())
```

Calls a function `f` on every value produced by an iterator and discards
the results. If you're looking to keep these results use `map` instead.

```motoko
import Iter "mo:base/Iter";
var sum = 0;
Iter.iterate<Nat>(Iter.range(1, 3), func(x, _index) {
  sum += x;
});
assert(6 == sum)
```

## Function `size`
``` motoko no-repl
func size<A>(xs : Iter<A>) : Nat
```

Consumes an iterator and counts how many elements were produced
(discarding them in the process).

## Function `map`
``` motoko no-repl
func map<A, B>(xs : Iter<A>, f : A -> B) : Iter<B>
```

Takes a function and an iterator and returns a new iterator that lazily applies
the function to every element produced by the argument iterator.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.range(1, 3);
let mappedIter = Iter.map(iter, func (x : Nat) : Nat { x * 2 });
assert(?2 == mappedIter.next());
assert(?4 == mappedIter.next());
assert(?6 == mappedIter.next());
assert(null == mappedIter.next());
```

## Function `filter`
``` motoko no-repl
func filter<A>(xs : Iter<A>, f : A -> Bool) : Iter<A>
```

Takes a function and an iterator and returns a new iterator that produces
elements from the original iterator if and only if the predicate is true.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.range(1, 3);
let mappedIter = Iter.filter(iter, func (x : Nat) : Bool { x % 2 == 1 });
assert(?1 == mappedIter.next());
assert(?3 == mappedIter.next());
assert(null == mappedIter.next());
```

## Function `make`
``` motoko no-repl
func make<A>(x : A) : Iter<A>
```

Creates an iterator that produces an infinite sequence of `x`.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.make(10);
assert(?10 == iter.next());
assert(?10 == iter.next());
assert(?10 == iter.next());
// ...
```

## Function `concat`
``` motoko no-repl
func concat<A>(a : Iter<A>, b : Iter<A>) : Iter<A>
```

Takes two iterators and returns a new iterator that produces
elements from the original iterators sequentally.
```motoko
import Iter "mo:base/Iter";
let iter1 = Iter.range(1, 2);
let iter2 = Iter.range(5, 6);
let concatenatedIter = Iter.concat(iter1, iter2);
assert(?1 == concatenatedIter.next());
assert(?2 == concatenatedIter.next());
assert(?5 == concatenatedIter.next());
assert(?6 == concatenatedIter.next());
assert(null == concatenatedIter.next());
```

## Function `fromArray`
``` motoko no-repl
func fromArray<A>(xs : [A]) : Iter<A>
```

Creates an iterator that produces the elements of an Array in ascending index order.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.fromArray([1, 2, 3]);
assert(?1 == iter.next());
assert(?2 == iter.next());
assert(?3 == iter.next());
assert(null == iter.next());
```

## Function `fromArrayMut`
``` motoko no-repl
func fromArrayMut<A>(xs : [var A]) : Iter<A>
```

Like `fromArray` but for Arrays with mutable elements. Captures
the elements of the Array at the time the iterator is created, so
further modifications won't be reflected in the iterator.

## Value `fromList`
``` motoko no-repl
let fromList
```

Like `fromArray` but for Lists.

## Function `toArray`
``` motoko no-repl
func toArray<A>(xs : Iter<A>) : [A]
```

Consumes an iterator and collects its produced elements in an Array.
```motoko
import Iter "mo:base/Iter";
let iter = Iter.range(1, 3);
assert([1, 2, 3] == Iter.toArray(iter));
```

## Function `toArrayMut`
``` motoko no-repl
func toArrayMut<A>(xs : Iter<A>) : [var A]
```

Like `toArray` but for Arrays with mutable elements.

## Function `toList`
``` motoko no-repl
func toList<A>(xs : Iter<A>) : List.List<A>
```

Like `toArray` but for Lists.

## Function `sort`
``` motoko no-repl
func sort<A>(xs : Iter<A>, compare : (A, A) -> Order.Order) : Iter<A>
```

Sorted iterator.  Will iterate over *all* elements to sort them, necessarily.
