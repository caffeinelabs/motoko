---
title: "Classical orthogonal persistence"
description: "Classical orthogonal persistence is the legacy implementation of Motoko's orthogonal persistence."
sidebar:
  order: 3
---

Classical orthogonal persistence was the legacy implementation of Motoko's orthogonal persistence. It has been **removed**: `moc` no longer accepts `--legacy-persistence` (nor the classical-only GC flags `--copying-gc`, `--compacting-gc`, `--generational-gc`, `--rts-stack-pages`), and 32-bit (`wasm32`) RTS builds no longer exist. All canisters are now compiled with [enhanced orthogonal persistence](./enhanced.md).

Upon upgrade, the classical orthogonal persistence mechanism used to serialize all stable data to the stable memory and then deserialize it back to the main memory. This had several downsides:

* At maximum, 2 GiB of heap data could be persisted across upgrades. This is because of an implementation restriction. Note that in practice, the supported amount of stable data could be way lower.
* Shared immutable heap objects could be duplicated, leading to potential state explosion on upgrades.
* Deeply nested structures could lead to a call stack overflow.
* The serialization and deserialization was expensive and could hit ICP's instruction limits.
* There was no built-in stable compatibility check in the runtime system. If users ignored the `dfx` upgrade warning, data could be lost or an upgrade could fail.

:::danger
The above-mentioned issues could lead to a stuck canister that can no longer be upgraded.
Therefore, it was absolutely necessary to thoroughly test how much data an upgrade of your application can handle and then conservatively limit the data held by that canister.
Moreover, it was ideal to have a backup plan to rescue data even if upgrades fail, e.g. by controller-privileged data query calls. Another option was to [snapshot](https://docs.internetcomputer.org/guides/canister-management/snapshots) the canister before attempting the upgrade.
:::

These issues are solved by [enhanced orthogonal persistence](./enhanced.md).


:::note
Existing classical canisters are **not** orphaned: the runtime keeps reading all earlier classical stable-memory formats, and a classical canister transparently migrates to enhanced persistence on its next upgrade (this requires recompiling with enhanced-orthogonal-persistence code — the migration is irreversible).

Because `--legacy-persistence` is gone, `moc` can no longer *produce* classical canisters. Projects that still need a classical module must keep an older `moc` (e.g. 1.14.x).
:::

