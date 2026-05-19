---
title: "Enhanced orthogonal persistence"
description: "Enhanced orthogonal persistence implements the vision of efficient and scalable orthogonal persistence in Motoko that combines:"
sidebar:
  order: 2
---

Enhanced orthogonal persistence implements the vision of efficient and scalable orthogonal persistence in Motoko that combines:
* **Stable heap**: Persisting the program's main memory across canister upgrades.
* **64-bit heap**: Extending the main memory to 64-bit for large-scale persistence.

As a result, the use of secondary storage (explicit stable memory, dedicated stable data structures, DB-like storage abstractions) will no longer be necessary: Motoko developers can directly work on their normal object-oriented program structures that are automatically persisted and retained across program version changes.


Enhanced orthogonal persistence is enabled by default. (It was previously only offered via the compiler flag `--enhanced-orthogonal-persistence`, which is now redundant.)

:::tip
Despite the use of enhanced orthogonal persistence, it is strongly recommended to thoroughly test the upgrades of your application.
Moreover, it is advised to have a backup possibility for rescuing data even when upgrades fail, e.g. by controller-privileged data query calls.
:::

:::note
[Classical orthogonal persistence](./classical.md) with 32-bit main memory and Candid stabilization was the previous default compilation mode for `moc`. If necessary, it can be re-enabled with compiler flag `--legacy-persistence`.
See [orthogonal persistence modes](./index.md) for a comparison.
:::

## Design
Compared to the legacy orthogonal persistence in Motoko, this design offers:
* **Performance**: New program versions directly resume from the existing main memory and have access to the memory-compatible data.
* **Scalability**: The upgrade mechanism scales with larger heaps and in contrast to serialization, does not hit IC instruction limits.

Compared to the explicit use of stable memory, this design improves:
* **Simplicity**: Developers do not need to deal with explicit stable memory.
* **Performance**: No copying to and from the separate stable memory is necessary.

The enhanced orthogonal persistence is based on the following main properties:
* Extension of the IC to retain main memory on upgrades.
* Supporting 64-bit main memory on the IC.
* A long-term memory layout that is invariant to new compiled program versions.
* A fast memory compatibility check that is performed on each canister upgrade.
* Incremental garbage collection using a partitioned heap.

### Compatibility check
Upgrades are only permitted if the new program version is compatible with the old version, such that the runtime system guarantees a compatible memory structure.

Compatible changes for immutable types are largely analogous to the allowed Motoko subtype relation modulo some flexibility for actor fields, i.e.
* Adding or removing actor fields.
* Changing mutability of actor fields (`let` to `var` and vice-versa).
* Removing object fields.
* Adding variant fields.
* Changing `Nat` to `Int`.
* Supporting shared function parameter contravariance and return type covariance.
* Any other change according to Motoko's subtyping rule.

The runtime system checks migration compatibility on upgrade, and if not fulfilled, rolls back the upgrade. This compatibility check serves as an additional safety measure on top of the `dfx` warning that can be bypassed by users.

Any more complex change can be performed with programmatic instruction, see [explicit migration](../data-persistence.md#explicit-migration).

### Migration path
When migrating from the old serialization-based stabilization to the new persistent heap, the old data is deserialized one last time from stable memory and then placed in the new persistent heap layout. Once operating on the persistent heap, the system should prevent downgrade attempts to the old serialization-based persistence.

#### Graph-copy-based stabilization
Assuming that the persistent memory layout needs to be changed in the future, the runtime system supports serialization and deserialization to and from stable memory in a defined data format using graph-copy-based stabilization. Arbitrarily large data can be serialized and deserialized beyond the instruction and working set limit of upgrades. Large data serialization and deserialization is split in multiple messages, running before and/or after the IC upgrade to migrate large heaps. Other messages will be blocked during this process and only the canister owner or the canister controllers are permitted to initiate this process.

This will only be needed in rare situations when Motoko's implementation changes its internal memory layout. Users will then be instructed to explicitly initiate this migration.

#### Usage
Graph-copy-based stabilization can be performed in three steps:

1. Initiate the explicit stabilization before the upgrade:

```
dfx canister call CANISTER_ID __motoko_stabilize_before_upgrade "()"
```

2. Run the upgrade:

```
dfx deploy CANISTER_ID
```

3. Complete the explicit destabilization after the upgrade:

```
dfx canister call CANISTER_ID __motoko_destabilize_after_upgrade "()"
```

Remarks:
* When receiving the `dfx` error "The request timed out." during explicit stabilization, upgrade, or destabilization, one can simply repeat the call until it completes.
* Step 3 (explicit destabilization) may not be needed if the corresponding operation fits into the upgrade message.

### Old stable memory
The old stable memory remains equally accessible as secondary (legacy) memory with the new support. Therefore, stable regions can be combined with orthogonal persistence.

## IC main memory retention

The IC introduces a new upgrade option `wasm_memory_persistence` to control the retention of the canister's Wasm main memory.
* `wasm_memory_persistence = opt keep` retains the Wasm main memory and is required for Motoko's enhanced orthogonal persistence. The IC prevents using this options for canisters with classical persistence.
* `wasm_memory_persistence = null` uses the classical persistence, replacing the main memory. However, a safety check is implemented to prevent that main memory is not accidentally dropped for enhanced orthogonal persistence.
* The other option `replace` is not recommended as it drops Wasm main memory, even for enhanced orthogonal persistence, leading to potential data loss.
