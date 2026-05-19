---
title: "Whitespace"
description: "Whitespace characters (spaces, tabs, newlines) are generally ignored in Motoko, but are essential for separating syntax components like keywords and identifi..."
sidebar:
  order: 11
---

Whitespace characters (spaces, tabs, newlines) are generally ignored in Motoko, but are essential for separating syntax components like keywords and identifiers. Proper use of whitespace enhances code readability.

### Incorrect use of whitespace

```motoko no-repl
persistent actor Counter{var x : Nat = 0; public func inc(): async Int{x+1; }};
```

### Proper whitespace usage

```motoko no-repl
persistent actor Counter {
  var x : Nat = 0;
  public func inc() : async Int {
    x + 1;
  };
};
```

## Resources

- [Motoko style guide](../../style-guide.md)

