---
title: "Literals"
description: "Literals are constant expressions that require no further evaluation:"
sidebar:
  order: 6
---

Literals are constant expressions that require no further evaluation:

- **Integer literals**: (Numbers, hexadecimal), such as: `42`, `109231`, `0x2A`

- **Float literals**: (Floating point numbers), such as: `3.14`, `2.5e3`

- **Character literals**: (Unicode characters), such as: `'A'`, `'J'`, `'✮'`

- **Text literals**: `"Hello"`

- **Blob literals**: Byte sequences using the same syntax as `Text`, such as: `"Motoko" : Blob` (interpreted as [UTF-8](https://en.wikipedia.org/wiki/UTF-8) when converted)

You can use literals directly in expressions.

```motoko no-repl
100 + 50
```

## Resources

- [Literals](../../reference/language-manual.md#literals)

