---
sidebar_position: 1
---

# Basic control flow

In Motoko, code normally executes sequentially, evaluating expressions and declarations in order.
However, certain constructs can alter the flow of control, such as exiting a block early, skipping iterations in a loop, returning a value from a function, or invoking another function.

# Control flow expressions

| Construct | Description |
|--------------|---------------|
| `return` | Exits a function and returns a value. |
| `if` | Executes a block if the condition is `true`. |
| `if/else` | Executes different blocks based on a condition. |
| `switch` | [Pattern matching](../8-pattern-matching.md) for variants, options, results, etc. |
| `let-else` | Destructure a pattern and handle the failure case inline. |
| `option block` | Evaluates an expression and wraps the result in an option type, allowing scoped handling of `null` values. |
| `label/break` | Allows exiting loops early. |
| `loop` | Iterates indefinitely |
| `loop ... while` | Iterates until some condition is false |
| `while` | Iterates while a condition is `true`. |
| `for` | Iterates over elements in a collection, terminating when no elements remain. |

## `return`

A `return` statement immediately exits a function or `async` block with a result. Unlike `break` or `continue`, which jump to a point within the same function (either to a label or to the innermost loop), `return` does not target a label. Instead, it exits the current function entirely, either returning control to the caller or, in asynchronous contexts, completing a future and resuming the caller that's awaiting the result.

Consider this function that computes the product of an array of integers.

```motoko no-repl
func product(numbers : [Int]) : Int {
  var prod : Int = 1;
  for (number in numbers.vals()) {
    prod *= number;
  };
  prod; // The implicit result of the block and function
}
```

This function doesn't require an explicit `return`. It just returns the result of its body, `prod`.

However, `prod` will remain `0` once it becomes `0` so you can save some work by using `return` to return from the function early, exiting both the loop and the function with result `0`.

```motoko no-repl
func product(numbers : [Int]) : Int {
  var prod : Int = 1;
  for (number in numbers.vals()) {
    prod *= number;
    if (prod == 0) return 0; // an early return can save work
  };
  prod; // The implicit result of the block and function
}
```

This also works with asynchronous functions that produce futures:

```motoko no-repl
func asyncProduct(numbers : [Int]) : async Int {
  var prod : Int = 1;
  for (number in numbers.vals()) {
    prod *= number;
    if (prod == 0) return 0; // an early return completes the future
  };
  prod; // The implicit result of the block and function
}
```

If the expected return type is `()` then you can just write `return` instead of `return ()`.

## `switch`

A `switch` expression matches a value against multiple cases and executes the block of code associated with the first matching case.

```motoko no-repl
import Nat "mo:core/Nat";

type HttpRequestStatus = {
  #ok: Nat;
  #err: Nat;
};

func checkStatus(r : HttpRequestStatus) : Text {
  switch (r) {
    case (#ok successCode) { "Success: " # Nat.toText(successCode) };
    case (#err errorCode ) { "Failure: " # Nat.toText(errorCode) };
  };
};
```

## `let-else`

The `let-else` construct allows conditional binding of the variables in a pattern, by attempting to match a value to the pattern. The `else` clause handles the case when the pattern is not a match. It is useful when working with `Result<T,E>` and optional values (`?T`), enabling concise error handling or early exits when the value is `null`.

Since the code following the `let` cannot execute without its matching bindings, the `else` clause must have type `None`, typically by diverting control using `return`, `throw`, `break` or `continue`.

```motoko no-repl
import Nat "mo:core/Nat";

type HttpRequestStatus = {
  #ok: Nat;
  #err: Nat;
};

func checkStatus(r : HttpRequestStatus) : Text {
  let #ok status = r else return "The request failed!";
  Nat.toText(status)
};
```

:::note
Unlike a `switch`, `let-else` discards any additional error information from non-matching cases, making it less suitable when detailed error handling is needed. The `(#err e)` case is dropped entirely; `e` cannot be inspected or logged.
:::

## Option block

These blocks represented as `do ? {...}` allow safe unwrapping of optional values using the postfix operator `!`, which short-circuits and exits the block with `null` if any value is `null`, simplifying code that handles multiple options. The result of the inner block, if any, is returned in an option.

A simple example uses an option block to concisely add optional number, return `null` when either is `null`.

```motoko no-repl
 // Returns the sum of optional values `n` and `m` or `null`, if either is `null`
func addOpt(n : ?Nat, m : ?Nat) : ?Nat {
  do ? {
    n! + m!
  }
};
let o1 = addOpt(?5, ?2);       // ?7
let o2 = addOpt(null, ?2);    // null
let o3 = addOpt(?5, null);    // null
let o4 = addOpt(null, null);  // null
```

Instead of having to switch on the options `n` and `m` in a verbose manner the use of the postfix operator `!` makes it easy to unwrap their values but exit the block with `null` when either is `null`.

A more interesting example of option blocks can be found at the end of the section on [switch](../5-control-flow/5-switch.md).

## `label` and `break`

A `label` assigns a name with an optional type to a block of code that executes like any other block.
The type on the label should indicate the type of the block and defaults to `()` when omitted.

When a labeled block runs, it evaluates the block to produce a result.
Labels don’t change how the block executes but enable early exits from the block using a `break` to that label.
If the type is not `()` those breaks must have an argument, to use as the result of the labelled expression.

Just as `return` exits a function early with a result, `break` exits its label early with a result.
Indeed, you can think of `return` as a `break` from the enclosing function.

```motoko no-repl
func product(numbers : [Int]) : Int {
  var prod : Int = 1;
  label l for (number in numbers.vals()) {
    prod *= number;
    if (prod == 0) break l;
  };
  prod; // The implicit result of the block and function
}
```

If the block produces a non-`()` result, as in this minor refactoring, the `break` should include a value:

```motoko no-repl
func product(numbers : [Int]) : Int {
  label result : Int {
    var prod : Int = 1;
    for (number in numbers.vals()) {
      prod *= number;
      if (prod == 0) break result 0;
    };
    prod
 }
}
```

Labels provide fine control over execution, allowing early exits and helping to structure complex logic.

## `loop`

A `loop` expression repeatedly executes a block of code (forever).

```motoko no-repl
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

var i = 0;
loop {
  Debug.print(Nat.toText(i));
  i += 1;
}
```

## `loop-while`

A `loop-while` expression repeatedly executes a block of code (at least once) until the while condition evaluates to `false`.

```motoko no-repl
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

var i = 0;
loop {
  Debug.print(Nat.toText(i));
  i += 1;
} while (i < 5)
```


## `while`

A `while` loop repeatedly executes a block of code as long as a specified condition evaluates to `true`.
If the condition is initially `false`, the block is never executed.

```motoko no-repl
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

var i = 0;
while (i < 5) {
  Debug.print(Nat.toText(i));
  i += 1;
}
```

## `for`

A `for` loop iterates over the elements of an iterator, and object of type `{ next: () -> ?T }`, executing a block of code for each element.

```motoko no-repl
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

let numbers = [0, 1, 2, 3, 4];
for (num in numbers.vals()) {
  Debug.print(Nat.toText(num));
}
```

It will run forever if the iterator's `next` method never returns `null`.

## `continue`

A `continue` expression skips the remainder of the current iteration in a loop and immediately proceeds to the next iteration. `continue` without a label continues the innermost loop. When a loop is labeled with a label `l`, then `continue l` continues the loop labeled `l`. This works within `while`, `for`, `loop`, or `loop-while` expressions.

For example, computing the product we can skip a multiplication when the number is `1`:

``` motoko no-repl
func product(numbers : [Int]) : Int {
  var prod : Int = 1;
  for (number in numbers.vals()) {
    if (number == 1) continue;
    prod *= number;
  };
  prod;
}
```

When you have nested loops and need to continue a specific outer loop, you can use a label:

``` motoko no-repl
func product(numbers : [Int]) : Int {
  var prod : Int = 1;
  label l for (number in numbers.vals()) {
    if (number == 1) continue l;
    prod *= number;
  };
  prod;
}
```

## Loop exits

You can always exit a `loop`, `loop-while`, `while` or `for` loop using `break`.
`break` without a label exits the innermost loop.
If a loop is labeled with a label `l`, then `break l` exits the loop labeled `l`.
You can also exit any loop in a function using `return` or (in an asynchronous function) `throw`.

## Function calls

A function call executes a function by passing arguments and receiving a result. In Motoko, function calls can be synchronous (executing immediately within the same [canister](https://internetcomputer.org/docs/building-apps/essentials/canisters)) or [asynchronous](../2-actors/1-actors-async.md#async--await) (message passing between canisters). Asynchronous calls use `async`/`await` and are essential for inter-canister communication.

```motoko no-repl
persistent actor {

  func product(numbers : [Int]) : Int {
    var prod : Int = 1;
    for (num in numbers.values()) {
      prod += num;
      if (prod == 0) return 0; // an early return can save work
    };
    prod;
  };

  public func asyncProduct(numbers : [Int]) : async Int {
    return product(numbers); // function call
  };

}
```

<!-- TODO: should we add a test method that await's asyncProduct -->

Execution begins in `asyncProduct()`, where the local function `product()` is invoked, transferring control to its logic. Inside `product()`, the numbers are processed one by one. If a zero is encountered, a `return` statement immediately exits the call to `product()` and returns 0.
Control then flows back to `asyncProduct()`, which just returns the result, completing the asynchronous call.

Function calls temporarily interrupt the normal sequential flow by shifting execution to a separate block of logic. Once the called function completes, control resumes at the point where the call was made, continuing with its result.



