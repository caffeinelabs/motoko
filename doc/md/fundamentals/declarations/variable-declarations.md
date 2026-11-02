---
title: "Variable declarations"
description: "In Motoko, variables are declared using:"
sidebar:
  order: 1
---

In Motoko, variables are declared using:

- `let` for immutable bindings: Once assigned, the value cannot be changed.

- `var` for mutable bindings: The value can be updated later.

## Immutable variables

The `let` keyword is used to bind a name to the result of an expression, creating an immutable variable. In a `let` declaration, the expression on the right-hand side is evaluated once, and its resulting value is associated with the given name. This value cannot be changed after the initial assignment. The Motoko compiler enforces this immutability by generating a compile-time error if there is any attempt to modify the variable later in the code. 

In concurrent code, immutable declarations should be preferred over mutable declarations, since they prevent concurrent modification.

```motoko no-repl
let x = 10;
x := 20; // Error: Cannot assign to immutable variable
```

The left hand side of a `let` can be also be a more general pattern, naming the components of a value by matching its structure:

For example, the declaration:

```motoko no-repl
let (fst, snd) = (1, 2); 
```

uses the pattern `(fst, snd)` to name the components of the pair `(1,2)`. The value of `fst` is `1` and the value of `snd` is `2`. Both `fst` and `snd` are immutable.

## Mutable variables

A `var` declaration in Motoko defines a mutable variable whose value can be updated after it is initially assigned. 
Unlike `let`, which creates an immutable binding, `var` allows reassignment using the `:=` operator. This means that the variable can hold different values over time, making it suitable for scenarios where state changes are required.

```motoko no-repl
var y = 10;
y := 20; // Allowed, updates the value of y
```

Unlike, `let` declarations, `var` declarations do not support pattern matching. 
For example, the following is a syntax error:

```motoko no-repl
var (a, b) = (1, 2); // Not supported
```

### Compound assignment operations

The assignment operation `:=` is general and works for all types.

Motoko provides special assignment operators that combine assignment with a binary operation. These compound operators update a variable by applying the operation between its current value and a given operand.

For example, numbers permit a combination of assignment and addition:

```motoko no-repl
var count = 2;
count += 40;
```

After the second line, the variable `count` holds `42`.

Motoko includes other compound assignments as well, such as `#=`:

```motoko no-repl
var text = "Motoko";
text #= " Ghost"
```

As with `+=`, this combined form avoids repeating the assigned variable’s name on the right hand side of the special assignment operator `#=`.
