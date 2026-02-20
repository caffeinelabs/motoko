# Motoko core package

* [Array](Array.md) Provides extended utility functions on immutable Arrays (values of type `[T]`).
* [Blob](Blob.md) Module for working with Blobs (immutable sequences of bytes).
* [Bool](Bool.md) Boolean type and operations.
* [CertifiedData](CertifiedData.md) Certified data.
* [Char](Char.md) Module for working with Characters (Unicode code points).
* [Cycles](Cycles.md) Managing cycles within actors in the Internet Computer Protocol (ICP).
* [Debug](Debug.md) Utility functions for debugging.
* [Error](Error.md) Error values and inspection.
* [Float](Float.md) Double precision (64-bit) floating-point numbers in IEEE 754 representation.
* [Func](Func.md) Functions on functions, creating functions from simpler inputs.
* [Int](Int.md) Signed integer numbers with infinite precision (also called big integers).
* [Int16](Int16.md) Utility functions on 16-bit signed integers.
* [Int32](Int32.md) Utility functions on 32-bit signed integers.
* [Int64](Int64.md) Utility functions on 64-bit signed integers.
* [Int8](Int8.md) Utility functions on 8-bit signed integers.
* [InternetComputer](InternetComputer.md) Low-level interface to the Internet Computer.
* [Iter](Iter.md) Utilities for `Iter` (iterator) values.
* [List](List.md) A mutable growable array data structure with efficient random access and dynamic resizing.
* [Map](Map.md) An imperative key-value map based on order/comparison of the keys.
* [Nat](Nat.md) Natural numbers with infinite precision.
* [Nat16](Nat16.md) Utility functions on 16-bit unsigned integers.
* [Nat32](Nat32.md) Utility functions on 32-bit unsigned integers.
* [Nat64](Nat64.md) Utility functions on 64-bit unsigned integers.
* [Nat8](Nat8.md) Utility functions on 8-bit unsigned integers.
* [Option](Option.md) Typesafe nullable values.
* [Order](Order.md) Utilities for `Order` (comparison between two values).
* [Principal](Principal.md) Module for interacting with Principals (users and canisters).
* [PriorityQueue](PriorityQueue.md) A mutable priority queue of elements.
* [Queue](Queue.md) A mutable double-ended queue of elements.
* [Random](Random.md) Random number generation.
* [Region](Region.md) Byte-level access to isolated, virtual stable memory regions.
* [Result](Result.md) Module for error handling with the Result type.
* [Runtime](Runtime.md) Runtime utilities.
* [Set](Set.md) Imperative (mutable) sets based on order/comparison of elements.
* [Stack](Stack.md) A mutable stack data structure.
* [Text](Text.md) Utility functions for `Text` values.
* [Time](Time.md) System time utilities and timers.
* [Timer](Timer.md) Timers for one-off or periodic tasks. Applicable as part of the default mechanism.
* [Tuples](Tuples.md) Contains modules for working with tuples of different sizes.
* [Types](Types.md) Common types used throughout the core package.
* [VarArray](VarArray.md) Provides extended utility functions on mutable Arrays (`[var]`).
* [WeakReference](WeakReference.md) Module that implements a weak reference to an object.
* [internal/BTreeHelper](internal/BTreeHelper.md) 
* [internal/PRNG](internal/PRNG.md) Collection of pseudo-random number generators
* [internal/SortHelper](internal/SortHelper.md) 
* [pure/List](pure/List.md) Purely-functional, singly-linked list data structure.
* [pure/Map](pure/Map.md) Immutable, ordered key-value maps.
* [pure/Queue](pure/Queue.md) Double-ended queue of a generic element type `T`.
* [pure/RealTimeQueue](pure/RealTimeQueue.md) Double-ended immutable queue with guaranteed `O(1)` push/pop operations (caveat: high constant factor).
* [pure/Set](pure/Set.md) Pure (immutable) sets based on order/comparison of elements.
