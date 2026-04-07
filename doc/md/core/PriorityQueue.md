# core/PriorityQueue
A mutable priority queue of elements.
Always returns the element with the highest priority first,
as determined by a user-provided comparison function.

Typical use cases include:
* Task scheduling (highest-priority task first)
* Event simulation
* Pathfinding algorithms (e.g. Dijkstra, A*)

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

persistent actor {
  let pq = PriorityQueue.empty<Nat>();
  PriorityQueue.push(pq, Nat.compare, 5);
  PriorityQueue.push(pq, Nat.compare, 10);
  PriorityQueue.push(pq, Nat.compare, 3);
  assert PriorityQueue.pop(pq, Nat.compare) == ?10;
  assert PriorityQueue.pop(pq, Nat.compare) == ?5;
  assert PriorityQueue.pop(pq, Nat.compare) == ?3;
  assert PriorityQueue.pop(pq, Nat.compare) == null;
}
```

Internally implemented as a binary heap stored in a core library `List`.

Performance:
* Runtime: `O(log n)` for `push` and `pop` (amortized).
* Runtime: `O(1)` for `peek`, `clear`, `size`, and `isEmpty`.
* Space: `O(n)`, where `n` is the number of stored elements.

Implementation note (due to `List`):
* There is an additive memory overhead of `O(sqrt(n))`.
* For `push` and `pop`, the amortized time is `O(log n)`,
  but the worst case can involve an extra `O(sqrt(n))` step.

## Type `PriorityQueue`
``` motoko no-repl
type PriorityQueue<T> = Types.PriorityQueue<T>
```


## Function `empty`
``` motoko no-repl
func empty<T>() : PriorityQueue<T>
```

Returns an empty priority queue.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";

let pq = PriorityQueue.empty<Nat>();
assert PriorityQueue.isEmpty(pq);
```

Runtime: `O(1)`. Space: `O(1)`.

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : PriorityQueue<T>
```

Returns a priority queue containing a single element.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";

let pq = PriorityQueue.singleton<Nat>(42);
assert PriorityQueue.peek(pq) == ?42;
```

Runtime: `O(1)`. Space: `O(1)`.

## Function `size`
``` motoko no-repl
func size<T>(self : PriorityQueue<T>) : Nat
```

Returns the number of elements in the priority queue.

Runtime: `O(1)`.

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : PriorityQueue<T>) : Bool
```

Returns `true` iff the priority queue is empty.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

let pq = PriorityQueue.empty<Nat>();
assert PriorityQueue.isEmpty(pq);
PriorityQueue.push(pq, Nat.compare, 5);
assert not PriorityQueue.isEmpty(pq);
```

Runtime: `O(1)`. Space: `O(1)`.

## Function `clear`
``` motoko no-repl
func clear<T>(self : PriorityQueue<T>)
```

Removes all elements from the priority queue.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";


let pq = PriorityQueue.empty<Nat>();
PriorityQueue.push(pq, Nat.compare, 5);
PriorityQueue.push(pq, Nat.compare, 10);
assert not PriorityQueue.isEmpty(pq);
PriorityQueue.clear(pq);
assert PriorityQueue.isEmpty(pq);
```

Runtime: `O(1)`. Space: `O(1)`.

## Function `push`
``` motoko no-repl
func push<T>(self : PriorityQueue<T>, compare : (implicit : (T, T) -> Order.Order), element : T)
```

Inserts a new element into the priority queue.

`compare` – comparison function that defines priority ordering.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

let pq = PriorityQueue.empty<Nat>();
PriorityQueue.push(pq, Nat.compare, 5);
PriorityQueue.push(pq, Nat.compare, 10);
assert PriorityQueue.peek(pq) == ?10;
```

Runtime: `O(log n)`. Space: `O(1)`.

## Function `peek`
``` motoko no-repl
func peek<T>(self : PriorityQueue<T>) : ?T
```

Returns the element with the highest priority, without removing it.
Returns `null` if the queue is empty.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";

let pq = PriorityQueue.singleton<Nat>(42);
assert PriorityQueue.peek(pq) == ?42;
```

Runtime: `O(1)`. Space: `O(1)`.

## Function `pop`
``` motoko no-repl
func pop<T>(self : PriorityQueue<T>, compare : (implicit : (T, T) -> Order.Order)) : ?T
```

Removes and returns the element with the highest priority.
Returns `null` if the queue is empty.

`compare` – comparison function that defines priority ordering.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

let pq = PriorityQueue.empty<Nat>();
PriorityQueue.push(pq, Nat.compare, 5);
PriorityQueue.push(pq, Nat.compare, 10);
assert PriorityQueue.pop(pq, Nat.compare) == ?10;
```

Runtime: `O(log n)`. Space: `O(1)`.

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Types.Iter<T>, compare : (implicit : (T, T) -> Order.Order)) : PriorityQueue<T>
```

Creates a new priority queue from an iterator.

`compare` – comparison function that defines priority ordering.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

let pq = PriorityQueue.fromIter<Nat>([5, 10, 3].values(), Nat.compare);
assert PriorityQueue.size(pq) == 3;
assert PriorityQueue.peek(pq) == ?10;
```

Runtime: `O(n * log(n))`.
Space: `O(n)`.
`n` denotes the number of elements in the iterator.

## Function `clone`
``` motoko no-repl
func clone<T>(self : PriorityQueue<T>) : PriorityQueue<T>
```

Creates a copy of the priority queue.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";

let original = PriorityQueue.fromIter<Nat>([5, 10, 3].values(), Nat.compare);
let copy = PriorityQueue.clone(original);
assert PriorityQueue.pop(copy, Nat.compare) == ?10;
assert PriorityQueue.size(original) == 3;
```

Runtime: `O(n)`. Space: `O(n)`.
`n` denotes the number of elements in the priority queue.

## Function `values`
``` motoko no-repl
func values<T>(self : PriorityQueue<T>, compare : (implicit : (T, T) -> Order.Order)) : Types.Iter<T>
```

Returns an iterator that yields elements in descending priority order
(highest priority first, matching `pop` semantics).

The original queue is not modified. Internally clones the heap
and pops from the clone on each `next()` call.

`compare` – comparison function that defines priority ordering.

Example:
```motoko
import PriorityQueue "mo:core/PriorityQueue";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

let pq = PriorityQueue.fromIter<Nat>([5, 10, 3].values(), Nat.compare);
assert Iter.toArray(PriorityQueue.values(pq, Nat.compare)) == [10, 5, 3];
```

Runtime: `O(n)` to create the iterator, `O(log n)` per `next()` call.
Space: `O(n)` for the internal clone.
`n` denotes the number of elements in the priority queue.
