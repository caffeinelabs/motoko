---
title: "Motoko `base` to `core` migration guide"
description: "Comprehensive guide for migrating from the Motoko base package to the new core package."
sidebar:
  order: 12
  hidden: true
---

* [GitHub repository](https://github.com/caffeinelabs/motoko-core)
* [Documentation](https://mops.one/core/docs/)

The `core` package is a new and improved standard library for Motoko, focusing on:
* AI-friendly design patterns.
* Familiarity coming from languages such as JavaScript, Python, Java, and Rust.
* Simplified usage of data structures in stable memory.
* Consistent naming conventions and parameter ordering.

This page provides a comprehensive guide for migrating from the `base` Motoko package to the new `core` package.

### Project configuration

Add the following to your `mops.toml` file to begin using the `core` package:

```toml
[dependencies]
core = "0.0.0" # Check the latest version: https://mops.one/core
```

If you are migrating an existing project, you can keep the `base` import and gradually transition to using the new API.

### Important considerations

:::caution[Version requirements]
The `core` package depends on new language features, so make sure to update to the latest dfx (0.28+) or Motoko compiler (0.15+) before migrating.
:::

When updating to the `core` package:

- All data structures can now be stored in stable memory without the need for pre-upgrade/post-upgrade hooks, provided those data structures are instantiated at stable type arguments.
- `range()` functions in the `core` library are now exclusive rather than inclusive! Keep this in mind when replacing `Iter.range()` with `Nat.range()`.
- Functions previously named `vals()` are renamed to `values()`. This also applies to fields. For example, `array.vals()` can be replaced with `array.values()`.
- Hash-based data structures are no longer included in the standard library. It is encouraged to use ordered maps and sets for improved security.
- In some cases, it won't be possible to fully migrate to `core` due to removal of some features in `base`. In these cases, you can continue using both packages side-by-side or search for [Mops packages](https://mops.one/) built by the community.

For details on function signatures, please refer to the official [documentation](https://mops.one/core/docs/).

Also, feel free to ask for help by posting on the [ICP developer forum](https://forum.dfinity.org/c/developers) or opening a GitHub issue on the [`caffeinelabs/motoko-core`](https://github.com/caffeinelabs/motoko-core/issues) repository.

## Module changes

### 1. New modules

The following modules are **new** in the `core` package:

- `List` - Mutable list
- `Map` - Mutable map
- `Queue` - Mutable double-ended queue
- `Set` - Mutable set
- `Runtime` - Runtime utilities and assertions
- `Tuples` - Tuple utilities
- `Types` - Common type definitions
- `VarArray` - Mutable array operations
- `pure/List` - Immutable list (originally `mo:base/List`)
- `pure/Map` - Immutable map (originally `mo:base/OrderedMap`)
- `pure/RealTimeQueue` - Queue implementation with performance tradeoffs
- `pure/Set` - Immutable set (originally `mo:base/OrderedSet`)

### 2. Renamed modules

| Base package                   | Core package       | Notes                                               |
| ------------------------------ | ------------------ | --------------------------------------------------- |
| `ExperimentalCycles`           | `Cycles`           | Stabilized module for cycle management              |
| `ExperimentalInternetComputer` | `InternetComputer` | Stabilized low-level ICP interface                  |
| `Deque`                        | `pure/Queue`       | Enhanced double-ended queue becomes immutable queue |
| `List`                         | `pure/List`        | Original immutable list moved to `pure/` namespace  |
| `OrderedMap`                   | `pure/Map`         | Ordered map moved to `pure/` namespace              |
| `OrderedSet`                   | `pure/Set`         | Ordered set moved to `pure/` namespace              |

:::note
The `pure/` namespace contains immutable (purely functional) data structures where operations return new values rather than modifying in place. The namespace makes it clear which data structures are mutable and which are immutable.
:::

### 3. Removed modules

The following modules have been **removed** in the core package:

- `AssocList` - Use `Map` or `pure/Map` instead
- `Buffer` - Use `List` or `VarArray` instead
- `ExperimentalStableMemory` - Deprecated
- `Hash` - Vulnerable to hash collision attacks
- `HashMap` - Use `Map` or `pure/Map`
- `Heap` - Use `Map` or `Set` instead
- `IterType` - Merged into `Types` module
- `None` - Use `switch x {}` in place of `None.impossible(x)`
- `Prelude` - Merged into `Debug` and `Runtime`
- `RBTree` - Use `Map` instead
- `Trie` - Use `Map` instead
- `TrieMap` - Use `Map` or `pure/Map` instead
- `TrieSet` - Use `Set` or `pure/Set` instead

:::note
Modules like `Random`, `Region`, `Time`, `Timer`, and `Stack` still exist in core but with modified APIs.
:::

## Data structure improvements

The `core` package brings significant changes to data structures, making a clear separation between mutable and immutable (purely functional) APIs. All data structures can now be stored directly in stable memory.

| Data Structure         | Description                                                                                                                                                 |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **List**               | Mutable list (originally [`mo:vector`](https://mops.one/vector))                                                                                            |
| **Map**                | Mutable map (originally [`mo:stableheapbtreemap`](https://mops.one/stableheapbtreemap))                                                                     |
| **Queue**              | Mutable queue
| **Set**                | Mutable set                                                                                                                                                 |
| **Array**              | Immutable array                                                                                                                                             |
| **VarArray**           | Mutable array                                                                                                                                               |
| **pure/List**          | Immutable list (originally `mo:base/List`)                                                                                                                  |
| **pure/Map**           | Immutable map (originally `mo:base/OrderedMap`)                                                                                                             |
| **pure/Set**           | Immutable set (originally `mo:base/OrderedSet`)                                                                                                             |
| **pure/Queue**         | Immutable queue  (orginally `mo:base/Deque`)                                                                                                                                              |
| **pure/RealTimeQueue** | Immutable queue with [constant-time operations](https://drops.dagstuhl.de/storage/00lipics/lipics-vol268-itp2023/LIPIcs.ITP.2023.29/LIPIcs.ITP.2023.29.pdf) |

## Interface changes by module

### [`Array`](https://mops.one/core/docs/Array)

#### Renamed functions
- `append()` → `concat()`
- `chain()` → `flatMap()`
- `freeze()` → `fromVarArray()`
- `init()` → `repeat()` with reversed argument order
- `make()` → `singleton()`
- `mapFilter()` → `filterMap()`
- `slice()` → `range()`
- `subArray()` → `sliceToArray()`
- `thaw()` → `toVarArray()`
- `vals()` → `values()`

#### New functions
- `all()` - Check if all elements satisfy predicate
- `any()` - Check if any element satisfies predicate
- `compare()` - Compare two arrays
- `empty()` - Create empty array
- `enumerate()` - Get indexed iterator
- `findIndex()` - Find index of first matching element
- `forEach()` - Apply function to each element
- `fromIter()` - Create array from iterator
- `isEmpty()` - Check if array is empty
- `join()` - Join arrays from iterator
- `toText()` - Convert array to text representation

#### Parameter order changes
- `indexOf(element, array, equal)` → `indexOf(array, equal, element)`
- `lastIndexOf(element, array, equal)` → `lastIndexOf(array, equal, element)`
- `nextIndexOf(element, array, fromInclusive, equal)` → `nextIndexOf(array, equal, element, fromInclusive)`
- `prevIndexOf(element, array, fromExclusive, equal)` → `prevIndexOf(array, equal, element, fromExclusive)`

#### Removed functions
- `take()` - Use `sliceToArray()` instead
- `sortInPlace()` - Use `VarArray.sortInPlace()` instead
- `tabulateVar()` - Use `VarArray.tabulate()` instead

### [`Blob`](https://mops.one/core/docs/Blo)

#### Modified functions
- `fromArrayMut()` → `fromVarArray()`
- `hash()` - Return type changed from `Nat32` to `Types.Hash`
- `toArrayMut()` → `toVarArray()`

#### New functions
- `empty()` - Create an empty blob (`"" : Blob`)
- `isEmpty()` - Check if blob is empty
- `size()` - Get number of bytes in a blob (equivalent to `blob.size()`)

### [`Bool`](https://mops.one/core/docs/Bool)

#### Renamed functions
- `logand()` → `logicalAnd()`
- `lognot()` → `logicalNot()`
- `logor()` → `logicalOr()`
- `logxor()` → `logicalXor()`

#### New functions
- `allValues()` - Iterator over all boolean values

### [`Char`](https://mops.one/core/docs/Char)

#### Renamed functions
- `isLowercase()` → `isLower()`
- `isUppercase()` → `isUpper()`

### [`Debug`](https://mops.one/core/docs/Deug)

#### Added functions
- `todo()` - Replaces `Prelude.nyi()`

#### Removed functions
- `trap()` - Moved to `Runtime.trap()`

### [`Float`](https://mops.one/core/docs/Float)

#### Modified functions
- `equal()` - Now requires epsilon parameter
- `notEqual()` - Now requires epsilon parameter

#### Removed functions
- `equalWithin()`, `notEqualWithin()` - Use `equal()` and `notEqual()` with epsilon

### [`Iter`](https://mops.one/core/docs/Iter)

`Iter.range()` has been removed in favor of type-specific range functions such as `Nat.range()`, `Int.range()`, `Nat32.range()`, etc. These functions have an **exclusive upper bound**, in contrast to the original inclusive upper bound of `Iter.range()`. 

```motoko no-repl
import Int "mo:base/Int";
import Debug "mo:base/Debug";

persistent actor {
  // Iterate through -3, -2, -1, 0, 1, 2 (exclusive upper bound)
  for (number in Int.range(-3, 3)) {
    Debug.print(debug_show number);
  };

  // Iterate through -3, -2, -1, 0, 1, 2, 3
  for (number in Int.rangeInclusive(-3, 3)) {
    Debug.print(debug_show number);
  };
}
```

`rangeInclusive()` is included for use cases with an inclusive upper bound. The original `Iter.range()` corresponds to `Nat.rangeInclusive()`.

Helper functions have been added, such as `allValues()`, for each finite type in the `base` package.

### [`Int`](https://mops.one/core/docs/Int)

#### New functions
- `fromNat()` - Convert Nat to Int
- `fromText()` - Parse Int from text
- `range()` - Create iterator over range
- `rangeBy()` - Create iterator with step
- `rangeByInclusive()` - Inclusive range with step
- `rangeInclusive()` - Inclusive range
- `toNat()` - Convert Int to Nat (safe conversion)

#### Modified functions
- `fromText()` - Now returns `null` instead of `?0` for the inputs "+" and "-"

#### Removed functions
- `hash()`
- `hashAcc()` 

### [`Nat`](https://mops.one/core/docs/Nat)

#### New functions
- `allValues()` - Iterator over all natural numbers
- `bitshiftLeft()` / `bitshiftRight()` - Bit shifting operations
- `fromInt()` - Safe conversion from Int
- `fromText()` - Parse Nat from text
- `range()`, `rangeInclusive()` - Range iterators
- `rangeBy()`, `rangeByInclusive()` - Range with step
- `toInt()` - Convert to Int

### [`Int8`, `Int16`, `Int32`, `Int64`, `Nat8`, `Nat16`, `Nat32`, `Nat64`](https://mops.one/core/docs/)

#### Renamed fields

- `maximumValue` → `maxValue`
- `minimumValue` → `minValue`

#### New functions
- `allValues()` - Iterator over all values in range
- `range()`, `rangeInclusive()` - Range iterators (replaces `Iter.range()`)
- `explode()` - Slice into constituent bytes (only for sizes `16`, `32`, `64`)

### [`Option`](https://mops.one/core/docs/Option)

#### Renamed functions
- `make()` → `some()` - Create option from value
- `iterate()` → `forEach()` - Apply function to option value

#### New functions
- `compare()` - Compare two options
- `toText()` - Convert option to text representation

#### Removed functions
- `assertNull()` - Removed in favor of pattern matching
- `assertSome()` - Removed in favor of pattern matching

### [`Order`](https://mops.one/core/docs/Order)

#### New functions
- `allValues()` - Iterator over all order values (`#less`, `#equal`, `#greater`)

### [`Random`](https://mops.one/core/docs/Random)

The `Random` module has been completely redesigned in the core package with a new API that provides better control over random number generation and supports both pseudo-random and cryptographic random number generation.

```motoko no-repl
import Random "mo:core/Random";

persistent actor {
  transient let random = Random.crypto();

  public func main() : async () {
    let coin = await* random.bool(); // true or false
    let byte = await* random.nat8(); // 0 to 255
    let number = await* random.nat64(); // 0 to 2^64
    let numberInRange = await* random.natRange(0, 10); // 0 to 9
  }
}
```

#### New classes
- `Random` - Synchronous pseudo-random number generator for simulations and testing
- `AsyncRandom` - Asynchronous cryptographic random number generator using ICP entropy

#### Class methods
- `bool()` - Random choice between `true` and `false`
- `nat8()` - Random `Nat8` value in range `[0, 256)`
- `nat64()` - Random `Nat64` value in range `[0, 2^64)`
- `nat64Range(from, to)` - Random `Nat64` in range `[from, to)`
- `natRange(from, to)` - Random `Nat` in range `[from, to)`
- `intRange(from, to)` - Random `Int` in range `[from, to)`

#### New functions
- `emptyState()` - Initialize empty random number generator state
- `seedState()` - Initialize pseudo-random state with 64-bit seed
- `seed()` - Create pseudo-random generator from seed
- `seedFromState()` - Create pseudo-random generator from state
- `crypto()` - Create cryptographic random generator using ICP entropy
- `cryptoFromState()` - Create cryptographic generator from state

### [`Result`](https://mops.one/core/docs/Result)

#### New functions
- `all()` - Check all results in iterator
- `any()` - Check any result satisfies predicate
- `forOk()` - Apply function to `#ok` value
- `forErr()` - Apply function to `#err` value
- `fromBool()` - Create Result from boolean

### [`Text`](https://mops.one/core/docs/Text)

#### Renamed functions
- `toLowercase()` → `toLower()`
- `toUppercase()` → `toUpper()`
- `translate()` → `flatMap()`

#### New functions
- `isEmpty()` - Check if text is empty
- `reverse()` - Swap the order of characters
- `toText()` - Identity function

#### Removed functions
- `hash()`
- `fromList()` - Use `fromIter()` with list iterator instead
- `toList()` - Use `toIter()` and convert to list if needed

## Data structure migration examples

This section provides detailed migration examples showing how to convert common data structures from the `base` package to the `core` package. Each example demonstrates:

1. **Original implementation** using the `base` package with pre/post-upgrade hooks
2. **Updated implementation** using the `core` package with automatic stable memory support
3. **Migration pattern** using the new `with migration` syntax for seamless data structure conversion

:::tip
The new migration pattern allows you to automatically convert existing stable data from `base` package structures to `core` package structures during canister upgrades. The migration function runs once during the first upgrade and the converted data becomes the new stable state.
:::

### Understanding the migration pattern

The `with migration` syntax follows this structure:

```motoko no-repl
(
  with migration = func(
    state : {
      // Original state types
    }
  ) : {
    // New state types
  } = {
    // Conversion logic
  }
)
persistent actorApp {
  // New stable declarations
};
```

It's also possible to use a function defined in an imported module:

```motoko no-repl
import { migrate } "Migration";

(with migration = migrate)
persistent actorApp {
  // New stable declarations
};
```

This pattern ensures that existing stable data is preserved and converted to the new format during canister upgrades.

### `Buffer`

#### Original (`base`)

```motoko no-repl
import Buffer "mo:base/Buffer";

persistent actor{
  type Item = Text;

  stable var items : [Item] = [];
  let buffer = Buffer.fromArray<Item>(items);

  system func preupgrade() {
    items := Buffer.toArray(buffer);
  };

  system func postupgrade() {
    items := [];
  };

  public func add(item : Item) : async () {
    buffer.add(item);
  };

  public query func getItems() : async [Item] {
    Buffer.toArray(buffer);
  };
};
```

#### Updated (`core`)

```motoko no-repl
import List "mo:core/List";

(
  with migration = func(
    state : {
      var items : [App.Item];
    }
  ) : {
    list : List.List<App.Item>;
  } = {
    list = List.fromArray(state.items);
  }
)
persistent actorApp {
  public type Item = Text; // `public` for migration

  stable let list = List.empty<Item>();

  public func add(item : Item) : async () {
    List.add(list, item);
  };

  public query func getItems() : async [Item] {
    List.toArray(list);
  };
};
```

### `Deque`

#### Original (`base`)

```motoko no-repl
import Deque "mo:base/Deque";

persistent actor{
  type Item = Text;

  stable var deque = Deque.empty<Item>();

  public func put(item : Item) : async () {
    deque := Deque.pushBack(deque, item);
  };

  public func take() : async ?Item {
    switch (Deque.popFront(deque)) {
      case (?(item, newDeque)) {
        deque := newDeque;
        ?item;
      };
      case null { null };
    };
  };
};
```

#### Updated (`core`)

```motoko no-repl
import Deque "mo:base/Deque"; // For migration
import Queue "mo:core/Queue";

(
  with migration = func(
    state : {
      var deque : Deque.Deque<App.Item>;
    }
  ) : {
    queue : Queue.Queue<App.Item>;
  } {
    let queue = Queue.empty<App.Item>();
    label l loop {
      switch (Deque.popFront(state.deque)) {
        case (?(item, deque)) {
          Queue.pushBack(queue, item);
          state.deque := deque;
        };
        case null {
          break l;
        };
      };
    };
    { queue };
  }
) actor App {
  public type Item = Text; // `public` for migration

  stable let queue = Queue.empty<Item>();

  public func put(item : Item) : async () {
    Queue.pushBack(queue, item);
  };

  public func take() : async ?Item {
    Queue.popFront(queue);
  };
};
```

### `HashMap`

#### Original (`base`)

```motoko no-repl
import HashMap "mo:base/HashMap";
import Text "mo:base/Text";
import Iter "mo:base/Iter";

persistent actor{
  stable var mapEntries : [(Text, Nat)] = [];
  let map = HashMap.fromIter<Text, Nat>(mapEntries.vals(), 10, Text.equal, Text.hash);

  system func preupgrade() {
    mapEntries := Iter.toArray(map.entries());
  };

  system func postupgrade() {
    mapEntries := [];
  };

  public func update(key : Text, value : Nat) : async () {
    map.put(key, value);
  };

  public func remove(key : Text) : async ?Nat {
    map.remove(key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(map.entries());
  };
};
```

#### Updated (`core`)

```motoko no-repl
import Map "mo:core/Map";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

(
  with migration = func(
    state : {
      var mapEntries : [(Text, Nat)];
    }
  ) : {
    map : Map.Map<Text, Nat>;
  } = {
    map = Map.fromIter(state.mapEntries.vals(), Text.compare);
  }
)
persistent actor{
  stable let map = Map.empty<Text, Nat>();

  public func update(key : Text, value : Nat) : async () {
    Map.add(map, Text.compare, key, value);
  };

  public func remove(key : Text) : async ?Nat {
    Map.take(map, Text.compare, key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(Map.entries(map));
  };
};
```

### `OrderedMap`

#### Original (`base`)

```motoko no-repl
import OrderedMap "mo:base/OrderedMap";
import Text "mo:base/Text";
import Iter "mo:base/Iter";

persistent actor{
  let textMap = OrderedMap.Make<Text>(Text.compare);
  stable var map = textMap.empty<Nat>();

  public func update(key : Text, value : Nat) : async () {
    map := textMap.put(map, key, value);
  };

  public func remove(key : Text) : async ?Nat {
    let (newMap, removedValue) = textMap.remove(map, key);
    map := newMap;
    removedValue;
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(textMap.entries(map));
  };
};
```

#### Updated (`core`)

```motoko no-repl
import OrderedMap "mo:base/OrderedMap"; // For migration
import Map "mo:core/Map";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

(
  with migration = func(
    state : {
      var map : OrderedMap.Map<Text, Nat>;
    }
  ) : {
    map : Map.Map<Text, Nat>;
  } {
    let compare = Text.compare;
    let textMap = OrderedMap.Make<Text>(compare);
    let map = Map.fromIter<Text, Nat>(textMap.entries(state.map), compare);
    { map };
  }
)
persistent actor{
  stable let map = Map.empty<Text, Nat>();

  public func update(key : Text, value : Nat) : async () {
    Map.add(map, Text.compare, key, value);
  };

  public func remove(key : Text) : async ?Nat {
    Map.take(map, Text.compare, key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(Map.entries(map));
  };
};
```

### `OrderedSet`

#### Original (`base`)

```motoko no-repl
import OrderedSet "mo:base/OrderedSet";
import Text "mo:base/Text";
import Iter "mo:base/Iter";

persistent actor{
  type Item = Text;

  let textSet = OrderedSet.Make<Item>(Text.compare);
  stable var set = textSet.empty();

  public func add(item : Item) : async () {
    set := textSet.put(set, item);
  };

  public func remove(item : Item) : async Bool {
    let oldSize = textSet.size(set);
    set := textSet.delete(set, item);
    oldSize > textSet.size(set);
  };

  public query func getItems() : async [Item] {
    Iter.toArray(textSet.vals(set));
  };
};
```

#### Updated (`core`)

```motoko no-repl
import OrderedSet "mo:base/OrderedSet"; // For migration
import Set "mo:core/Set";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

(
  with migration = func(
    state : {
      var set : OrderedSet.Set<App.Item>;
    }
  ) : {
    set : Set.Set<App.Item>;
  } {
    let compare = Text.compare;
    let textSet = OrderedSet.Make<App.Item>(compare);
    let set = Set.fromIter(textSet.vals(state.set), compare);
    { set };
  }
)
persistent actorApp {
  public type Item = Text; // `public` for migration

  stable let set = Set.empty<Item>();

  public func add(item : Item) : async () {
    Set.add(set, Text.compare, item);
  };

  public func remove(item : Item) : async Bool {
    Set.delete(set, Text.compare, item);
  };

  public query func getItems() : async [Item] {
    Iter.toArray(Set.values(set));
  };
};
```

### `Trie`

#### Original (`base`)

```motoko no-repl
import Trie "mo:base/Trie";
import Text "mo:base/Text";
import Iter "mo:base/Iter";

persistent actor{
  type Key = Text;
  type Value = Nat;

  stable var trie : Trie.Trie<Key, Value> = Trie.empty();

  public func update(key : Key, value : Value) : async () {
    let keyHash = Text.hash(key);
    trie := Trie.put(trie, { key = key; hash = keyHash }, Text.equal, value).0;
  };

  public func remove(key : Key) : async ?Value {
    let keyHash = Text.hash(key);
    let (newTrie, value) = Trie.remove(trie, { key = key; hash = keyHash }, Text.equal);
    trie := newTrie;
    value;
  };

  public query func getItems() : async [(Key, Value)] {
    Iter.toArray(Trie.iter(trie));
  };
};
```

#### Updated (`core`)

```motoko no-repl
import Trie "mo:base/Trie"; // For migration
import Map "mo:core/Map";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

(
  with migration = func(
    state : {
      var trie : Trie.Trie<Text, Nat>;
    }
  ) : {
    map : Map.Map<Text, Nat>;
  } = {
    map = Map.fromIter(Trie.iter(state.trie), Text.compare);
  }
)
persistent actor{
  stable let map = Map.empty<Text, Nat>();

  public func update(key : Text, value : Nat) : async () {
    Map.add(map, Text.compare, key, value);
  };

  public func remove(key : Text) : async ?Nat {
    Map.take(map, Text.compare, key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(Map.entries(map));
  };
};
```

### `TrieMap`

#### Original (`base`)

```motoko no-repl
import TrieMap "mo:base/TrieMap";
import Text "mo:base/Text";
import Iter "mo:base/Iter";

persistent actor{
  stable var mapEntries : [(Text, Nat)] = [];
  let map = TrieMap.fromEntries<Text, Nat>(mapEntries.vals(), Text.equal, Text.hash);

  system func preupgrade() {
    mapEntries := Iter.toArray(map.entries());
  };

  system func postupgrade() {
    mapEntries := [];
  };

  public func update(key : Text, value : Nat) : async () {
    map.put(key, value);
  };

  public func remove(key : Text) : async ?Nat {
    map.remove(key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(map.entries());
  };
};
```

#### Updated (`core`)

```motoko no-repl
import Map "mo:core/Map";
import Text "mo:core/Text";
import Iter "mo:core/Iter";

(
  with migration = func(
    state : {
      var mapEntries : [(Text, Nat)];
    }
  ) : {
    map : Map.Map<Text, Nat>;
  } = {
    map = Map.fromIter(state.mapEntries.values(), Text.compare);
  }
)
persistent actor{
  stable let map = Map.empty<Text, Nat>();

  public func update(key : Text, value : Nat) : async () {
    Map.add(map, Text.compare, key, value);
  };

  public func remove(key : Text) : async ?Nat {
    Map.take(map, Text.compare, key);
  };

  public query func getItems() : async [(Text, Nat)] {
    Iter.toArray(Map.entries(map));
  };
};
```

### `TrieSet`

#### Original (`base`)

```motoko no-repl
import TrieSet "mo:base/TrieSet";
import Text "mo:base/Text";

persistent actor{
  type Item = Text;

  stable var set : TrieSet.Set<Item> = TrieSet.empty<Item>();

  public func add(item : Item) : async () {
    set := TrieSet.put<Item>(set, item, Text.hash(item), Text.equal);
  };

  public func remove(item : Item) : async Bool {
    let contained = TrieSet.mem<Item>(set, item, Text.hash(item), Text.equal);
    set := TrieSet.delete<Item>(set, item, Text.hash(item), Text.equal);
    contained;
  };

  public query func getItems() : async [Item] {
    TrieSet.toArray(set);
  };
};
```

#### Updated (`core`)

```motoko no-repl
import Set "mo:core/Set";
import Text "mo:core/Text";
import Iter "mo:core/Iter";
import TrieSet "mo:base/TrieSet";

(
  with migration = func(
    state : {
      var set : TrieSet.Set<Text>;
    }
  ) : {
    set : Set.Set<Text>;
  } = {
    set = Set.fromIter(TrieSet.toArray(state.set).vals(), Text.compare);
  }
)
persistent actorApp {
  public type Item = Text; // `public` for migration

  stable let set = Set.empty<Item>();

  public func add(item : Item) : async () {
    Set.add(set, Text.compare, item);
  };

  public func remove(item : Item) : async Bool {
    Set.delete(set, Text.compare, item);
  };

  public query func getItems() : async [Item] {
    Iter.toArray(Set.values(set));
  };
};
```

## Troubleshooting

### Version compatibility errors

If you encounter errors like `field Array_tabulateVar does not exist in module`, this indicates a version mismatch between your Motoko compiler and the `core` package. 

**Solution:**
2. Ensure you're using the latest Motoko compiler version
3. Update the `core` package to the latest version in your `mops.toml`
4. Clean and rebuild your project: `dfx stop && dfx start --clean`

### Migration issues

If you experience issues with the migration pattern:
1. Ensure your project structure follows the new `with migration` syntax exactly
2. Verify that all types referenced in the migration function are accessible (marked as `public` if needed)
3. Test the migration incrementally by converting one data structure at a time

For additional help, visit the ICP [developer forum](https://forum.dfinity.org/c/developers) or [Discord community](https://discord.internetcomputer.org).
