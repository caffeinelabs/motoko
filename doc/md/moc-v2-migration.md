---
title: "Migrating from moc 1 to moc 2"
description: "What changes when you move a project from moc 1.16 to moc 2.0, and how to fix each breaking change."
sidebar:
  order: 2
  label: "Migrating to moc 2"
---

This guide is for projects on `moc` 1.16.x whose canisters already use enhanced orthogonal persistence (EOP), the 1.16 default. It lists every breaking change in `moc` 2.0 with the message you will see and the fix.

Most projects need only a few changes: removing flags that no longer exist, fixing code that the former warnings now reject, and optionally deleting the now-redundant `persistent` and `stable` keywords.

## Upgrade checklist

1. Build cleanly with the latest 1.16.x first. The `.vals()` (M0269) and `preupgrade`/`postupgrade` (M0270) deprecation warnings are the same in 2.0, so you can fix them before switching.
2. Bump the toolchain in `mops.toml`:

   ```toml
   [toolchain]
   moc = "2.0.0"
   ```

3. Remove the [removed flags](#removed-flags) from `[moc].args`, `[canisters.<name>].args` and your build scripts. `moc` rejects each one with `unknown option`.
4. Run `mops check` (or `mops build`) and fix what it reports, in this order:
   1. syntax errors: [`??`](#-is-whitespace-sensitive), [glued branches](#if-while-for-and-switch-heads), [`flexible`](#flexible-is-removed);
   2. [errors that used to be warnings](#warnings-that-are-now-errors);
   3. library and module errors: [M0142](#libraries-must-be-modules-m0142), [M0193](#actor-class-return-type-m0193);
   4. persistence errors such as M0131, which only appear if you used [`--legacy-actors`](#actors-are-persistent-by-default).
5. Check upgrade compatibility against the deployed version (`mops check-stable`, or `moc --stable-compatible old.most new.most`) before you upgrade a live canister.

## Summary

| Change | What you see | Fix |
|---|---|---|
| [Actors are persistent by default](#actors-are-persistent-by-default) | warnings M0217, M0218 | delete `persistent` and `stable`; mark reset-on-upgrade fields `transient` |
| [`flexible` removed](#flexible-is-removed) | `syntax error [M0001]` | write `transient` |
| [`preupgrade`/`postupgrade` deprecated](#preupgrade-and-postupgrade) | warning M0270 | a migration function |
| [`??` whitespace and right-hand side](#-is-whitespace-sensitive) | `syntax error [M0001]`, M0273 | `a ?? b`; `a ?? do { ... }` for a block |
| [Glued `if` branches](#if-while-for-and-switch-heads) | M0275 | put a space before the branch |
| [`.vals()` deprecated](#vals-is-deprecated) | warning M0269 | `.values()` |
| [Record update copies `var` fields](#record-update-copies-var-fields) | nothing (was error M0179) | none, unless you relied on `--experimental-field-aliasing` |
| [Warnings that are now errors](#warnings-that-are-now-errors) | M0145, M0222, M0210, M0212, M0215, M0128, M0242, M0005 | per code, below |
| [Inferred `Any`/`None`](#inferred-any-or-none) | M0074, M0081, M0101, M0166, M0167 | fix the code, or annotate `Any` |
| [Bare-declaration libraries](#libraries-must-be-modules-m0142) | error M0142 | wrap in `module { ... }` |
| [Actor class return type](#actor-class-return-type-m0193) | error M0193 | `: async actor { ... }` |
| [Removed primitives](#removed-primitives-and-experimentalstablememory) | M0072 in `ExperimentalStableMemory` | use `Region` |
| [Read-only primitives drop `<system>`](#read-only-primitives-no-longer-take-system) | warning M0196 | delete `<system>` |
| [Removed flags](#removed-flags) | `unknown option` | see the table |
| [`moc --check` per file](#moc---check-checks-each-file-on-its-own) | M0057 unbound variable; `-r expects exactly one source file` | use imports |
| [New default warnings](#new-default-warnings) | M0217, M0236 | fix them, or `-A` them if you build with `-Werror` |
| [`moc.js` API](#mocjs) | `Invalid_argument` | `Motoko.run([], ...)`; `gcFlags` `"force"`/`"scheduling"` only |
| [Release artifacts](#release-artifacts) | no Intel-Mac or `base` tarball | on Intel Macs build from source, stay on moc 1, or use `moc.js` |

## Persistence

### Actors are persistent by default

In 2.0 every `actor` and `actor class` is persistent: its `let` and `var` fields keep their values across upgrades unless declared `transient`. The `persistent` keyword and the `stable` modifier are redundant, and `moc` reports them as warnings M0217 and M0218. Both still compile, and deleting them leaves the stable signature unchanged.

```motoko no-repl
// moc 1
persistent actor {
  stable var count : Nat = 0;
  transient var hits : Nat = 0;
};
```

```motoko no-repl
// moc 2
actor {
  var count : Nat = 0;
  transient var hits : Nat = 0;
};
```

With moc 1.16 defaults, an actor without `persistent` was already an error (M0219 on each unmarked field, M0220 on the actor), so your code says `persistent actor` and compiles under 2.0 without changes.

If you built with `--legacy-actors`, unmarked fields used to be transient and are now persistent. Mark every field that should reset on upgrade `transient`. A field whose type cannot be persisted, such as a function or an object with methods, is now error M0131 until you mark it:

```motoko no-repl
actor {
  transient let log = func (t : Text) { Debug.print(t) };
  transient var cache : [Nat] = [];
};
```

A field that was transient and becomes persistent needs no migration. On the first upgrade it is new to the stable signature, so it runs its initializer, and from then on it keeps its value.

See [Data persistence](fundamentals/actors/data-persistence.md).

### `flexible` is removed

`flexible` was an alias of `transient`. It is now an ordinary identifier, so `flexible var x = 0` is a syntax error. Write `transient var x = 0`.

### `preupgrade` and `postupgrade`

`system func preupgrade` and `system func postupgrade` still work in 2.0, with deprecation warning M0270. Replace them with a migration function, which cannot leave a canister stuck the way a trapping `preupgrade` can.

The usual reason for these hooks is a data structure that could not be persisted, such as `mo:base/HashMap`, copied into a stable array on the way out and rebuilt on the way in:

```motoko no-repl
// moc 1
import HashMap "mo:base/HashMap";
import Iter "mo:base/Iter";
import Text "mo:base/Text";

persistent actor {
  var entries : [(Text, Nat)] = [];
  transient let users = HashMap.HashMap<Text, Nat>(16, Text.equal, Text.hash);

  public func set(k : Text, v : Nat) : async () { users.put(k, v) };

  system func preupgrade() {
    entries := Iter.toArray(users.entries());
  };

  system func postupgrade() {
    for ((k, v) in entries.vals()) { users.put(k, v) };
    entries := [];
  };
};
```

The data structures in `mo:core` are stable, so the map can be an ordinary persistent field. A one-time migration function moves the existing entries into it:

```motoko no-repl
// moc 2
import Map "mo:core/Map";
import Text "mo:core/Text";

(with migration = func(old : { var entries : [(Text, Nat)] }) : { users : Map.Map<Text, Nat> } {
  { users = Map.fromIter(old.entries.values(), Text.compare) }
})
actor {
  let users : Map.Map<Text, Nat> = Map.empty();

  public func set(k : Text, v : Nat) : async () { users.add(k, v) };
};
```

The upgrade that installs this version still runs the old version's `preupgrade`, so `entries` holds the data when the migration runs. `moc` reports the dropped field as warning M0207, which is intended here. Once this version is deployed, delete the `(with migration = ...)` clause: later versions no longer have an `entries` field to consume.

If you use the [enhanced migration chain](fundamentals/actors/enhanced-multi-migration.md) (`--enhanced-migration`), put the same function in the next file of `migrations/` as `public func migration`.

Code that only initializes something in `postupgrade` can move into the actor body, which runs on every install and upgrade. See [Explicit migration using a migration function](fundamentals/actors/compatibility.md#explicit-migration-using-a-migration-function).

### Canisters still on classical persistence

`moc` 2 can no longer produce classical-persistence canisters, but it can still upgrade one. Compile that one-time upgrade with `--enhanced-orthogonal-persistence` and without `--enhanced-migration`, as described in [Migration path](fundamentals/actors/orthogonal-persistence/enhanced.md#migration-path). Later upgrades need neither flag. If your canisters are already on EOP, `--enhanced-orthogonal-persistence` is accepted and has no effect, so you can remove it.

## Syntax

### `??` is whitespace-sensitive

The null-coalescing operator must be followed by whitespace, as `<` and `>` already are. `??x` with no space is two option introductions, `?(?x)`.

```motoko no-repl
let n = o ??0;   // moc 1: 0 if o is null. moc 2: syntax error
let n = o ?? 0;  // both
```

The right-hand side of `??` is now an expression, so `{` there opens a record literal. A block needs `do`:

```motoko no-repl
let r = o ?? { x = 1 };                    // record literal, new in moc 2
let n = o ?? do { let k = f(); k + 1 };    // block: was `o ?? { ... }`, now M0273 without `do`
```

### `if`, `while`, `for` and `switch` heads

A condition, scrutinee or `for` collection may be any expression without parentheses. The branches or body are then blocks:

```motoko no-repl
if f(x) { a } else { b };
while n > 0 { n -= 1 };
for x in xs.values() { total += x };
switch p.x {
  case 0 { "zero" }
  case _ { "other" }
};
```

The moc 1 forms, such as `if (c) a else b`, `for (x in xs) ...` and `switch (e) { ... }`, still work. Two things break:

- **A bare branch glued to the condition.** A `(`, `[` or prefix operator directly after a name or a parenthesized condition now continues the condition. Put a space before the branch:

  ```motoko no-repl
  if c[0] else [1];     // moc 2: `c[0]` is the condition, M0275
  if c [0] else [1];    // branch `[0]`

  if (c)-1 else 1;      // moc 2: `(c)-1` is the condition, M0275
  if (c) -1 else 1;     // branch `-1`
  ```

- **Bare branches after a compound condition** (M0275). When the condition is more than a name or a parenthesized expression, the branches must be blocks. Write `if f(x) { a } else { b }`, or keep `if (f(x)) a else b`.

A record literal as a head needs parentheses, `switch ({ x = 0 }) { ... }` (M0272). The [style guide](reference/style-guide.md#parentheses) has the spacing rules.

### Lighter `switch` cases

These forms are new and optional; code in the moc 1 style still compiles.

- The `;` between cases can be dropped.
- A case pattern that is a literal, `null`, `?p`, `#tag`, `#tag(p)` or `_` needs no parentheses, and can be combined with `or`, `and` and `: T`.
- A variant payload always has its own parentheses: `case #node(n)`, never `case #node n`.

```motoko no-repl
switch t {
  case #leaf { 0 }
  case #node(n) { n }
};

switch n {
  case -1 { "negative" }
  case 0 or 1 { "small" }
  case _ { "large" }
};
```

A case body is a block, so a record literal in it must be nested: `case null { { x = 0 } }`. Writing `case null { x = 0 }` is M0272.

### `.vals()` is deprecated

The built-in `.vals()` on arrays and `Blob` is deprecated with warning M0269. Use `.values()`:

```motoko no-repl
for x in xs.values() { total += x };
```

### Record update copies `var` fields

`{ base with ... }` now copies the base's `var` fields into new cells, like the equivalent record literal. In moc 1 this was error M0179, unless you passed `--experimental-field-aliasing`, which made the copy share the base's cells.

```motoko no-repl
let base = { var count = 0; name = "a" };
let copy = { base with name = "b" };
copy.count += 1;   // base.count is still 0
```

If you relied on aliasing, keep the mutable state in a shared object and reference it from both records.

## Warnings that are now errors

These diagnostics flag code that traps, or silently does something other than what it says. They are errors by default in 2.0. Each can be downgraded back to a warning with `-W <code>`, for example `-W=M0145,M0215` in `[moc].args`. Treat this as a stopgap while you fix the code.

| Code | Flags | Before | After |
|---|---|---|---|
| M0145 | a pattern that does not cover every value, in `switch`, `let`, `catch`, `for` and function parameters | `let #ok(f) = r;` | `let #ok(f) = r else { return 0 };`, or add `case _ { ... }` |
| M0222 | `ignore` of an `async*` value, which never runs | `ignore bump();` | `await* bump();` |
| M0210 | a parenthetical on an `await*` call, where it has no effect | `await* (with cycles = n) pay();` | attach cycles to an `async` call: `await (with cycles = n) pay();` |
| M0212 | an unknown parenthetical attribute | `(with cycle = n)` | `(with cycles = n)` |
| M0215 | a record field the expected type drops, for example a typo in an update | `{ u with emial = e }` | `{ u with email = e }` |
| M0128 | a function named like a system method, but not declared `system` | `func heartbeat() : async () { ... }` | `system func heartbeat() : async () { ... }`, or rename it |
| M0242 | a `public func` without a return type, which is implicitly oneway | `public func log(t : Text) { ... }` | `public func log(t : Text) : () { ... }`, or `: async ()` |
| M0005 | an import path whose letter case differs from the file name | `import L "lib";` for `Lib.mo` | `import L "Lib";` |

### Inferred `Any` or `None`

When the type `moc` infers collapses to `Any` or `None`, the value is useless and the code is almost always a mistake. Joins that leave a smaller but useful type, such as two records reduced to their common fields, are unaffected.

| Code | Before | After |
|---|---|---|
| M0074 | `let xs = [1, "two"];` | make the elements agree, or annotate `let xs : [Any] = [1, "two"];` |
| M0081 | `if b { 1 } else { "one" }` | make the branches agree, or annotate `: Any` |
| M0101 | `switch o { case null { "none" } case ?n { n } }` | `case ?n { debug_show n }` |
| M0166 | `type U = Nat and Text;` (`None`) | write the intended type |
| M0167 | `type V = Nat or Text;` (`Any`) | write `Any`, or the intended type |

## Libraries and modules

### Libraries must be modules (M0142)

An imported file that is a bare sequence of declarations is now error M0142, and `-W M0142` no longer works. Wrap the declarations in `module { ... }` and mark the exported ones `public`:

```motoko no-repl
// Lib.mo
module {
  public func greet() : Text { "hi" };
};
```

### Actor class return type (M0193)

An actor class whose declared return type is not `async` is now error M0193 (it was warning M0135):

```motoko no-repl
actor class C() : actor {} { };        // M0193
actor class C() : async actor {} { };
```

### Implicits and dot notation search nested modules

Implicit arguments and contextual dot (`e.f(...)`) are also resolved from modules nested inside the modules in scope, up to a depth of 8, so importing a facade that re-exports its package's modules, such as `public let Map = _Map;`, is enough for both. This is not a breaking change: nested modules are only searched when no module in scope has a matching direct field, so every call that resolved in moc 1 resolves to the same function. Calls that moc 1 rejected may now resolve, or be ambiguous between two nested modules (M0224, M0231); the [language manual](reference/language-manual.md#resolution-of-dotted-calls-and-implicit-arguments) has the rules.

### Removed primitives and `ExperimentalStableMemory`

- The `stableMemory*` primitives are gone, so importing `mo:base/ExperimentalStableMemory` fails to type-check. Use `Region` from `mo:core` (or `mo:base`) instead. The rest of `base` still compiles; for new code use [`core`](base-core-migration.md).
- `Prim.createActor` is removed. Use actor classes, or the management canister's `create_canister` and `install_code`.
- Canisters no longer export the `__motoko_stable_var_info` query, which always trapped under EOP.

### Read-only primitives no longer take `<system>`

`Prim.getSelfPrincipal`, `Prim.envVarNames`, `Prim.envVar`, `Prim.callerInfoSigner`, `Prim.callerInfoData`, `Prim.getCandidLimits` and `Prim.getCandidTypeLimits` no longer need the `system` capability, so `query` methods can call them. An explicit `<system>` on them is warning M0196; delete it:

```motoko no-repl
public query func me() : async Principal { Prim.getSelfPrincipal() };
```

## Compiler and tooling

### Removed flags

`moc` rejects these flags with `unknown option`. Remove them.

| Flag | Replacement |
|---|---|
| `--legacy-persistence` | none: `moc` 2 cannot build classical-persistence canisters. Keep an older `moc` for those. |
| `--default-persistent-actors`, `--require-persistent-actors` | no longer needed: actors are persistent by default |
| `--legacy-actors` | mark fields `transient` explicitly |
| `--copying-gc`, `--compacting-gc`, `--generational-gc` | none: the incremental GC is the only GC |
| `--incremental-gc` | no longer needed: always on |
| `--rts-stack-pages`, `--skip-gc-deprecation-warning`, `--experimental-rtti` | no longer needed |
| `--experimental-stable-memory` | use `Region` |
| `--generate-view-queries` | write `public query func` getters for the fields clients read. The generated `__<field>` queries disappear from the canister's interface on upgrade. |
| `--experimental-multi-value`, `--no-experimental-multi-value` | no longer needed |
| `--experimental-field-aliasing` | none: [record update copies `var` fields](#record-update-copies-var-fields) |
| `--trap-on-call-error` | none: a failed call throws an `Error`; handle it with `try`/`catch` |
| `-no-system-api` | `-wasi-system-api` to run outside the Internet Computer, e.g. in `wasmtime` |
| `-ref-system-api` | no longer needed: it selected the default Internet Computer system API |
| `--print-source-on-error`, `-no-link`, `--profile`, `--profile-file`, `--profile-line-prefix`, `--profile-field` | none |

`--enhanced-orthogonal-persistence` stays, for [classical canisters](#canisters-still-on-classical-persistence).

### `moc --check` checks each file on its own

`moc --check a.mo b.mo` checks each file in a scope that holds only its own imports. In moc 1 the files were concatenated into one program, so `b.mo` could use a declaration from `a.mo` without importing it; that is now M0057 (unbound variable). Compiling (`-c`, `--idl`) and running (`-r`) take exactly one main file, and the REPL (`-i`) preloads at most one.

Move the shared declarations into a module and import it:

```motoko no-repl
// Helper.mo
module {
  public func helper() : Nat { 1 };
};
```

```motoko no-repl
// main.mo
import Helper "Helper";

let n = Helper.helper();
```

### `-g`

`-g` now emits only the DWARF line table (`.debug_line` and `.debug_line_str`). The `.debug_abbrev`, `.debug_addr` and `.debug_rnglists` sections are gone; they had no content without the `.debug_info` section, which `moc` never emitted.

### New default warnings

Two warnings are new by default. They matter mainly if you build with `-Werror`:

- M0217: redundant `persistent` keyword. Delete it.
- M0236: a call that could use dot notation, such as `Map.size(map)` for `map.size()`. Apply the suggestion (`mops check --fix`), or silence it with `-A M0236`.

## `moc.js`

- `Motoko.run` no longer preloads files. Its first argument must be `[]`, as in `Motoko.run([], "main.mo")`; any other list raises `Invalid_argument`. Use imports instead.
- `gcFlags` accepts only `"force"` and `"scheduling"`. `"incremental"`, `"enhancedOP"`, `"copying"`, `"marking"`, `"generational"` and `"classicOP"` raise `Invalid_argument`; remove those calls.

## Release artifacts

- There is no Intel-Mac build (`motoko-Darwin-x86_64`). On an Intel Mac, build from source, stay on `moc` 1, or use `moc.js`. Apple Silicon (`motoko-Darwin-arm64`), `x86_64` Linux and `aarch64` Linux builds are unchanged.
- The `motoko-base-library.tar.gz` artifact is gone. Get `base` from [mops](https://mops.one/base); `motoko-core.tar.gz` is unchanged.
