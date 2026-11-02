---
title: "Printing values"
description: "Motoko uses Debug.print to output text to the terminal or a canister's log depending on execution context."
sidebar:
  order: 3
---

Motoko uses `Debug.print` to output text to the terminal or a canister's log depending on execution context.
It takes a [`Text`](https://mops.one/core/docs/Text) value and returns `()`.
`()` is the empty tuple and represents a token or trivial return value.

```motoko no-repl
import Debug "mo:core/Debug";

Debug.print("Hello, world!");
```

For debugging purposes, `debug_show` converts most Motoko types into [`Text`](https://mops.one/core/docs/Text). The operator handles most types well, but may not work with cyclic data structures or types containing functions or type parameters.

```motoko no-repl
import Debug "mo:core/Debug";

Debug.print(debug_show {life = 42} ); // "{life = 42}"
```

Functions like `Debug.print("Hello, World!")` are considered **impure functions** because they cause a side effect by printing to the console or log.

In contrast, [**pure functions**](../types/function-types.md#pure-functions) return values that do not modify output or have other side effects like sending messages.  For example `Nat.toText(42)` is pure because it always returns `"42"` with no other effect.


## Resources

- [Debug](https://mops.one/core/docs/Debug)

