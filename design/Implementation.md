# Motoko Implementation Strategy

## Subtyping

* All subtyping is non-coercive and thus zero-cost and higher-order.

* Q: Also have first-order implicit coercibility?


## Heap

* Uniform representation with a defined word size. 
[Enhanced orthogonal persistence](OrthogonalPersistence.md) uses 64-bit words.

* Use pointer tagging in the LSB:
  - 1 for pointers, 0 for scalars.
  - Scalars are real value shifted left by 1, lowest bit clear.
  For [enhanced orthogonal persistence](OrthogonalPersistence.md), the types of scalars are additionally tagged.
  
* Garbage collected.


## Primitive types

* Nat and Int compile to heap-allocated big nums; unboxed for small numbers `<= 31` bit.

* Nat8/16 compile to unboxed scalars; 
  On a 32-bit heap, Nat32/64 are boxed. 
  On a 64-bit heap, only Nat64 is boxed, while Nat32 remains unboxed.
  - May unbox locally.

* Characters are scalars (unicode code points).

* Text is heap-allocated. Using ropes for concatenations.


## Tuples

* Heap-allocated when first-class.

* Flattened when used as function parameter or result.

* Q: How to avoid calling convention mismatch when instantiating polymorphic function with tuple type?
  - Don't make tuples subtypes of Any, thereby disallowing their use in instantiation?


## Options

* Represent null as null pointer.

* Q: union or sum?
  - If union, don't make it subtype of Any?
  - If sum, use coercibility?


## Objects

* Compile to heap-allocated lookup table over field name hashes.
  - Either as hash table or ordered list.

* Records of closures and regular public fields; private fields are closed over.

* Field names are hashed; hashes are required to be unique per type.


## Functions

* Compile to heap-allocated closures as usual.

* Definitions lifted to the surrounding actor.

* Closed-over locals need indirection through heap if mutable or if definedness cannot be proved.


## Actor Objects

* Compile to immediately instantiated modules.

* Private fields become regular globals or functions.

* Public methods become exported functions.
  - In general, have to generate wrapper functions to forward external calls to pre-existing local closure.

* Instantiate via hypervisor; build record of functions forwarding to extracted funcrefs.

* Closed-over definitions must be turned into imports.
  - Types directly mapping to Wasm types become (immutable) global imports.
  - Q: What about heap-allocated types? Serialise?
  - Q: What about functions?
  - Q: Disallow closing over mutable definitions?
  - Q: What about definedness checks?

* Closure imports are pre-bound via hypervisor interface.


## Actor Classes

* Constructor compiles to function instantiating the actor object.


## Async

TODO


## Serialisation

TODO

## Persistence

See [enhanced orthogonal persistence](OrthogonalPersistence.md).

# Hypervisor Extensions needed

* Closures as funcrefs?

* Binding imports for modulerefs

* Efficiently extract all exports from an actor
  - as sequence of reference in an elembuf?
