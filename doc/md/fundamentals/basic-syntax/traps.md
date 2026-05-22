---
title: "Assertions"
description: "An assertion checks a condition at runtime and traps if it fails."
sidebar:
  order: 12
---

An assertion checks a condition at runtime and traps if it fails.

```motoko no-repl
let n = 10;
assert n % 2 == 1; // Traps
```

```motoko no-repl
let n = 10;
assert n % 2 == 0; // Succeeds
```

Assertions help catch logic errors early, but should not be used for regular [error handling](../error-handling.md).

