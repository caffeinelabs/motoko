---
title: "Classical orthogonal persistence"
description: "Classical orthogonal persistence is the legacy implementation of Motoko's orthogonal persistence."
sidebar:
  order: 3
---

Classical orthogonal persistence is the legacy implementation of Motoko's orthogonal persistence. Classical persistence is deprecated in favor of enhanced orthogonal persistence.

Upon upgrade, the classical orthogonal persistence mechanism serializes all stable data to the stable memory and then deserializes it back to the main memory. This has several downsides:

* At maximum, 2 GiB of heap data can be persisted across upgrades. This is because of an implementation restriction. Note that in practice, the supported amount of stable data can be way lower.
* Shared immutable heap objects can duplicated, leading to potential state explosion on upgrades.
* Deeply nested structures can lead to a call stack overflow.
* The serialization and deserialization is expensive and can hit ICP's instruction limits.
* There is no built-in stable compatibility check in the runtime system. If users ignore the `dfx` upgrade warning, data may be lost or an upgrade fails.

:::danger
The above-mentioned issues can lead to a stuck canister that can no longer be upgraded.
Therefore, it is absolutely necessary to thoroughly test how much data an upgrade of your application can handle and then conservatively limit the data held by that canister.
Moreover, it is ideal to have a backup plan to rescue data even if upgrades fail, e.g. by controller-privileged data query calls. Another option is to [snapshot](https://docs.internetcomputer.org/guides/canister-management/snapshots) the canister before attempting the upgrade.
:::

These issues are solved by [enhanced orthogonal persistence](./enhanced.md).


:::note
Classical orthogonal persistence was previously the default compilation mode for Motoko code. Going forward, the default compilation mode is enhanced orthogonal persistence,
previously available only with `moc` compiler flag `--enhanced-orthogonal-persistence`.

Users unwilling or unable to migrate their code can re-enable support for classical orthogonal persistence using the new compiler flag `--legacy-persistence`.

To re-activate classical orthogonal persistence under `dfx`, the following command-line argument needs to be specified in `dfx.json`:

```
...
    "type" : "motoko"
    ...
    "args" : "--legacy-persistence"
...
```
:::

