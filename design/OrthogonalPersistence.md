
# Enhanced Orthogonal Persistence

This implements the vision of **enhanced orthogonal persistence** in Motoko that combines:
* **Stable heap**: Persisting the program main memory across canister upgrades.
* **64-bit heap**: Extending the main memory to 64-bit for large-scaled persistence.

As a result, the use of secondary storage (explicit stable memory, dedicated stable data structures, DB-like storage abstractions) will no longer be necessary: Motoko developers can directly work on their normal object-oriented program structures that are automatically persisted and retained across program version changes.

## Activation
Enhanced orthogonal persistence is the only persistence mode and is enabled by default (it was previously offered via the compiler flag `--enhanced-orthogonal-persistence`, which is now redundant).
Classical persistence with 32-bit main memory and Candid stabilization was removed in the 1.16 → v2 migration.
See `design/PersistenceModes.md` for more information.

## Advantages
Compared to the existing orthogonal persistence in Motoko, this design offers:
* **Performance**: New program versions directly resume from the existing main memory and have access to the memory-compatible data.
* **Scalability**: The upgrade mechanism scales with larger heaps and in contrast to serialization, does not hit IC instruction limits.

Compared to the explicit use of stable memory, this design improves:
* **Simplicity**: Developers do not need to deal with explicit stable memory.
* **Performance**: No copying to and from the separate stable memory is necessary.

## Design
The enhanced orthogonal persistence is based on the following main properties:
* Extension of the IC to retain main memory on upgrades.
* Supporting 64-bit main memory on the IC.
* A long-term memory layout that is invariant to new compiled program versions.
* A fast memory compatibility check performed on each canister upgrade.
* Incremental garbage collection using a partitioned heap.

## IC Main Memory Retention

The IC introduces a new upgrade option `wasm_memory_persistence` to control the retention of the canister's Wasm main memory.
* `wasm_memory_persistence = opt keep` retains the Wasm main memory and is required for Motoko's enhanced orthogonal persistence. The IC prevents using this options for canisters with classical persistence.
* `wasm_memory_persistence = null` uses the classical persistence, replacing the main memory. However, a safety check is implemented to prevent that main memory is not accidentally dropped for enhanced orthogonal persistence.
* The other option `replace` is not recommended as it drops main memory, even for enhanced orthogonal persistence.

### Memory Layout
In a co-design between the compiler and the runtime system, the main memory is arranged in the following structure, invariant of the compiled program version:
* Lower 4MB: Rust call stack.
* Space between 4MB and 4.5MB: Limited reserved space Wasm data segments, only used for the Motoko runtime system.
* Between 4.5MB and 5MB: Persistent metadata.
* Thereafter: Dynamic heap space. Fix start address at 5MB.

### Persistent Metadata
The persistent metadata describes all anchor information for the program to resume after an upgrade. 

More specifically, it comprises:
* A stable heap version that allows evolving the persistent memory layout in the future.
* The stable subset of the main actor, containing all stable variables declared in the main actor.
* A descriptor of the stable static types to check memory compatibility on upgrades.
* The runtime state of the garbage collector, including the dynamic heap metadata and memory statistics.
* A reserve for future metadata extensions.

### Compatibility Check
Upgrades are only permitted if the new program version is compatible with the old version, such that the runtime system guarantees a compatible memory structure.

Compatible changes for immutable types are largely analogous to the allowed Motoko subtype relation, e.g.
* Adding or removing actor fields.
* Removing object fields.
* Adding variant fields.
* `Nat` to `Int`.
* Shared function parameter contravariance and return type covariance.

The existing IDL-subtype functionality is reused with some adjustments to check memory compatibility: The compiler generates the type descriptor, a type table, that is recorded in the persistent metadata. Upon an upgrade, the new type descriptor is compared against the existing type descriptor, and the upgrade only succeeds for compatible changes.

This compatibility check serves as an additional safety measure on top of the DFX Candid subtype check that can be bypassed by users (when ignoring a warning). Moreover, in some aspects, the memory compatibility rules differ to the Candid sub-type check:
* Top-level actor fields (`stable` fields) can change mutability (`let` to `var` and vice-versa).
* Support of variable (MutBox) with type invariance.
* Types cannot be made optional (no insertion of Option).
* Same arity for function parameters and function return types (no removed optional parameters, no additional optional results).
* Records cannot introduce additional optional fields.
* Same arity for tuple types (no insertion of optional items).
* Records and tuples are distinct.

### Garbage Collection
The implementation focuses on the incremental GC and abandons the other GCs because the GCs use different memory layouts. For example, the incremental GC uses a partitioned heap with objects carrying a forwarding pointer.

The incremental GC is chosen because it is designed to scale on large heaps and the stable heap design also aims to increase scalability.

The garbage collection state needs to be persisted and retained across upgrades. This is because the GC may not yet be completed at the time of an upgrade, such that object forwarding is still in use. The heap partition structure is described by a linked list of partition tables that is reachable from the GC state.

The garbage collector uses two kinds of roots:
* Persistent roots: These refer to root objects that need to survive canister upgrades.
* Transient roots: These cover additional roots that are only valid in a specific version of a program and are discarded on an upgrade.

The persistent roots are registered in the persistent metadata and comprise:
* All stable variables of the main actor, only stored during an upgrade.
* The stable type table.

The transient roots are referenced by the Wasm data segments and comprise:
* All canister variables of the current version, including flexible variables.

### Main Actor
On an upgrade, the main actor is recreated and existing stable variables are recovered from the persistent root. The remaining actor variables, the flexible fields as well as new stable variables, are (re)initialized. 

As a result, the GC can collect unreachable flexible objects of previous canister versions. Unused stable variables of former versions can also be reclaimed by the GC.

### No Static Heap
The static heap is abandoned and former static objects need to be allocated in the dynamic heap. This is because these objects may also need to survive upgrades and the persistent main memory cannot accommodate a growing static heap of a new program version in front of the existing dynamic heap. The incremental GC also operates on these objects, meaning that forwarding pointer resolution is also necessary for these objects. 

For memory and runtime efficiency, object pooling is implemented for compile-time-known constant objects (with side-effect-free initialization), i.e. those objects are already created on program initialization/upgrade in the dynamic heap and thereafter the reference to the corresponding prefabricated object is looked up whenever the constant value is needed at runtime.

The runtime system avoids any global Wasm variables for state that needs to be preserved on upgrades. Instead, such global runtime state is stored in the persistent metadata.

### Wasm Data Segments
Only passive Wasm data segments are used by the Motoko compiler and runtime system. In contrast to ordinary active data segments, passive segments can be explicitly loaded to a dynamic address.

This simplifies two aspects: 
* The generated Motoko code can contain arbitrarily large data segments (to the maximum that is supported by the IC). The segments can be loaded to the dynamic heap when needed.
* The IC can simply retain the main memory on an upgrade without needing to patch any active data segments of the new program version to the persistent main memory.

However, more specific handling is required for the Rust-implemented runtime system (RTS): The Rust-generated active data segment of the runtime system is changed to the passive mode and loaded to the expected static address on the program start (canister initialization and upgrade). The location and size of the RTS data segments is therefore limited to a defined reserve of 512 KB, see above. This is acceptable because the RTS only requires a controlled small amount of memory for its data segments, independent of the compiled Motoko program.

### Null Sentinel
As an optimization, the top-level `null` pointer is represented as a constant sentinel value pointing to the last unallocated Wasm page. This allows fast null tests without involving forwarding pointer resolution of potential non-null comparand pointers.

### Memory Capacity
The canister has no upfront knowledge of the maximum allocatable Wasm main memory in 64-bit address space, as there is no IC API call to query the main memory limit. This limit may also be increased in future IC releases.

Therefore, a mechanism is implemented to deal with an unknown and dynamically increasable main memory capacity offered by the IC. This is needed in two cases:

* GC reserve (strict): The runtime system ensures sufficient free space to allow garbage collection at all times, even if the heap is full. For this purpose, the runtime system already pre-allocates the reserve, to be sure that the reserve is available despite the unknown capacity. As an optimization, this pre-allocation is skipped when the memory demand including the GC reserve is below a guaranteed minimum Wasm memory limit of the IC, e.g. 4GB or 6GB.
* GC scheduling (heuristic): The GC schedules at high frequency when memory is becoming scarce. For this purpose, the GC maintains an assumption of the minimum memory limit and probes the supposed limit when the heap size approaches this limit. If the allocation succeeds, the assumed limit is increased. Otherwise, the critical high-frequency GC scheduling rate is activated.

In both cases, the runtime system tries to reduce Wasm memory allocations as much as possible, i.e. not pre-allocating memory for small heap sizes, and not probing an allocation in certain memory ranges by assuming that the IC only offers main memory of a certain granularity, e.g. multiples of 2GB. To save instructions, the critical GC scheduling is only activated when reaching the actual memory limit. Moreover, the mechanism can handle an increased memory capacity at runtime, e.g. when the IC is upgraded to a new release with a higher memory limit.

### Migration Path
When migrating from the old serialization-based stabilization to the new persistent heap, the old data is deserialized one last time from stable memory and then placed in the new persistent heap layout. Once operating on the persistent heap, the system should prevent downgrade attempts to the old serialization-based persistence. 

Assuming that the persistent memory layout needs to be changed in the future, the runtime system supports serialization and deserialization to and from stable memory in a defined data format using graph copy. 

### Graph Copy
The graph copy is an alternative persistence mechanism that will be only used in the rare situation when the persistent memory layout will be changed in the future. Arbitrarily large data can be serialized and deserialized beyond the instruction and working set limit of upgrades: Large data serialization and deserialization is split in multiple messages, running before and/or after the IC upgrade to migrate large heaps. Of course, other messages will be blocked during this process and only the canister owner or the canister controllers are permitted to initiate this process. 

Graph copying needs to be explicitly initiated before an upgrade to new Motoko version that is incompatible to the current enhanced orthogonal persistent layout. For large data, the graph copy needs to be manually completed after the actual upgrade.

```
dfx canister call CANISTER_ID __motoko_stabilize_before_upgrade "()"
dfx deploy CANISTER_ID
dfx canister call CANISTER_ID __motoko_destabilze_after_upgrade "()"
```

More detailed information and instructions on graph copy are contained in `design/GraphCopyStabilization.md`.

### Old Stable Memory
The old stable memory remains equally accessible as secondary (legacy) memory with the new support.

## Current Limitations
* The memory footprint of a program increases with 64 bit as the word size for scalars and pointers are doubled. In turn, in some cases, boxing can be avoided due to larger word size which again reduces memory demand.
* Freeing old object fields: While new program versions can drop object fields, the runtime system should also delete the redundant fields of persistent objects of previous program versions. This could be realized during garbage collection when objects are copied. For this purpose, the runtime system may maintain a set of field hashes in use and consult this table during garbage collection. Another, probably too restrictive solution could be to disallow field removal (subtyping) on object upgrades during the memory compatibility check.
