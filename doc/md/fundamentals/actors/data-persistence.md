---
title: "Data persistence"
description: "One key feature of Motoko is its ability to automatically persist the program's state without explicit user instruction."
sidebar:
  order: 3
---

One key feature of Motoko is its ability to automatically persist the program's state without explicit user instruction. This is called **orthogonal persistence**. Data persists across transactions and canister upgrades.

Motoko data persistence is not simple, but it prevents data corruption or loss while being efficient at the same time. No database, stable memory API, or stable data structure is required to retain state across upgrades. Instead, a simple `stable` keyword is sufficient to declare a data structure of arbitrary shape persistent, even if the structure uses sharing, has a deep complexity, or contains cycles transfers.

In comparison to other supported languages for building canisters, such as Rust, data persistence must be achieved through explicit use of stable data structures and stable memory, as other languages are not designed for orthogonal persistence and instead rearranges memory structures in an uncontrolled manner on re-compilation or at runtime.

## Declaring stable variables

Within an actor, you can configure which part of the program is considered to be persistent (retained across upgrades) and which part is ephemeral (reset on upgrades).

More precisely, each `let` and `var` variable declaration in an actor can specify whether the variable is `stable` or `transient`. If you don’t provide a modifier, the variable is assumed to be `transient` by default.

* `stable` means that all values directly or indirectly reachable from that stable variable are considered persistent and are automatically retained across upgrades. This is the primary choice for most of the program's state.

* `transient` means that the variable is re-initialized on upgrade such that the values referenced by the transient variable are discarded, unless the values are transitively reachable by other variables that are stable. `transient` is only used for temporary state or references to high-order types, such as local function references.

:::note

You can only use the `stable`, `transient` (or legacy `flexible`) modifier on `let` and `var` declarations that are **actor fields**. You cannot use these modifiers anywhere else in your program.

:::

The following is a simple example of how to declare a stable counter that can be upgraded while preserving the counter’s value:

```motoko file=<motokoExamples>/StableCounter.mo
```

When you compile and deploy a canister for the first time, all transient and stable variables in the actor are initialized in sequence. When a canister is upgraded, all stable variables that existed in the previous version of the actor are pre-initialized with their old values and the remaining transient and any newly-added stable variables are initialized in sequence.

Starting with Motoko v0.13.5, if you prefix the `actor` keyword with the keyword `persistent`, then all `let` and `var` declarations of the actor or actor class are implicitly declared `stable`. Only `transient` variables will need an explicit `transient` declaration.

Using a `persistent` actor can help avoid unintended data loss. It is the recommended declaration syntax for actors and actor classes. The non-`persistent` declaration is provided for backwards compatibility.

```motoko file=<motokoExamples>/PersistentCounter.mo
```

## Stable types

The Motoko compiler must ensure that stable variables are compatible with the upgraded program. To achieve this, every `stable` variable must have a stable type. A type is stable if removing all `var` modifiers from it results in a shared type.

The only difference between stable types and shared types is the former’s support for mutation. Like shared types, stable types are restricted to first-order data, excluding local functions and structures built from local functions (such as class instances). Excluding local functions is required because the meaning of a function value, consisting of both data and code, cannot easily be preserved across an upgrade while the value of plain data, mutable or not, can be.

:::note

In general, classes are not stable because they can contain local functions. However, a plain record of stable data is a special case of object types that are stable. Moreover, references to actors and shared functions are also stable, allowing you to preserve their values across upgrades.

:::

## Converting non-stable types into stable types

For variables that do not have a stable type, there are two options for making them stable:

1. Use a `stable` module for the type, such as:

  - [StableBuffer](https://github.com/canscale/StableBuffer)
  - [StableHashMap](https://github.com/canscale/StableHashMap)
  - [StableRBTree](https://github.com/canscale/StableRBTree)

:::note
Unlike stable data structures in the Rust CDK, these modules do not use stable memory but instead rely on orthogonal persistence. The adjective "stable" only denotes a stable type in Motoko.
:::

2. Extract the state in a stable type and wrap it in the non-stable type.

For example, the stable type `TemperatureSeries` covers the persistent data, while the non-stable type `Weather` wraps this with additional methods (local function types).

```motoko no-repl file=<motokoExamples>/WeatherActor.mo
```

__Discouraged and not recommended__: [Pre- and post-upgrade hooks](#preupgrade-and-postupgrade-system-methods) allow copying non-stable types to stable types during upgrades. This approach is error-prone and does not scale for large data. **Per best practices, using these methods should be avoided if possible.** Conceptually, it also does not align well with the idea of orthogonal persistence.

## Stable type signatures

The collection of stable variable declarations in an actor can be summarized in a stable signature. The textual representation of an actor’s stable signature resembles the internals of a Motoko actor type. It specifies the names, types, and mutability of the actor’s stable fields, possibly preceded by relevant Motoko type declarations.


```motoko no-repl
actor {
  stable x : Nat;
  stable var y : Int;
  stable z : [var Nat];
};
```

:::tip

You can emit the stable signature of an actor or actor class to a `.most` file using `moc` compiler option `--stable-types`. You should never need to author your own `.most` file.

:::

A stable signature `<stab-sig1>` is stable-compatible with another signature `<stab-sig2>` if, for every stable field `<id>: T` in `<stab-sig1>`, the following condition holds:

- `<stab-sig2>` has a stable field `<id>: U` such that `T` is a stable subtype of `U`.

#### Notes
- `<stab-sig2>` may include additional fields not present in `<stab-sig1>`.
- Matching fields may differ in mutability (`var` vs. non-`var`).

`<stab-sig1>` represents the signature of an older version, and `<stab-sig2>` represents a newer version.

The stable subtyping condition ensures that the final value of a field from the old version can be safely used as the initial value of that field in the new version, without loss of data.

:::tip

You can check the stable-compatibility of two `.most` files containing stable signatures using the `moc` compiler option `--stable-compatible file1.most file2.most`.

:::


## Upgrade safety

When upgrading a canister, it is important to verify that the upgrade can proceed without:

-   Introducing an incompatible change in stable declarations.
-   Breaking clients due to a Candid interface change.

With [enhanced orthogonal persistence](./orthogonal-persistence/enhanced.md), Motoko rejects incompatible changes of stable declarations during an upgrade attempt.
Moreover, `dfx` checks the two conditions before attempting the upgrade and warns users as necessary.

A Motoko canister upgrade is safe provided:

-  The canister’s Candid interface evolves to a Candid subtype. You can check valid Candid subtyping between two services described in `.did` files using the [`didc` tool](https://github.com/dfinity/candid) with argument `check file1.did file2.did`.
-  The canister’s Motoko stable signature evolves to a stable-compatible one.

:::danger
With [classical orthogonal persistence](./orthogonal-persistence/classical.md), the upgrade can still fail due to resource constraints. This is problematic as the canister can then not be upgraded. It is therefore strongly advised to test the scalability of upgrades extensively. This does not apply to enhanced orthogonal persistence.
:::


## Upgrading a canister

If you have a Motoko canister that has already been deployed, then you make changes to that canister's code and want to upgrade it, the command `dfx deploy` will check that the interface is compatible, and if not, displays a warning:

```
You are making a BREAKING change. Other canisters or frontend clients relying on your canister may stop working.
```

Motoko canisters using enhanced orthogonal persistence implement an extra safeguard in the runtime system to ensure that the stable data is compatible to exclude any data corruption or misinterpretation. Moreover, `dfx` also warns about incompatibility and dropping stable variables.

## Data migration

Often, data representation changes with a new program version. For orthogonal persistence, it is important the language is able to allow flexible data migration to the new version.

Motoko supports two kinds of data migrations: Implicit migration and explicit migration.

### Implicit migration

Migration is automatically supported when the new program version is stable-compatible with the old version. The runtime system of Motoko then automatically handles the migration on upgrade.

More precisely, the following changes can be implicitly migrated:
* Adding actor fields.
* Changing the mutability of an actor field.
* Adding variant fields.
* Changing `Nat` to `Int`.
* Any change that is allowed by Motoko stable subtyping rules. These are similar to Motoko subtyping, but stricter, and do not allow dropping of record fields or promotion to the type `Any`, either of which can result in data loss.

Motoko versions prior to v0.14.6 allowed actor fields to be dropped or promoted to `Any`, but such changes now require explicit migrations.
The rules have been strengthened to prevent accidental loss of data.

### Explicit migration

More complex migration patterns, which involve non-trivial data transformations, are possible. However, they require additional coding effort and careful handling.

One common approach is to replace a set of stable variables with new ones of different types through a sequence of upgrade steps. Each step incrementally transforms the program state, ultimately producing the desired structure and values.

For this purpose, a three step approach is taken:
1. Introduce new variables of the desired types while keeping the old declarations.
2. Write logic to copy the state from the old variables to the new variables upon upgrade.
3. Drop the old declarations once all data has been migrated.

A cleaner, more maintainable solution, is to declare an explicit migration expression that is used to transform a subset of existing stable variables into a subset of replacement stable variables.

Both of these data migration paths are supported by static and dynamic checks that prevent data loss or corruption. A user may still lose data due to coding errors, so should tread carefully.

For more information, see the [example of explicit migration](./compatibility.md#explicit-migration-using-a-migration-function) and the
reference material on [migration expressions](../../reference/language-manual.md#migration-expressions).

## Legacy features

:::danger
Using the pre- and post-upgrade system methods is discouraged. It is error-prone and can render a canister unusable. In particular, if a `preupgrade` method traps and cannot be prevented from trapping by other means, then your canister may be left in a state in which it can no longer be upgraded. Per best practices, using these methods should be avoided if possible.
:::

Motoko supports user-defined upgrade hooks that run immediately before and after an upgrade. These upgrade hooks allow triggering additional logic on upgrade.
They are declared as `system` functions with special names, `preugrade` and `postupgrade`. Both functions must have type `: () → ()`.

If `preupgrade` raises a trap, hits the instruction limit, or hits another IC computing limit, the upgrade can no longer succeed and the canister is stuck with the existing version.

`postupgrade` is not needed, as the equal effect can be achieved by introducing initializing expressions in the actor, e.g. non-stable `let` expressions or expression statements.


