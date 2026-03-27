# core/pure/RealTimeQueue
Double-ended immutable queue with guaranteed `O(1)` push/pop operations (caveat: high constant factor).
For a default immutable queue implementation, see `pure/Queue`.

This module provides an alternative implementation with better worst-case performance for single operations, e.g. `pushBack` and `popFront`.
These operations are always constant time, `O(1)`, which eliminates spikes in performance of `pure/Queue` operations
that are caused by the amortized nature of the `pure/Queue` implementation, which can lead to `O(n)` worst-case performance for a single operation.
The spikes in performance can cause a single message to take multiple more rounds to complete than most other messages.

However, the `O(1)` operations come at a cost of higher constant factor than the `pure/Queue` implementation:
- 'pop' operations are on average 3x more expensive
- 'push' operations are on average 8x more expensive

For better performance across multiple operations and when the spikes in single operations are not a problem, use `pure/Queue`.
For guaranteed `O(1)` operations, use `pure/RealTimeQueue`.

---

The interface is purely functional, not imperative, and queues are immutable values.
In particular, Queue operations such as push and pop do not update their input queue but, instead, return the
value of the modified Queue, alongside any other data.
The input queue is left unchanged.

Examples of use-cases:
- Queue (FIFO) by using `pushBack()` and `popFront()`.
- Stack (LIFO) by using `pushFront()` and `popFront()`.
- Deque (double-ended queue) by using any combination of push/pop operations on either end.

A Queue is internally implemented as a real-time double-ended queue based on the paper
"Real-Time Double-Ended Queue Verified (Proof Pearl)". The implementation maintains
worst-case constant time `O(1)` for push/pop operations through gradual rebalancing steps.

Construction: Create a new queue with the `empty<T>()` function.

Note that some operations that traverse the elements of the queue (e.g. `forEach`, `values`) preserve the order of the elements,
whereas others (e.g. `map`, `contains`) do NOT guarantee that the elements are visited in any order.
The order is undefined to avoid allocations, making these operations more efficient.

```motoko name=import
import Queue "mo:core/pure/RealTimeQueue";
```

## Type `Queue`
``` motoko no-repl
type Queue<T> = {#empty; #one : T; #two : (T, T); #three : (T, T, T); #idles : (Idle<T>, Idle<T>); #rebal : States<T>}
```

The real-time queue data structure can be in one of the following states:

- `#empty`: the queue is empty
- `#one`: the queue contains a single element
- `#two`: the queue contains two elements
- `#three`: the queue contains three elements
- `#idles`: the queue is in the idle state, where `l` and `r` are non-empty stacks of elements fulfilling the size invariant
- `#rebal`: the queue is in the rebalancing state

## Function `empty`
``` motoko no-repl
func empty<T>() : Queue<T>
```

Create a new empty queue.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.empty<Nat>();
  assert Queue.isEmpty(queue);
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : Queue<T>) : Bool
```

Determine whether a queue is empty.
Returns true if `queue` is empty, otherwise `false`.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.empty<Nat>();
  assert Queue.isEmpty(queue);
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : Queue<T>
```

Create a new queue comprising a single element.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.singleton<Nat>(25);
  assert Queue.size(queue) == 1;
  assert Queue.peekFront(queue) == ?25;
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `size`
``` motoko no-repl
func size<T>(self : Queue<T>) : Nat
```

Determine the number of elements contained in a queue.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.singleton<Nat>(42);
  assert Queue.size(queue) == 1;
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `contains`
``` motoko no-repl
func contains<T>(self : Queue<T>, equal : (implicit : (T, T) -> Bool), item : T) : Bool
```

Test if a queue contains a given value.
Returns true if the queue contains the item, otherwise false.

Note: The order in which elements are visited is undefined, for performance reasons.

Example:
```motoko include=import
import Nat "mo:core/Nat";

persistent actor {
  let queue = Queue.pushBack(Queue.pushBack(Queue.empty<Nat>(), 1), 2);
  assert Queue.contains(queue, Nat.equal, 1);
  assert not Queue.contains(queue, Nat.equal, 3);
}
```

Runtime: `O(size)`

Space: `O(1)`

## Function `peekFront`
``` motoko no-repl
func peekFront<T>(self : Queue<T>) : ?T
```

Inspect the optional element on the front end of a queue.
Returns `null` if `queue` is empty. Otherwise, the front element of `queue`.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.pushFront(Queue.pushFront(Queue.empty<Nat>(), 2), 1);
  assert Queue.peekFront(queue) == ?1;
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `peekBack`
``` motoko no-repl
func peekBack<T>(self : Queue<T>) : ?T
```

Inspect the optional element on the back end of a queue.
Returns `null` if `queue` is empty. Otherwise, the back element of `queue`.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.pushFront(Queue.pushFront(Queue.empty<Nat>(), 2), 1);
  assert Queue.peekBack(queue) == ?2;
}
```

Runtime: `O(1)`.

Space: `O(1)`.

## Function `pushFront`
``` motoko no-repl
func pushFront<T>(self : Queue<T>, element : T) : Queue<T>
```

Insert a new element on the front end of a queue.
Returns the new queue with `element` in the front followed by the elements of `queue`.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.pushFront(Queue.pushFront(Queue.empty<Nat>(), 2), 1);
  assert Queue.peekFront(queue) == ?1;
  assert Queue.peekBack(queue) == ?2;
  assert Queue.size(queue) == 2;
}
```

Runtime: `O(1)` worst-case!

Space: `O(1)` worst-case!

## Function `pushBack`
``` motoko no-repl
func pushBack<T>(self : Queue<T>, element : T) : Queue<T>
```

Insert a new element on the back end of a queue.
Returns the new queue with all the elements of `queue`, followed by `element` on the back.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.pushBack(Queue.pushBack(Queue.empty<Nat>(), 1), 2);
  assert Queue.peekBack(queue) == ?2;
  assert Queue.size(queue) == 2;
}
```

Runtime: `O(1)` worst-case!

Space: `O(1)` worst-case!

## Function `popFront`
``` motoko no-repl
func popFront<T>(self : Queue<T>) : ?(T, Queue<T>)
```

Remove the element on the front end of a queue.
Returns `null` if `queue` is empty. Otherwise, it returns a pair of
the first element and a new queue that contains all the remaining elements of `queue`.

Example:
```motoko include=import
import Runtime "mo:core/Runtime";

persistent actor {
  do {
    let initial = Queue.pushBack(Queue.pushBack(Queue.empty<Nat>(), 1), 2);
    let ?(frontElement, remainingQueue) = Queue.popFront(initial) else Runtime.trap "Empty queue impossible";
    assert frontElement == 1;
    assert Queue.size(remainingQueue) == 1;
  }
}
```

Runtime: `O(1)` worst-case!

Space: `O(1)` worst-case!

## Function `popBack`
``` motoko no-repl
func popBack<T>(self : Queue<T>) : ?(Queue<T>, T)
```

Remove the element on the back end of a queue.
Returns `null` if `queue` is empty. Otherwise, it returns a pair of
a new queue that contains the remaining elements of `queue`
and, as the second pair item, the removed back element.

Example:
```motoko include=import
import Runtime "mo:core/Runtime";

persistent actor {
  do {
    let initial = Queue.pushBack(Queue.pushBack(Queue.empty<Nat>(), 1), 2);
    let ?(reducedQueue, removedElement) = Queue.popBack(initial) else Runtime.trap "Empty queue impossible";
    assert removedElement == 2;
    assert Queue.size(reducedQueue) == 1;
  }
}
```

Runtime: `O(1)` worst-case!

Space: `O(1)` worst-case!

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Iter<T>) : Queue<T>
```

Turn an iterator into a queue, consuming it.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([0, 1, 2, 3, 4].values());
  assert Queue.peekFront(queue) == ?0;
  assert Queue.peekBack(queue) == ?4;
  assert Queue.size(queue) == 5;
}
```

Runtime: `O(size)`

Space: `O(size)`

## Function `toQueue`
``` motoko no-repl
func toQueue<T>(self : Iter<T>) : Queue<T>
```

Convert an iterator into a queue, consuming the iterator.

Example:
```motoko include=import
persistent actor {
  transient let iter = [0, 1, 2, 3, 4].values();

  let queue = iter.toQueue();

  assert Queue.peekFront(queue) == ?0;
  assert Queue.peekBack(queue) == ?4;
  assert Queue.size(queue) == 5;
}
```

Runtime: `O(size)`

Space: `O(size)`

## Function `values`
``` motoko no-repl
func values<T>(self : Queue<T>) : Iter.Iter<T>
```

Create an iterator over the elements in the queue. The order of the elements is from front to back.

Example:
```motoko include=import
import Iter "mo:core/Iter";

persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  assert Iter.toArray(Queue.values(queue)) == [1, 2, 3];
}
```

Runtime: `O(1)` to create the iterator and for each `next()` call.

Space: `O(1)` to create the iterator and for each `next()` call.

## Function `equal`
``` motoko no-repl
func equal<T>(self : Queue<T>, other : Queue<T>, equal : (implicit : (T, T) -> Bool)) : Bool
```

Compare two queues for equality using a provided equality function to compare their elements.
Two queues are considered equal if they contain the same elements in the same order.

Example:
```motoko include=import
import Nat "mo:core/Nat";

persistent actor {
  let queue1 = Queue.fromIter<Nat>([1, 2, 3].values());
  let queue2 = Queue.fromIter<Nat>([1, 2, 3].values());
  let queue3 = Queue.fromIter<Nat>([1, 3, 2].values());
  assert Queue.equal(queue1, queue2, Nat.equal);
  assert not Queue.equal(queue1, queue3, Nat.equal);
}
```

Runtime: `O(size)`

Space: `O(size)`

## Function `compare`
``` motoko no-repl
func compare<T>(self : Queue<T>, other : Queue<T>, compareItem : (implicit : (compare : (T, T) -> Types.Order))) : Types.Order
```

Compare two queues lexicographically using a provided comparison function to compare their elements.
Returns `#less` if `queue1` is lexicographically less than `queue2`, `#equal` if they are equal, and `#greater` otherwise.

Example:
```motoko include=import
import Nat "mo:core/Nat";

persistent actor {
  let queue1 = Queue.fromIter<Nat>([1, 2, 3].values());
  let queue2 = Queue.fromIter<Nat>([1, 2, 4].values());
  assert Queue.compare(queue1, queue2, Nat.compare) == #less;
}
```

Runtime: `O(size)`

Space: `O(size)`

## Function `all`
``` motoko no-repl
func all<T>(self : Queue<T>, predicate : T -> Bool) : Bool
```

Return true if the given predicate is true for all queue elements.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([2, 4, 6].values());
  assert Queue.all<Nat>(queue, func n = n % 2 == 0);
  assert not Queue.all<Nat>(queue, func n = n > 4);
}
```

Runtime: `O(size)`

Space: `O(size)` as the current implementation uses `values` to iterate over the queue.

*Runtime and space assumes that the `predicate` runs in `O(1)` time and space.

## Function `any`
``` motoko no-repl
func any<T>(self : Queue<T>, predicate : T -> Bool) : Bool
```

Return true if the given predicate is true for any queue element.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  assert Queue.any<Nat>(queue, func n = n > 2);
  assert not Queue.any<Nat>(queue, func n = n > 3);
}
```

Runtime: `O(size)`

Space: `O(size)` as the current implementation uses `values` to iterate over the queue.

*Runtime and space assumes that the `predicate` runs in `O(1)` time and space.

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : Queue<T>, f : T -> ())
```

Call the given function for its side effect on each queue element in order: from front to back.

Example:
```motoko include=import
import Nat "mo:core/Nat";
persistent actor {
  var text = "";
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  Queue.forEach<Nat>(queue, func n = text #= Nat.toText(n));
  assert text == "123";
}
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in `O(1)` time and space.

## Function `map`
``` motoko no-repl
func map<T1, T2>(self : Queue<T1>, f : T1 -> T2) : Queue<T2>
```

Create a new queue by applying the given function to each element of the original queue.

Note: The order of visiting elements is undefined with the current implementation.

Example:
```motoko include=import
import Nat "mo:core/Nat";

persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  let mapped = Queue.map<Nat, Nat>(queue, func n = n * 2);
  assert Queue.size(mapped) == 3;
  assert Queue.peekFront(mapped) == ?2;
  assert Queue.peekBack(mapped) == ?6;
}
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `f` runs in `O(1)` time and space.

## Function `filter`
``` motoko no-repl
func filter<T>(self : Queue<T>, predicate : T -> Bool) : Queue<T>
```

Create a new queue with only those elements of the original queue for which
the given predicate returns true.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3, 4].values());
  let filtered = Queue.filter<Nat>(queue, func n = n % 2 == 0);
  assert Queue.size(filtered) == 2;
  assert Queue.peekFront(filtered) == ?2;
  assert Queue.peekBack(filtered) == ?4;
}
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that `predicate` runs in `O(1)` time and space.

## Function `filterMap`
``` motoko no-repl
func filterMap<T, U>(self : Queue<T>, f : T -> ?U) : Queue<U>
```

Create a new queue by applying the given function to each element of the original queue
and collecting the results for which the function returns a non-null value.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3, 4].values());
  let filtered = Queue.filterMap<Nat, Nat>(queue, func n = if (n % 2 == 0) { ?n } else null);
  assert Queue.size(filtered) == 2;
  assert Queue.peekFront(filtered) == ?2;
  assert Queue.peekBack(filtered) == ?4;
}
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that f runs in `O(1)` time and space.

## Function `toText`
``` motoko no-repl
func toText<T>(self : Queue<T>, f : (implicit : (toText : T -> Text))) : Text
```

Create a `Text` representation of a queue for debugging purposes.

Example:
```motoko include=import
import Nat "mo:core/Nat";

persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  assert Queue.toText(queue, Nat.toText) == "RealTimeQueue[1, 2, 3]";
}
```

Runtime: `O(size)`

Space: `O(size)`

*Runtime and space assumes that f runs in `O(1)` time and space.

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : Queue<T>) : Queue<T>
```

Reverse the order of elements in a queue.
This operation is cheap, it does NOT require copying the elements.

Example:
```motoko include=import
persistent actor {
  let queue = Queue.fromIter<Nat>([1, 2, 3].values());
  let reversed = Queue.reverse(queue);
  assert Queue.peekFront(reversed) == ?3;
  assert Queue.peekBack(reversed) == ?1;
}
```

Runtime: `O(1)`

Space: `O(1)`
