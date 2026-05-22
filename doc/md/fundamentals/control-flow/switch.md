---
title: "Switch"
description: "A switch expression is a control flow construct that, given a value, selects a control flow path based on the pattern or shape of the value."
sidebar:
  order: 5
---

A `switch` expression is a control flow construct that, given a value, selects a control flow path based on the pattern or shape of the value.

A switch is constructed from an expression and a sequence of cases. Each case consists of a [pattern](../pattern-matching.md) guarding an expression or block that defines a possible branch of execution.

Switch evaluates its expression and based on its value, selects the first case whose pattern matches the value.
Any identifiers bound by the pattern are available in the selected branch, allowing you to access case-specific data in the branch.

Only one branch is ever executed in a switch, the first one that matches. If no case matches the value, execution traps.

Motoko will warn you if any of the cases are redundant and can never matched due to previous cases.

Motoko will also warn you if the cases don't cover all possible values, meaning that the switch could fail with a trap at runtime.

A `switch` is useful for replacing long `if-else` chains, improving both readability and structure. It lets you match specific values or patterns and handle each case individually.

Only the first case of a `switch` expression that matches will execute. The wildcard pattern `_` matches any value and should be placed last, because if `_` appears earlier, it will prevent later cases from being checked. Once a match is found, evaluation continues with the code of the matching case.

The simplest use of switch is to emulate an `if-else` expression:

```motoko no-repl
func toText(b : Bool) : Text {
   switch b {
      case true "true";
      case false "false";
   }
}
```

If you add a second case for `true`, Motoko issues a warning that it will never be matched and is unreachable or dead code:

```motoko no-repl
func toText(b : Bool) : Text {
   switch b {
      case true "true";
      case false "false";
      case true "dead code";
   }
}
```

If you forget the case for `false`, Motoko will also issue a warning that `false is not covered by any case`:

```motoko no-repl
func toText(b : Bool) : Text {
   switch b {
      case true "true";
   }
}
```

Motoko is able to issue these warnings for much more complicated patterns where it might be hard to see that you've made a mistake, helping you construct correct code and catch errors at compilation.
```motoko no-repl
func getDayOfWeek(day : Nat) : Text {
    switch day {
        case 1 "Monday";
        case 2 "Tuesday";
        case 3 "Wednesday";
        case 4 "Thursday";
        case 5 "Friday";
        case 6 "Saturday";
        case 7 "Sunday";
        case _ "Invalid day"; // Default case for numbers outside 1-7
    }
}
```

This example function takes a number (`day: Nat`) as input, representing a day of the week. The `switch` expression then compares this number against predefined cases. If a matching case is found, the corresponding day name is returned. If the input does not match any case, such as `0` or `8`, the default case (`case _`) prevents an error (`trap`) by returning `"Invalid day"`.

Above, the example `switch` expressions have only matched against simple constants like `true`, `false`, and `1`. You can also match against richer patterns such as tuples, options, variants, and objects. Each pattern can have patterns inside it.

Here's a simple example of matching against an option:

```motoko no-repl
func value<T>(option : ?T, default : T) : T {
   switch option {
      case null default;
      case (?value) value;
   }
}
```

The first case matches against the literal `null`, returning `default`. The second case matches against the option pattern `?value`, binding the option's contents to the variable `value` and returning `value`.

If either case is omitted, Motoko will warn that the switch does not cover the `null` or `?_` case.

Here's an example of a nested pattern:

```motoko no-repl
type List<T> = ?(T, List<T>);

func size<T>(list : List<T>) : Nat {
  switch list {
     case null { 0 };
     case (?(_, tail)) { 1 + size(tail) };
  }
}
```

This function returns the size of a list by using pattern matching to recurse on the shape of the list. Either the list is `null` or it is some tuple consisting of a value and another list, `tail`.

Notice that the second pattern consists of an option pattern containing a tuple pattern with a wildcard `_` and an identifier pattern, `tail`,  inside it.

In this example, the branches are enclosed in blocks to demonstrate that it is supported. This can be useful if you need to add some local declarations to a branch.

A more complex example can be found below:

```motoko no-repl
type Exp = {#Lit : Nat; #Div : (Exp, Exp); #If : (Exp, Exp, Exp)};
func eval(e : Exp) : ? Nat {
  switch e {
    case (#Lit n) { ?n };
    case (#Div (e1, e2)) {
      switch (eval e1, eval e2) {
        case (?v1, ?v2) {
          if (v2 == 0) null
          else ?(v1 / v2)
        };
        case _ { null }
      }
    };
    case (#If (e1, e2, e3)) {
      switch (eval e1) {
        case (?0) { eval e2 };
        case (?_) { eval e3 };
        case _ { null }
      };
    };
  };
};

let expr : Exp = #If(
  #Div(#Lit 10, #Lit 2),   // 10 / 2 = 5 (non-zero, so evaluate e3)
  #Lit 0,                  // e2 (ignored because e1 ≠ 0)
  #Div(#Lit 6, #Lit 3)     // e3 → 6 / 3 = 2
);

eval(expr);
```

Using option blocks this code can be rewritten to reduce the need for nested switches, if that's preferred:

```motoko no-repl
type Exp = {#Lit : Nat; #Div : (Exp, Exp); #If : (Exp, Exp, Exp)};
func eval(e : Exp) : ? Nat {
  do ? {
    switch e {
      case (#Lit n) { n };
      case (#Div (e1, e2)) {
        let v1 = eval e1 !;  // If eval e1 returns null, exit with null
        let v2 = eval e2 !;  // If eval e2 returns null, exit with null
        if (v2 == 0)
          null !  // Explicitly exit with null for division by zero
        else v1 / v2
      };
      case (#If (e1, e2, e3)) {
        if (eval e1 ! == 0)  // Unwrap and check if zero
          eval e2 !  // Return result of e2 (or null if it's null)
        else
          eval e3 !  // Return result of e3 (or null if it's null)
      };
    };
  };
};

let expr : Exp = #If(
  #Div(#Lit 10, #Lit 2),   // 10 / 2 = 5 (non-zero, so evaluate e3)
  #Lit 0,                  // e2 (ignored because e1 ≠ 0)
  #Div(#Lit 6, #Lit 3)     // e3 → 6 / 3 = 2
);

eval(expr);
```
