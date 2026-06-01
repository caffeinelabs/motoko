---
title: "Pattern matching"
description: "Pattern matching in Motoko is a language feature that makes it easy to test and break down complex data structures."
sidebar:
  order: 8
---

Pattern matching in Motoko is a language feature that makes it easy to test and break down complex data structures. It is commonly used in `switch` expressions to extract and work with parts of a value.

Unlike building structured data, pattern matching lets you deconstruct that data and assign its parts to variables. The syntax often looks like how you construct the data, but it's used in places like function arguments, `case` clauses in `switch` expressions, and after `let` or `var` when declaring variables.

Motoko supports several types of patterns:

| Pattern type | Description | Example |
|-------------|-------------|---------|
| Wildcard (`_`) | Matches any value without binding it to a variable. | `switch (x) { case (_) { ... } }` |
| Literal | Matches specific constant values. | `switch (x) { case (0) { ... } case (1) { ... } }` |
| Option (`?T`) | Matches optional values. | `switch (opt) { case (?v) { ... } case (null) { ... } }` |
| Object | Matches object fields. | `switch (obj) { case ({field}) { ... } }` |
| Variant | Matches tagged union types. | `switch (variant) { case (#tag v) { ... } }` |
| Named  | Introduces identifiers into a new scope. | `age`, `x` |
| Tuple                   | Must have at least two components.   | `( component0, component1, …​ )` |
| Alternative (`or`-pattern) | Match multiple patterns. | `0 or 1`                        |
| Conjunctive (`and`-pattern) | Match both patterns; bindings combine. | `?x and s`                    |

## Concepts

### Enumerated types

Unlike traditional `enum` types in other languages, which define fixed sets of values with no attached data, Motoko uses **variant types**, a more flexible alternative. Variants can optionally carry payloads and support full pattern matching. For simple enumerations without payloads, Motoko offers a shorthand syntax, such as `type Weekday = { #mon; #tue; #wed; ... }`.

### Irrefutable and refutable patterns

Patterns are either **refutable** or **irrefutable**. A refutable pattern can fail to match (like literal patterns or specific variant tags). An irrefutable pattern always matches any value of its type. Examples include the wildcard `_`, simple variable names, or structured patterns (like records or tuples) made only from irrefutable parts.

### Singleton types

Some types only have a single possible value. These are called singleton types. Examples include the unit type `()` and variants with a single tag or singleton payload. Pattern matching on singleton types is simple because there’s only one possible match.

### Exhaustiveness checking

To prevent runtime errors when no `switch` case matches, the Motoko compiler performs exhaustiveness checking. It will warn you if any possible input is not handled and shows an example of a missing case. It also flags redundant patterns that will never be matched, helping you write safer, cleaner code.

## Using pattern matching

Consider the following function call:

```motoko no-repl
let name : Text = fullName({ first = "Motoko"; mid = "X"; last = "Ghost" });
```

This creates a record with three fields and passes it to the `fullName` function. The result is stored in a variable called `name`. This act of assigning a value to a variable using a pattern (`name : Text`) is a simple example of pattern matching.

Now, look at the function itself:

```motoko no-repl
func fullName({ first : Text; mid : Text; last : Text }) : Text {
  first # " " # mid # " " # last
};
```

Here, the input record is*destructured. Its fields are matched and their values are assigned to the variables `first`, `mid`, and `last`, which are then used in the function body. This example uses name punning, where the field name (e.g., `first`) is reused as the variable name. A more flexible pattern lets you give the value a different name, like this:

```motoko no-repl
mid = m : Text
```

In this case, the function matches the `mid` field but uses `m` as the variable name for its value.

## Wildcard (`_`)

The wildcard pattern `_` matches any value but does not bind to a variable. It is useful for handling cases where the specific value does not matter.

```motoko no-repl
func processNumber(n : Nat) : Text {
    switch (n) {
        case 0 { "Zero" };
        case 1 { "One" };
        case _ { "Other" };  // Matches any other number
    };
};
```

## Literal

Pattern matching supports literal patterns, which look like constant values (e.g., `#female`). These are especially useful in `switch` expressions because they let you match specific values, and if the match fails, the program moves on to the next `case`.

```motoko no-repl
switch ("Adrienne", #female) {
  case (name, #female) { name # " is a girl!" };
  case (name, #male) { name # " is a boy!" };
  case (name, _) { name # ", is a human!" };
}
```

In this example:

* The input is a pair: A name and a gender.
* The first `case` matches because `#female` equals `#female`, and `name` is bound to `"Adrienne"`.
* The result is: `"Adrienne is a girl!"`.

The last case uses the wildcard pattern `_`, which matches anything but doesn’t bind it to a variable. It’s a fallback in case no earlier patterns match.

## Option (`?T`)

Option patterns allow destructuring of `?T` values, matching either `null` or `?someValue`.

```motoko no-repl
import Nat "mo:core/Nat";

func getValue(opt : ?Nat) : Text {
    switch opt {
        // Parentheses required around `?n`
        case (?n) { "Value: " # Nat.toText(n) };
        case null { "No value" };
    };
};
```

## Object

Objects with named fields can be matched to extract specific properties.

```motoko no-repl
type Person = { name : Text; age : Nat };

func describePerson(person : Person) : Text {
    switch (person) {
        case ({ name = fullName; age }) { fullName # " is " # Nat.toText(age) # " years old." };
    };
};
```

## Variant

Motoko’s variant types represent a form of disjoint union, also known as a sum type. Each variant value contains exactly one discriminator (or tag) and an optional payload, which can differ depending on the discriminator. When pattern matching against a variant, the match only succeeds if the discriminators are the same. If they match, the associated payload is then made available for further matching or use within the pattern.

```motoko no-repl
type Status = { #ok; #error : Text };

func processStatus(status : Status) : Text {
    switch status {
        case (#ok) { "Success" };
        case (#error message) { "Error: " # message };
    };
};
```

## Alternative `or` patterns

An `or` pattern lets you match multiple patterns using the keyword `or`. Each sub-pattern is tried from left to right, and the match succeeds as soon as one of them works. However, all sub-patterns must bind the same variables, so the result is consistent no matter which one matches.

If none of the sub-patterns match, the whole `or` pattern fails.


