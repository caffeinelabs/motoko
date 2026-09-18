# Memory Management and Persistence in Wasm/IC/Motoko

## Preliminaries

### Wasm

#### Values

Wasm has two kinds of data (assuming the upcoming reference type proposal):

* *Numerics* (int32/64, float32/64): *transparent* data; bit pattern observable; can be stored in memory
* *References* (anyref, funcref, ...): *opaque* data; representation not exposed, usually implemented as pointers; can be stored in tables but not memory

#### State

Wasm currently has 3 forms of mutable state, all of which can be ex/imported from a module as concrete entities:

* *Globals*: can each store a single value; can be mutated (if defined as mutable)
* *Memory*: an array of raw bytes; can be mutated and grown; can store numeric values
* *Tables*: an array of references; can be mutated and grown; can store reference values

A reference can also point to *host objects* provided by the embedder, which could introduce additional forms of state at the embedder's discretion.

#### Further Evolution

In the future (with the GC proposal), Wasm will have a 4th form of mutable state:

* *Heap*: memory area used for allocating GCed structures; distinct from Memory; controlled by VM

The Heap is *not* an explicit entity that can be im/exported, only individual references to structures on the heap can be passed.

Note: It is highly likely that several managed languages implemented on Wasm will eventually use Wasm GC.
However, in our case, it would require snapshotting the Wasm managed heap which is currently not possible for `wasmtime`.
Moreover, the GC implemented on the managed heap does probably not fit the IC with hard instruction limits. 
A fully incremental GC would be needed, which is currently not implemented in any Wasm engine (often only using reference counting or a GC that has worst-case unbounded pauses).
Conceptually, enhanced orthogonal persistence could be implemented on Wasm GC.

### Internet Computer (IC)

#### API Types

The Hypervisor API introduces a set of Wasm host reference types to represent actors, functions, and their arguments:

* *Modules*: a reference to a compiled but not yet instantiated actor (Wasm module); can be linked and instantiated multiple times
* *Actors*: a reference to an instantiated and active actor
* *Functions*: a reference to an exported actor method; invoking it initiates an asynchronous message send
* *Data buffers*: a reference to an immutable array of bytes; used to pass binary data or aggregates of numeric values along messages
* *Element buffers*: a reference to an immutable array of references; used to pass aggregates of references along messages

All references are *sharable*, i.e., can be passed between actors as message arguments.
Other than actors, all reference types must be pure (immutable and without identity) to prevent shared state and allow transparent copying by the implementation.
Element buffers can encode arbitrary object trees.

## Language Implementation Rationales

### Representing Data Structures

There are 3 possible ways of representing structured data in Wasm/IC.

#### Using Wasm Memory <- Chosen Design

All data structures are laid out and managed in Wasm memory by the compiler and the runtime system.
Function references are stored via indirections through a Wasm table.

   Pros:
   1. local data access maximally efficient
   2. may ease transparent persistence (see below)

   Cons:
   1. message arguments require de/serialisation into IC buffers on both ends (in addition to the de/serialisation steps already performed by IC)
   2. each actor must ship its own instance of a GC (for both memory and table) and de/serialisation code
   3. all function references require an indirection
   4. more implementation effort

#### Using IC API

All data structures are mapped to element and data buffers created through the API.

   Pros:
   1. quickest to implement and get off the ground
   2. no extra de/serialisation step for message sends
   3. easy transition path to Wasm Heap

   Cons:
   1. local access to data involves system call overhead
   2. requires adding mutable buffer types to IC API
   3. mixed numeric/reference structures must be split into data and element buffers
   4. IC-only solution unlikely to be adopted by other languages

#### Using Wasm Heap

All data structures are represented as Wasm GCed objects.

   Pros:
   1. local access should be as fast as Memory
   2. GC is handled by VM
   3. no extra de/serialisation step for message args (provided IC API can handle Wasm structs)
   4. can freely mix numerics and references
   5. this is likely the route most future languages compiling to Wasm will take

   Cons:
   1. Wasm GC is 1-2 years out
   2. unclear how to implement transparent persistence (see below)

## Persistence

See [enhanced orthogonal persistence](OrthogonalPersistence.md).
