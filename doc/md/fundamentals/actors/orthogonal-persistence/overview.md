---
title: "What is orthogonal persistence?"
description: "Orthogonal persistence is the ability to for a program to automatically preserve its state across transactions and canister upgrades without requiring manual..."
sidebar:
  order: 1
---

Orthogonal persistence is the ability to for a program to automatically preserve its state across transactions and canister upgrades without requiring manual intervention. This means that data persists seamlessly, without the need for a database, stable memory APIs, or specialized stable data structures.

Although Motoko’s persistence model is complex under the hood, it’s designed to be both safe and efficient. Because actors are persistent by default, developers get persistence for free: every actor field survives upgrades, with no keyword, annotation, or special structure required. This abstraction significantly reduces the risk of data loss or corruption during upgrades.

In contrast, other canister development languages like Rust require explicit handling of persistence. Developers must manually manage stable memory and use specialized data structures to ensure data survives upgrades. These languages lack orthogonal persistence, and may rearrange memory unpredictably during recompilation or runtime, making safe persistence more error-prone and labor-intensive.

Motoko's orthogonal persistence is implemented by [enhanced orthogonal persistence](./enhanced.md). It provides very fast upgrades, scaling independently of the heap size. This is realized by retaining the entire Wasm main memory on an upgrade and simply performing a type-driven upgrade safety check. By using 64-bit address space, it is designed to scale beyond 4 GiB and in the future, offer the same capacity like stable memory.
