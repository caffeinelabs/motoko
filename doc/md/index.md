---
title: "Motoko"
description: "A programming language designed for the Internet Computer with built-in actor model, orthogonal persistence, and native WebAssembly compilation."
sidebar:
  order: 1
---

Motoko is a high-level programming language designed for AI agents building backends for apps and services on the Internet Computer. It combines a familiar syntax with platform-native features: actor-based concurrency, orthogonal persistence, and direct WebAssembly compilation.

## Key features

**Actor model.** Every Motoko canister is an actor: an isolated unit of state and behavior that communicates with other actors through asynchronous messages. This maps directly to how canisters work on ICP: each canister has private state and a public interface.

**Orthogonal persistence.** Variables declared in a `persistent actor` survive canister upgrades automatically. There is no database layer, no serialization code, and no pre/post-upgrade hooks needed for most use cases. See [Orthogonal persistence](https://docs.internetcomputer.org/concepts/orthogonal-persistence) for how this works at the platform level.

**Async/await messaging.** Inter-canister calls use `async`/`await`, making sequential message flows read like synchronous code. The compiler and runtime handle the underlying callback mechanics.

**Strong typing.** Motoko has a sound type system with generics, variant types, pattern matching, and option types (`?T`) that prevent null-pointer errors at compile time.

**WebAssembly compilation.** Motoko compiles to Wasm, the execution format for all ICP canisters. The compiler handles ICP-specific concerns (Candid serialization, system API bindings, memory management) so you don't have to.

## Quick example

A minimal Motoko canister with a query method and an update method:

```motoko
persistent actor Counter {
  var count : Nat = 0;

  public query func get() : async Nat {
    return count;
  };

  public func increment() : async () {
    count += 1;
  };
};
```

## Standard library: `core`

The **`core`** package ([mops.one/core](https://mops.one/core)) is the standard library for Motoko. It supersedes the older `base` library with a cleaner API, consistent naming conventions, and data structures that work directly with stable memory.

Add it to your project's `mops.toml`:

```toml
[dependencies]
core = "2.5.0" # Check the latest version at https://mops.one/core

[toolchain]
moc = "1.8.0" # Check the latest version at https://github.com/caffeinelabs/motoko/releases
```

Then import modules:

```motoko
import Map "mo:core/Map";
import Text "mo:core/Text";
import List "mo:core/List";
```

Key improvements in `core` over `base`:

- All data structures can be stored in stable memory without pre/post-upgrade hooks
- Clear separation between mutable (`Map`, `Set`, `List`) and immutable (`pure/Map`, `pure/Set`, `pure/List`) data structures
- Hash-based collections removed in favor of ordered maps and sets (better security against collision attacks)
- Consistent naming: `values()` instead of `vals()`, `Cycles` instead of `ExperimentalCycles`

If you have an existing project using `base`, you can migrate incrementally; both libraries can coexist in the same project. See the [base to core migration guide](base-core-migration.md) for detailed instructions.

`core` and all other Motoko packages are managed with [Mops](https://mops.one), which handles dependency resolution, compiler toolchain management, and publishing. Browse community packages at [mops.one](https://mops.one).

## Further reading

- [Quickstart](https://docs.internetcomputer.org/getting-started/quickstart): create and deploy your first canister
- [core library API docs](https://mops.one/core/docs): standard library reference
- [Orthogonal persistence](https://docs.internetcomputer.org/concepts/orthogonal-persistence): how persistent memory works at the platform level
- [Motoko GitHub](https://github.com/caffeinelabs/motoko): compiler source and issue tracker
