---
title: "Comments"
description: "Motoko supports single-line, multi-line, and nested comments."
sidebar:
  order: 10
---

Motoko supports single-line, multi-line, and nested comments.

## Single line

Use `//` for comments that extend to the end of a line.

```motoko no-repl
// This is a single-line comment
```

Use `///` for function or module documentation (also known as "doc comments"). Module documentation can be exported into documentation files such as Markdown or HTML using [mo-doc](https://docs.internetcomputer.org/developer-tools/#mo-doc).

```motoko no-repl
/// Returns the sum of two integers.
func add(a : Int, b : Int) : Int {
  a + b
}
```

## Multi-line

Use `/* ... */` for block comments spanning multiple lines.

```motoko no-repl
/* This is a
    multi-line comment */
```

## Nested

Multi-line comments can be nested within each other.

```motoko no-repl
/* Outer comment
    /* Nested comment */
    End of outer comment */
```

## Resources

- [Comment style guide](../../style-guide.md#comments)

- [Generating Motoko documentation](https://docs.internetcomputer.org/developer-tools/#mo-doc)

