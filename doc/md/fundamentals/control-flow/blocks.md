---
title: "Block expressions"
description: "A block expression in Motoko is a sequence of declarations enclosed in { ..."
sidebar:
  order: 4
---

A block expression in Motoko is a sequence of declarations enclosed in `{ ... }`.
Since every expression is also a declaration, the sequence can include expressions.
Intermediate expressions, that produce values other than `()`, must be prefixed with `ignore`.

Blocks are used to group multiple operations, define local variables, and structure code for clarity.

Block expressions are typically used to define function bodies and the bodies of `async` expressions.
They can also be used as branches in conditional expressions and as the bodies of cases in switches and the blocks of `try-catch-finally` expressions.
Since blocks  enclose declarations, they define new scopes for locally defined variables and types.

The last declaration in a block, which might be an expression, determines the block’s result. If no meaningful final declaration is present, the block returns `()`.

## Block expressions in functions

Every function in Motoko contains a block expression that defines its body and behavior.

```motoko no-repl
// The function body is a block expression that returns the result of the last expression.
shared func add(x : Nat, y : Nat) : async Nat {
    let sum : Nat = x + y;
    sum;
};

// Example of using a block expression with conditional logic:
func classify(n : Int) : Text {
    if (n > 0) {
        "Positive"
    } else if (n < 0) {
        "Negative"
    } else {
        "Zero"
    } // The last evaluated branch determines the return value.
};
```

### `do` blocks

Blocks are not limited to functions; they can be used anywhere an expression is expected by prefixing the block with `do`:

```motoko no-repl
let result : Nat = do {
  let x : Nat = 10;
  let y : Nat = 5;
  x * y // This value is returned and assigned to `result`
};
```


The `do {}` expression in Motoko can be used to enter a new scope and make some local declarations before producing a value.


A `do {}` block can also just return `()` and be evaluated for its side effect:

```motoko no-repl
do {
    let x : Nat = 10;
    let y : Nat = 5;
    Debug.print("Adding " # debug_show(x) # " and " # debug_show(y));
    let sum : Nat = x + y;
    Debug.print("Result: " # debug_show(sum)); // unit `()` return type
};
```

