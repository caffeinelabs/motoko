---
title: "Modules and imports"
description: "Motoko minimizes built-in types and operations, relying on a core package of modules to provide essential functionality."
sidebar:
  order: 7
---

Motoko minimizes built-in types and operations, relying on a core package of modules to provide essential functionality. This modular approach keeps the language simple.

The examples in this section show how to use the `module` and `import` keywords in different scenarios.

:::caution

The core package is actively maintained and updates may introduce breaking changes. Developers should review the latest Motoko migration guide when updating dependencies.

:::

## Importing from the core package

The Motoko core package includes common utilities for working with data structures, debugging, and other functionality. To import from the core package, use the `import` keyword, followed by the `mo:core/<name>` module path.

```motoko no-repl
import Debug "mo:core/Debug";

Debug.print("Hello, world!");
```

The `mo:` prefix identifies a Motoko module. The declaration does not include the `.mo` file extension.

You can also selectively import and rename a subset of named values and types from a module by using the object pattern syntax:

```motoko no-repl
import { type List; get; foldLeft = fold } "mo:core/List";
```

## Importing from another file

Projects may split code into multiple files for better organization, such as:

```
src/project_backend
 ├── main.mo   // Contains the main actor
 ├── types.mo  // Stores type definitions
 ├── utils.mo  // Contains helper functions
```


In this scenario, you might place all three files in the same directory and use a local import to make the functions available where they are needed.

For example, the `main.mo` contains the following lines to reference the modules in the same directory:

```motoko no-repl
import Types "types";
import Utils "utils";
```

These lines import modules from the local project instead of the Motoko library and don’t use the `mo:` prefix. In this example, both the `types.mo` and `utils.mo` files are in the same directory as the `main.mo` file.

## Importing from another package or directory

You can also import modules from other packages or from directories other than the local directory.

For example, the following lines import modules from a `redraw` package that is defined as a dependency:

```motoko no-repl
import Render "mo:redraw/Render";
import Mono5x5 "mo:redraw/glyph/Mono5x5";
```

You can define dependencies for a project using a package manager or in the project `dfx.json` configuration file.

In this example, the `Render` module is in the default location for source code in the `redraw` package and the `Mono5x5` module is in a `redraw` package subdirectory called `glyph`.

## Importing packages from a package manager

Modules can also be imported from other packages, such as those imported from a package manager.

Dependencies are managed using a package manager or defined in the project's `dfx.json` file. Motoko supports package managers like [Mops](https://mops.one/) to install third-party libraries.

### Configuring the package manager in `dfx.json`

```json
{
  "defaults": {
    "build": {
      "packtool": "mops sources"
    }
  }
}
```

### Installing a package with a package manager

With [Mops](https://mops.one/), add the package to your project's dependencies with `mops add`.

Then import the package into your Motoko code:

```motoko no-repl
import Vec "mo:vector";
```

## Naming imported modules

While the imported module name usually matches the file name, custom names can be used to avoid conflicts or simplify references.

```motoko no-repl
import List "mo:core/List";
import L "mo:core/List";
import PureList "mo:core/pure/List";
```

## Importing from another canister

Actors and their functions can be imported from other [canisters](https://docs.internetcomputer.org/concepts/canisters) using the `canister:` prefix.

```motoko no-repl
import BigMap "canister:BigMap";
import Connectd "canister:Connectd";
```

`BigMap` and `Connectd` are separate canisters defined in `dfx.json`. Canister functions are shared and may require `await` to call them.

Unlike a Motoko module, an imported canister:

- Can be implemented in any language that emits a Candid interface.
- Has its type derived from a `.did` file, not from Motoko itself.

:::danger
When importing from another canister, the canister must be listed as a dependency in the importing canister's `dfx.json`. These must both:

1. Be listed in the `dependencies` array of `my_canister`.
2. Have their own canister definitions specified elsewhere in the same file.

```json
{
  "canisters": {
    "my_canister": {
      "main": "src/my_canister/main.mo",
      "type": "motoko",
      "dependencies": ["BigMap", "Connectd"]
    }
  }
}
```

:::

## Importing actor classes

When imported, an [actor](./actors/actors-async.md) class provides a type definition describing the class interface and a function that returns an instance of the class.

For example, if you define the following actor class:

```motoko no-repl title="Counters.mo"
persistent actor class Counter(init : Nat) {
  var count = init;

  public func inc() : async () { count += 1 };

  public func read() : async Nat { count };

  public func bump() : async Nat {
    count += 1;
    count;
  };
};
```

It can be imported into another file:

```motoko no-repl
import Counters "Counters";
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

persistent actor CountToTen {
  public func countToTen() : async () {
    let counterActor = await Counters.Counter(1);
    while ((await counterActor.read()) < 10) {
      Debug.print(Nat.toText(await counterActor.read()));
      await counterActor.inc();
    };
  }
};
```

`Counters.Counter(1)` installs a new counter on the network. Installation is [asynchronous](./actors/actors-async.md#async--await), so the result is awaited.  If the actor class is not named, it will result in a bad import error because actor class imports cannot be anonymous.

## Importing `Blob` values

The `import` syntax can also be used with the `blob:file:` URI scheme to import raw `Blob` values:

```motoko no-repl
import pub = "blob:file:./keys/id_ed25519.pub";

actor {
    func checkSig(key : Blob, cyphertext : Text) { ... };

    public func verify(cyphertext : Text) : async () {
        checkSig(pub, cyphertext);
    };
};
```

This use case also caters for the import of externally-built Wasm `Blob`s for low-level installation and upgrade by means of the management canister.
