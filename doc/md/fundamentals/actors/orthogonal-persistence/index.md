---
title: "Orthogonal persistence"
description: "How Motoko preserves the entire program state across canister upgrades, with no database layer or serialization code."
sidebar:
  order: 6
  label: "Orthogonal persistence"
  hidden: true
---

Orthogonal persistence is the mechanism by which Motoko preserves an actor's state across canister upgrades automatically — no database, no stable memory API, no serialization code. This section covers the classical and enhanced persistence models and the trade-offs between them.
