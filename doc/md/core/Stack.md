# core/Stack
A mutable stack data structure.
Elements can be pushed on top of the stack
and removed from top of the stack (LIFO).

Example:
```motoko
import Stack "mo:core/Stack";
import Debug "mo:core/Debug";

persistent actor {
  let levels = Stack.empty<Text>();
  Stack.push(levels, "Inner");
  Stack.push(levels, "Middle");
  Stack.push(levels, "Outer");
  assert Stack.pop(levels) == ?"Outer";
  assert Stack.pop(levels) == ?"Middle";
  assert Stack.pop(levels) == ?"Inner";
  assert Stack.pop(levels) == null;
}
```

The internal implementation is a singly-linked list.

Performance:
* Runtime: `O(1)` for push, pop, and peek operation.
* Space: `O(n)`.
`n` denotes the number of elements stored on the stack.

## Type `Stack`
``` motoko no-repl
type Stack<T> = Types.Stack<T>
```


## Function `toPure`
``` motoko no-repl
func toPure<T>(self : Stack<T>) : PureList.List<T>
```

Convert a mutable stack to an immutable, purely functional list.
Please note that functional lists are ordered like stacks (FIFO).

Example:
```motoko
import Stack "mo:core/Stack";
import PureList "mo:core/pure/List";
import Iter "mo:core/Iter";

persistent actor {
  let mutableStack = Stack.empty<Nat>();
  Stack.push(mutableStack, 3);
  Stack.push(mutableStack, 2);
  Stack.push(mutableStack, 1);
  let immutableList = Stack.toPure(mutableStack);
  assert Iter.toArray(PureList.values(immutableList)) == [1, 2, 3];
}
```

Runtime: `O(1)`.
Space: `O(1)`.
where `n` denotes the number of elements stored in the stack.
@deprecated M0235

## Function `toArray`
``` motoko no-repl
func toArray<T>(self : Stack<T>) : [T]
```


## Function `toVarArray`
``` motoko no-repl
func toVarArray<T>(self : Stack<T>) : [var T]
```


## Function `fromPure`
``` motoko no-repl
func fromPure<T>(list : PureList.List<T>) : Stack<T>
```

Convert an immutable, purely functional list to a mutable stack.
Please note that functional lists are ordered like stacks (FIFO).

Example:
```motoko
import Stack "mo:core/Stack";
import PureList "mo:core/pure/List";
import Iter "mo:core/Iter";

persistent actor {
  let immutableList = PureList.fromIter<Nat>([1, 2, 3].values());
  let mutableStack = Stack.fromPure<Nat>(immutableList);
  assert Iter.toArray(Stack.values(mutableStack)) == [1, 2, 3];
}
```

Runtime: `O(n)`.
Space: `O(n)`.
where `n` denotes the number of elements stored in the queue.
@deprecated M0235

## Function `fromVarArray`
``` motoko no-repl
func fromVarArray<T>(array : [var T]) : Stack<T>
```


## Function `fromArray`
``` motoko no-repl
func fromArray<T>(array : [T]) : Stack<T>
```


## Function `empty`
``` motoko no-repl
func empty<T>() : Stack<T>
```

Create a new empty mutable stack.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let stack = Stack.empty<Text>();
  assert Stack.size(stack) == 0;
}
```

Runtime: `O(1)`.
Space: `O(1)`.

## Function `tabulate`
``` motoko no-repl
func tabulate<T>(size : Nat, generator : Nat -> T) : Stack<T>
```

Creates a new stack with `size` elements by applying the `generator` function to indices `[0..size-1]`.
Elements are pushed in ascending index order.
Which means that the generated element with the index `0` will be at the bottom of the stack.

Example:
```motoko
import Stack "mo:core/Stack";
import Iter "mo:core/Iter";

persistent actor {
  let stack = Stack.tabulate<Nat>(3, func(i) { 2 * i });
  assert Iter.toArray(Stack.values(stack)) == [4, 2, 0];
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack and
assuming that `generator` has O(1) costs.

## Function `singleton`
``` motoko no-repl
func singleton<T>(element : T) : Stack<T>
```

Creates a new stack containing a single element.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.singleton<Text>("motoko");
  assert Stack.peek(stack) == ?"motoko";
}
```

Runtime: O(1)
Space: O(1)

## Function `clear`
``` motoko no-repl
func clear<T>(self : Stack<T>)
```

Removes all elements from the stack.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  Stack.clear(stack);
  assert Stack.isEmpty(stack);
}
```

Runtime: O(1)
Space: O(1)

## Function `clone`
``` motoko no-repl
func clone<T>(self : Stack<T>) : Stack<T>
```

Creates a deep copy of the stack with the same elements in the same order.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let original = Stack.fromIter<Nat>([3, 2, 1].values());
  let copy = Stack.clone(original);
  assert Stack.equal(copy, original, Nat.equal);
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack.

## Function `isEmpty`
``` motoko no-repl
func isEmpty<T>(self : Stack<T>) : Bool
```

Returns true if the stack contains no elements.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  assert Stack.isEmpty(stack);
}
```

Runtime: O(1)
Space: O(1)

## Function `size`
``` motoko no-repl
func size<T>(self : Stack<T>) : Nat
```

Returns the number of elements on the stack.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.size(stack) == 3;
}
```

Runtime: O(1)
Space: O(1)

## Function `contains`
``` motoko no-repl
func contains<T>(self : Stack<T>, equal : (implicit : (T, T) -> Bool), element : T) : Bool
```

Returns true if the stack contains the specified element.
Uses the provided equality function to compare elements.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.contains(stack, Nat.equal, 2);
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and assuming
that `equal` has O(1) costs.

## Function `reverseValues`
``` motoko no-repl
func reverseValues<T>(self : Stack<T>) : Iter.Iter<T>
```


## Function `push`
``` motoko no-repl
func push<T>(self : Stack<T>, value : T)
```

Pushes a new element onto the top of the stack.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 42);
  assert Stack.peek(stack) == ?42;
}
```

Runtime: O(1)
Space: O(1)

## Function `peek`
``` motoko no-repl
func peek<T>(self : Stack<T>) : ?T
```

Returns the top element of the stack without removing it.
Returns null if the stack is empty.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  assert Stack.peek(stack) == ?1;
}
```

Runtime: O(1)
Space: O(1)

## Function `pop`
``` motoko no-repl
func pop<T>(self : Stack<T>) : ?T
```

Removes and returns the top element of the stack.
Returns null if the stack is empty.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  assert Stack.pop(stack) == ?1;
  assert Stack.pop(stack) == ?2;
  assert Stack.pop(stack) == ?3;
  assert Stack.pop(stack) == null;
}
```

Runtime: O(1)
Space: O(1)

## Function `get`
``` motoko no-repl
func get<T>(self : Stack<T>, position : Nat) : ?T
```

Returns the element at the specified position from the top of the stack.
Returns null if position is out of bounds.
Position 0 is the top of the stack.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Char>();
  Stack.push(stack, 'c');
  Stack.push(stack, 'b');
  Stack.push(stack, 'a');
  assert Stack.get(stack, 0) == ?'a';
  assert Stack.get(stack, 1) == ?'b';
  assert Stack.get(stack, 2) == ?'c';
  assert Stack.get(stack, 3) == null;
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack.

## Function `reverse`
``` motoko no-repl
func reverse<T>(self : Stack<T>)
```

Reverses the order of elements in the stack.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  Stack.reverse(stack);
  assert Stack.pop(stack) == ?3;
  assert Stack.pop(stack) == ?2;
  assert Stack.pop(stack) == ?1;
  assert Stack.pop(stack) == null;
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack.

## Function `values`
``` motoko no-repl
func values<T>(self : Stack<T>) : Types.Iter<T>
```

Returns an iterator over the elements in the stack, from top to bottom.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  assert Iter.toArray(Stack.values(stack)) == [1, 2, 3];
}
```

Runtime: O(1) for iterator creation, O(n) for full traversal
Space: O(1)
where `n` denotes the number of elements stored on the stack.

## Function `all`
``` motoko no-repl
func all<T>(self : Stack<T>, predicate : T -> Bool) : Bool
```

Returns true if all elements in the stack satisfy the predicate.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromIter<Nat>([2, 4, 6].values());
  assert Stack.all<Nat>(stack, func(n) = n % 2 == 0);
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and
assuming that `predicate` has O(1) costs.

## Function `any`
``` motoko no-repl
func any<T>(self : Stack<T>, predicate : T -> Bool) : Bool
```

Returns true if any element in the stack satisfies the predicate.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.any<Nat>(stack, func(n) = n == 2);
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and
assuming `predicate` has O(1) costs.

## Function `forEach`
``` motoko no-repl
func forEach<T>(self : Stack<T>, operation : T -> ())
```

Applies the operation to each element in the stack, from top to bottom.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  var text = "";
  Stack.forEach<Nat>(stack, func(n) = text #= Nat.toText(n));
  assert text == "123";
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and
assuming that `operation` has O(1) costs.

## Function `map`
``` motoko no-repl
func map<T, U>(self : Stack<T>, project : T -> U) : Stack<U>
```

Creates a new stack by applying the projection function to each element.
Maintains the original order of elements.

Example:
```motoko
import Stack "mo:core/Stack";
import Iter "mo:core/Iter";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  let doubled = Stack.map<Nat, Nat>(stack, func(n) { 2 * n });
  assert Stack.get(doubled, 0) == ?2;
  assert Stack.get(doubled, 1) == ?4;
  assert Stack.get(doubled, 2) == ?6;
  assert Stack.get(doubled, 3) == null;
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack and
assuming that `project` has O(1) costs.

## Function `filter`
``` motoko no-repl
func filter<T>(self : Stack<T>, predicate : T -> Bool) : Stack<T>
```

Creates a new stack containing only elements that satisfy the predicate.
Maintains the relative order of elements.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 4);
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  let evens = Stack.filter<Nat>(stack, func(n) { n % 2 == 0 });
  assert Stack.pop(evens) == ?2;
  assert Stack.pop(evens) == ?4;
  assert Stack.pop(evens) == null;
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack and
assuming `predicate` has O(1) costs.

## Function `filterMap`
``` motoko no-repl
func filterMap<T, U>(self : Stack<T>, project : T -> ?U) : Stack<U>
```

Creates a new stack by applying the projection function to each element
and keeping only the successful results (where project returns ?value).
Maintains the relative order of elements.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.empty<Nat>();
  Stack.push(stack, 4);
  Stack.push(stack, 3);
  Stack.push(stack, 2);
  Stack.push(stack, 1);
  let evenDoubled = Stack.filterMap<Nat, Nat>(stack, func(n) {
    if (n % 2 == 0) {
      ?(n * 2)
    } else {
      null
    }
  });
  assert Stack.pop(evenDoubled) == ?4;
  assert Stack.pop(evenDoubled) == ?8;
  assert Stack.pop(evenDoubled) == null;
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack and
assuming that `project` has O(1) costs.

## Function `find`
``` motoko no-repl
func find<T>(self : Stack<T>, predicate : T -> Bool) : ?T
```

Return the first element for which the given `predicate` is true,
if such an element exists.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromPure<Nat>(?(1, ?(2, ?(3, null))));
  assert Stack.find<Nat>(stack, func n = n > 1) == ?2;
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `findIndex`
``` motoko no-repl
func findIndex<T>(self : Stack<T>, predicate : T -> Bool) : ?Nat
```

Return the first index for which the given `predicate` is true.
If no element satisfies the predicate, returns null.

Example:
```motoko
import Stack "mo:core/Stack";

persistent actor {
  let stack = Stack.fromPure(?('A', ?('B', ?('C', ?('D', null)))));
  let found = Stack.findIndex<Char>(stack, func x = x == 'C');
  assert found == ?2;
}
```

Runtime: O(size)

Space: O(1)

*Runtime and space assumes that `predicate` runs in O(1) time and space.

## Function `equal`
``` motoko no-repl
func equal<T>(self : Stack<T>, other : Stack<T>, equal : (implicit : (T, T) -> Bool)) : Bool
```

Compares two stacks for equality using the provided equality function.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let stack1 = Stack.fromIter<Nat>([3, 2, 1].values());
  let stack2 = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.equal(stack1, stack2, Nat.equal);
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and
assuming that `equal` has O(1) costs.

## Function `fromIter`
``` motoko no-repl
func fromIter<T>(iter : Types.Iter<T>) : Stack<T>
```

Creates a new stack from an iterator.
Elements are pushed in iteration order. Which means that the last element
of the iterator will be the first element on top of the stack.

Example:
```motoko
import Stack "mo:core/Stack";
import Iter "mo:core/Iter";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Iter.toArray(Stack.values(stack)) == [1, 2, 3];
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of iterated elements.

## Function `toStack`
``` motoko no-repl
func toStack<T>(self : Types.Iter<T>) : Stack<T>
```

Convert an iterator into a stack.
Elements are pushed in iteration order. Which means that the last element
of the iterator will be the first element on top of the stack.

Example:
```motoko
import Stack "mo:core/Stack";
import Iter "mo:core/Iter";

persistent actor {
  transient let iter = [3, 2, 1].values();

  let stack = iter.toStack<Nat>();

  assert Iter.toArray(Stack.values(stack)) == [1, 2, 3];
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of iterated elements.

## Function `toText`
``` motoko no-repl
func toText<T>(self : Stack<T>, format : (implicit : (toText : T -> Text))) : Text
```

Converts the stack to its string representation using the provided
element formatting function.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let stack = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.toText(stack, Nat.toText) == "Stack[1, 2, 3]";
}
```

Runtime: O(n)
Space: O(n)
where `n` denotes the number of elements stored on the stack and
assuming that `format` has O(1) costs.

## Function `compare`
``` motoko no-repl
func compare<T>(self : Stack<T>, other : Stack<T>, compare : (implicit : (T, T) -> Order.Order)) : Order.Order
```

Compares two stacks lexicographically using the provided comparison function.

Example:
```motoko
import Stack "mo:core/Stack";
import Nat "mo:core/Nat";

persistent actor {
  let stack1 = Stack.fromIter<Nat>([2, 1].values());
  let stack2 = Stack.fromIter<Nat>([3, 2, 1].values());
  assert Stack.compare(stack1, stack2, Nat.compare) == #less;
}
```

Runtime: O(n)
Space: O(1)
where `n` denotes the number of elements stored on the stack and
assuming that `compare` has O(1) costs.
