---
title: "Expression declarations"
description: "An expression declaration is a declaration that consists of a single expression."
sidebar:
  order: 6
---

An expression declaration is a declaration that consists of a single expression. The expression is evaluated solely for its value and side effects. Unlike other declarations, it does not declare any new names.

When used as an intermediate declaration within a block, a declaration expression must have type `()`.
If it produces a value of another type, it must be preceded by the `ignore` keyword to discard the value and ensure the expression evaluates to `()`.

However, when used as the final declaration in a block, the declaration expression determines the type and value of the entire block. In this case, it may be of any valid type, not just `()`.
For example:

```motoko no-repl
var n : Nat  = 0;

func regUser(name : Text) : Text {
  ignore bumpUsers(); // Returns Nat, but result is not needed
  let greeting = "Welcome, " # name # "!";
  greeting
};

func bumpUsers() : Nat {
   n += 1;
   n
};
```

In Motoko, expressions of type `()` play the role of statements in other languages.

## Basic usage

Expression declarations are commonly used for functions or operations that produce side effects, such as printing or modifying [state](../actors/state.md).

```motoko no-repl
Debug.print("Hello, Motoko!");
```

In this example, the string `"Hello, Motoko!"` is printed, but the expression is not assigned to any variable.

## Expression declarations in a sequence

If an expression appears inside a sequence of declarations but is not the last declaration, it must return `()` (the unit type).

```motoko no-repl
let x = 10;
Debug.print("Processing..."); // Expression declaration with side effects
let y = x * 2;                // Valid, since Debug.print() returns ()
```

In this example, `Debug.print()` is used for its side effect. Because it returns `()`, it can safely appear before `let y = x * 2;` in the sequence.

In comparison, below is an invalid expression declaration:

```motoko no-repl
let x = 10;
x * 2;  // This expression produces a non-`()` value but is not named or ignored.
let y = 5;
```

The expression `x * 2;` returns a value of type `Nat`, but since it is not assigned to a variable and is not the last declaration, this is invalid. Declaration expressions in intermediate positions must return `()` or be marked `ignore`.

## Anonymous functions

Motoko supports anonymous functions as a type of expression.

```motoko no-repl
func applyFunction(f :  Int -> Int, value : Int) : Int { f(value) };
applyFunction( func (x : Int) : Int { x * 2 } , 2);
```

In this example, the first argument to `applyFunction` is the anonymous function `func (x : Int) : Int { x * 2 }`.
This is just an anonymous version of the function named `double` above.

The compiler can infer the argument and result types of anonymous functions, when the types are determined from the context, so you can even just write:

```motoko no-repl
applyFunction( func x { x * 2 } , 2);
```
