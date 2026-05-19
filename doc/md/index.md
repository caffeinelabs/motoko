---
title: "Motoko"
description: "A programming language designed for the Internet Computer with built-in actor model, orthogonal persistence, and seamless WebAssembly integration."
sidebar:
  order: 1
---

Motoko is a next-generation, high-level programming language designed to empower developers building backends for apps and services on the Internet Computer. While it draws on several popular modern languages, it introduces features purpose-built for ICP: actor-based concurrency, orthogonal persistence, and seamless WebAssembly integration.

## Key features

**Actor model.** Every Motoko canister is an actor: an isolated unit of state and behavior that communicates with other actors through asynchronous messages. This maps directly to how canisters work on ICP, where each canister has private state and a public interface.

**Orthogonal persistence.** Variables declared in a `persistent actor` survive canister upgrades automatically. There is no database layer, no serialization code, and no pre/post-upgrade hooks needed for most use cases.

**Async/await messaging.** Inter-canister calls use `async`/`await`, making sequential message flows read like synchronous code. The compiler and runtime handle the underlying callback mechanics.

**Strong typing.** Motoko has a sound type system with generics, variant types, pattern matching, and option types (`?T`) that prevent null-pointer errors at compile time.

## Get started

- [Hello, world!](./fundamentals/hello-world.md) — your first Motoko canister
- [Install](https://docs.internetcomputer.org/getting-started/) — set up your local development environment
- [Motoko core package](https://mops.one/core) — standard library reference
- [Example projects](https://github.com/dfinity/examples/tree/master/motoko)
