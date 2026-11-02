---
title: "Imports"
description: "In Motoko, related code modules are organized into packages."
sidebar:
  order: 2
---

In Motoko, related code modules are organized into packages. Modules can be imported either from named packages or from the local file system using relative paths. The compiler locates packages on the file system based on a command line argument specifying their location.

Imports should be placed at the top of the source file. They enable code reuse from external libraries or modules, helping to improve maintainability and organization. You can import from:

**1. Standard modules provided by the core package.**

```motoko no-repl
import Text "mo:core/Text";
import Nat "mo:core/Nat";
import Math "mo:core/Float";
```

The package `core` is Motoko's standard library.
This imports the `Text`, `Nat` and `Float` modules from package `core` under the local names `Text`, `Nat` and `Math`.

While not required, it's considered good practice to import a module using its package-defined name. This helps with consistency and readability across codebases.

**2. Packages installed via a package manager (such as Mops).**

```motoko no-repl
import Iter "mo:itertools";
```

The module `Iter` is imported from a third-party package `itertools`.

**3. Files within the current project.**

```motoko no-repl
import Utils "Utils";
```

**You can also import specific functions from a module:**

```motoko no-repl
import { compare } "mo:core/Nat";
```

**You can also import specific types from a module:**

```motoko no-repl
import { type Result; mapOk } "mo:core/Result";
```

Learn more about [modules and imports](./imports.md).
