# moc v2

Versions: `moc` 2.0.0-beta.4 · `mo-fmt` 0.2.0 · `mops` 3.4.1
Full reference: [moc v1 → v2 migration guide](doc/md/moc-v2-migration.md) · [Changelog](Changelog.md)

| Area | moc 1 | moc 2 |
|---|---|---|
| Persistence | `persistent actor` + `stable var` | plain `actor`, `transient` to opt out |
| Memory model | classical **or** EOP, 4 GCs | EOP only, incremental GC only |
| Control flow | `if (c) …`, `switch (e) { case (p) …; }` | `if c { … }`, `switch e { case p { … } }` |
| Safety | traps-in-waiting are warnings | they are errors |
| Dot notation | opt-in hint | on by default, works across nested modules |
| Formatting | Prettier plugin (Node) | `mo-fmt` (Rust, tree-sitter), rewrites to moc 2 syntax |

---

## 1. One file, end to end

The same actor in moc 1 and moc 2. Imports omitted. Both compile with moc 2 and have **the same stable signature** (`.most` files are identical), so the upgrade is safe.

<table>
<tr><th>moc 1</th><th>moc 2</th></tr>
<tr><td>

```diff
-persistent actor Shop {
   type Status = { #open; #paused; #closed : Text };
 
-  stable var orders : Nat = 0;
-  stable let stock = Map.empty<Text, Nat>();
   transient var hits : Nat = 0;
 
   public query func describe(s : Status) : async Text {
-    switch (s) {
-      case (#open) { "open" };
-      case (#paused) { "paused" };
-      case (#closed(reason)) { "closed: " # reason };
-    };
   };
 
   public func buy(item : Text, qty : Nat) : async Text {
     hits += 1;
-    switch (Map.get(stock, Text.compare, item)) {
-      case (null) { return "unknown item" };
-      case (?n) {
-        if (n < qty) return "only " # Nat.toText(n) # " left";
-        Map.add(stock, Text.compare, item, n - qty : Nat);
-      };
     };
     orders += 1;
-    "ok";
   };
 
   public query func total(prices : [Nat]) : async Nat {
     var sum = 0;
-    for (p in prices.vals()) { sum += p };
-    sum;
   };
 
   public query func sign(n : Int) : async Text {
-    if (n < 0) "negative" else if (n == 0) "zero" else "positive";
-  };
-};
```

</td><td>

```diff
+actor Shop {
   type Status = { #open; #paused; #closed : Text };
 
+  var orders : Nat = 0;
+  let stock = Map.empty<Text, Nat>();
   transient var hits : Nat = 0;
 
   public query func describe(s : Status) : async Text {
+    switch s {
+      case #open { "open" }
+      case #paused { "paused" }
+      case #closed(reason) { "closed: " # reason }
+    }
   };
 
   public func buy(item : Text, qty : Nat) : async Text {
     hits += 1;
+    switch stock.get(Text.compare, item) {
+      case null { return "unknown item" }
+      case ?n {
+        if n < qty { return "only " # n.toText() # " left" };
+        stock.add(Text.compare, item, n - qty : Nat)
+      }
     };
     orders += 1;
+    "ok"
   };
 
   public query func total(prices : [Nat]) : async Nat {
     var sum = 0;
+    for p in prices.values() { sum += p };
+    sum
   };
 
   public query func sign(n : Int) : async Text {
+    if n < 0 { "negative" } else if n == 0 { "zero" } else { "positive" }
+  }
+}
```

</td></tr>
</table>

How the right column was produced:

| Step | Command | What it changed |
|---|---|---|
| 1 | `mops check --fix` | dropped `stable` (M0218), `Map.get(stock, …)` → `stock.get(…)` (M0236) |
| 2 | `mops format` with `mo-fmt` in `moc2` mode | parentheses, `case` patterns, `;` after `case` arms, braced branches |
| 3 | by hand | `persistent actor` → `actor` (M0217), `.vals()` → `.values()` (M0269) |
| 4 | by hand, optional | trailing `;` after the last expression: moc 1 style uses it everywhere, moc 2 style keeps `;` only between declarations and statements. Both compile |

---

## 2. Persistence

### Actors are persistent by default

```motoko
// moc 1
persistent actor {
  stable var count : Nat = 0;
  transient var hits : Nat = 0;
};
```

```motoko
// moc 2
actor {
  var count : Nat = 0;
  transient var hits : Nat = 0;
};
```

- `persistent` / `stable` still compile, as warnings M0217 / M0218. Deleting them keeps the stable signature.
- `--default-persistent-actors`, `--require-persistent-actors`, `--legacy-actors` are gone.
- `flexible` is removed: write `transient`.

### One memory model

| | moc 1 | moc 2 |
|---|---|---|
| Persistence | classical (32-bit) or EOP | EOP (64-bit) only |
| GC | copying, compacting, generational, incremental | incremental |
| Raw stable memory | `ExperimentalStableMemory` | `Region` |

A classical canister still upgrades: compile that one upgrade with `--enhanced-orthogonal-persistence`.

### `preupgrade` / `postupgrade`

No change in moc 2: deprecated since moc 1.16.0 (warning M0270), still compile. Replacement: a migration function.

---

## 3. Syntax

All old forms keep working, except for the few whitespace cases in [3.3](#33-the-whitespace-rule) and [3.4](#34--is-whitespace-sensitive).

### 3.1 Control-flow heads need no parentheses

```motoko
// moc 1
if (f(x)) a else b;
if (f(x)) { a } else { b };   // current house style: parens and blocks
while (n > 0) { n -= 1 };
for (x in xs.vals()) { total += x };
switch (p.x) { ... };
```

```motoko
// moc 2
if f(x) { a } else { b };
while n > 0 { n -= 1 };
for x in xs.values() { total += x };
switch p.x { ... };
```

Without parentheses, the body must be a block.

### 3.2 Lighter `switch`

```motoko
// moc 1
switch (o) {
  case (null) { 0 };
  case (?n) { n };
};
switch (n) {
  case (-1) { "minus one" };
  case (0 or 1) { "small" };
  case (x) { debug_show x };
};
```

```motoko
// moc 2
switch o {
  case null { 0 }
  case ?n { n }
};
switch n {
  case -1 { "minus one" }
  case 0 or 1 { "small" }
  case x : Int { debug_show x }
};
```

- `;` between cases is optional.
- No parentheses needed for literals, `null`, `?p`, `#tag`, `#tag(p)`, `_`, combined with `or`, `and`, `: T`.
- A payload always takes its own parentheses: `case #node(n)`, never `case #node n`.
- `catch` takes the same patterns: `catch e : Error { ... }`.

### 3.3 The whitespace rule

Glued continues the condition, spaced starts the branch:

| Code | Meaning in moc 2 |
|---|---|
| `if xs[i] { … }` | condition `xs[i]` |
| `if c [i] else []` | condition `c`, branch `[i]` |
| `if (c) -1 else 1` | condition `c`, branch `-1` |
| `if (c)-1 else 1` | condition `(c)-1`, **error M0275** |
| `if n - 1 > 0 { … }` | spaced on both sides: just an operator |
| `if f(x) a else b` | compound condition, bare branches: **error M0275** |

This rule exists only so that the moc 1 form (bare branches) and the moc 2 form (paren-free heads) can coexist. Once the moc 1 form is removed, branches are always blocks and the rule goes away.

### 3.4 `??` is whitespace-sensitive

| Code | moc 1 | moc 2 |
|---|---|---|
| `o ?? 0` | coalesce | coalesce |
| `o ??0` | coalesce | syntax error |
| `??x` | `?(?x)` | `?(?x)` |
| `o ?? { x = 1 }` | syntax error | record literal |
| `o ?? { let k = f(); k + 1 }` | block | **M0273**, write `o ?? do { … }` |

### 3.5 `do` blocks as operands

`do { … }` is the block you can put anywhere an expression goes.

```motoko
// an escape is already an expression: no `do`
let u = users.get(id) ?? throw Error.reject("no user " # id.toText());
let v = map.get(k) ?? return null;

// memoize: compute, store, return
cache.get(k) ?? do {
  let v = compute(k);
  cache.add(k, v);
  v
};

// option chain with a default
do ? { u.address!.city! } ?? "unknown";

// operands
base - do { if vip { 10 } else { 0 } };
debug_show do { let { name } = u; name };
(do { … }).field;   // postfix needs parentheses
```

### 3.6 Blocks: where they are required, where you write `do`

Direction for the syntax beyond moc 2: **no position accepts both a block and an expression.** A `{` then never needs to be guessed as block or record. moc 2 already accepts every form below; the moc 1 optional-block forms (`if (c) a else b`, `func f() = e`) are what goes away later.

**Always a block** `{ … }`:

| Construct | Form |
|---|---|
| `if` | `if e { … } else { … }`, `else if e { … }` |
| `while` | `while e { … }` |
| `for` | `for p in e { … }` |
| `loop` | `loop { … } while e` |
| `switch` / `case` | `switch e { case p { … } case q { … } }` |
| `try` / `catch` / `finally` | `try { … } catch p { … } finally { … }` |
| `async` / `async*` | `async { … }`, `async* { … }` |
| `do` / `do ?` | `do { … }`, `do ? { … }` |
| `func` | `func f(x : T) : U { … }` |

**Always an expression**, a block needs `do { … }` (here `{` would be a record):

| Construct | Form | With a block |
|---|---|---|
| `return`, `throw` | `return e` | `return do { … }` |
| `break` | `break l e` | `break l do { … }` |
| `ignore`, `assert` | `ignore e` | `ignore do { … }` |
| `debug` | `debug assert e` | `debug do { … }` |
| `await`, `await*`, `await?` | `await e` | `await do { … }` |
| `debug_show` | `debug_show e` | `debug_show do { … }` |
| `label` | `label l while e { … }` | `label l do { … }` |
| `let … else` | `let ?v = e else return` | `let ?v = e else do { … }` |
| `??` | `a ?? e` | `a ?? do { … }` |
| `let`, `var` | `let x = e` | `let x = do { … }` |
| arguments, fields, operands | `f(e)`, `{ x = e }`, `e1 + e2` | `f(do { … })` |
| control heads | `if e { … }`, `switch e { … }` | a record head needs parens: `switch ({ x = 0 }) { … }` |

### 3.7 Parse errors that tell you the fix

| Code | Input | Message (abridged) |
|---|---|---|
| M0272 | `case null { x = 0 }` | `x = e` is a record field, but this position holds declarations … nest it as the block's result: `{ { x = 0 } }` |
| M0272 | `switch { x = 0 } { … }` | a record literal is not allowed in this position; wrap it: `({ ... })` |
| M0273 | `o ?? { let k = 2; k }` | braces enclose a record literal in this position, not a block; use `do { ... }` |
| M0274 | `let query = 1` | `query` is a reserved keyword; choose a different name (e.g. `query_`) |
| M0275 | `if (c)-1 else 1` | expected a block `{ ... }` … a spaced `(`/`[` starts the branch |

In moc 1 all of these were a generic `syntax error [M0001]`.

---

## 4. Stricter by default

### 4.1 Warnings that are now errors

Code that traps, or silently does something other than what it says. `-W <code>` turns one back into a warning.

| Code | moc 1 (warning) | Meaning | moc 2 fix |
|---|---|---|---|
| M0145 | `let #ok(n) = r;` | pattern doesn't cover every value: traps on `#err` | `let #ok(n) = r else { return 0 };` |
| M0222 | `ignore bump();` | ignoring an `async*` value: the computation never runs | `await* bump();` |
| M0215 | `{ u with emial = e }` | field the expected type drops: a typo is silently discarded | `{ u with email = e }` |
| M0212 | `(with cycle = n)` | unknown parenthetical attribute: ignored, no cycles sent | `(with cycles = n)` |
| M0210 | `await* (with cycles = n) pay();` | parenthetical on `await*`: has no effect | attach cycles to an `async` call |
| M0128 | `func heartbeat() : async () { … }` | named like a system method but not `system`: never called by the IC | `system func heartbeat() …` |
| M0242 | `public func log(t : Text) { … }` | no return type: implicitly oneway, callers get no reply or error | `public func log(t : Text) : () { … }` |
| M0005 | `import L "lib";` for `Lib.mo` | import path case differs from the file: breaks on case-sensitive filesystems | `import L "Lib";` |
| M0278 | migration file without `public func migration` | file in the migrations dir is skipped: the migration never runs | export `migration` (after beta.4) |
| M0142 | library file with bare declarations | imported library isn't a `module`: deprecated form | wrap in `module { … }` |
| M0193 | `actor class C() : actor {} { … }` | actor class with a non-`async` return type: creation is async | `: async actor {}` |

### 4.2 Inferred `Any` / `None` is an error

```motoko
let xs = [1, "two"];                // M0074: annotate `: [Any]` if intended
if b { 1 } else { "one" };          // M0081
type U = Nat and Text;              // M0166: None
```

### 4.3 Constant comparisons are an error

```motoko
n == t        // n : Nat, t : Text      M0276 (was warning M0062)
user == order // records sharing no field  M0276
```

### 4.4 Patterns must fit the matched type

Lands after 2.0.0-beta.4.

```motoko
type Status = { #active; #suspended };

switch s { case #suspnded { … } … }   // M0116: did you mean tag #suspended?
switch n { case -1 { … } … }          // n : Nat, M0050 as for `let n : Nat = -1`
```

In moc 1 these were "pattern never matched" warnings at best, and nothing at all in `let … else`.

---

## 5. Types and libraries

### Record update copies `var` fields

```motoko
let base = { var count = 0; name = "a" };
let copy = { base with name = "b" };
copy.count += 1;
debug_show (base.count, copy.count)   // moc 1: error M0179 · moc 2: (0, 1)
```

### Dot notation everywhere

```motoko
Map.size(m)    // moc 2: warning M0236, you can use the dot notation `m.size(...)`
m.size()
m.add("a", 1)  // implicit `compare`
```

- M0236 is on by default, and `mops check --fix` applies it.
- Dot notation and implicits also search nested modules, so importing a facade that re-exports its package is enough.

### Read-only primitives in queries

```motoko
public query func me() : async Principal { Prim.getSelfPrincipal() };
// moc 1: M0197 `system` capability required · moc 2: OK
```

Also `envVar`, `envVarNames`, `callerInfoSigner`, `callerInfoData`, `getCandidLimits`, `getCandidTypeLimits`.

---

## 6. Compiler and tooling

- **`moc --check a.mo b.mo` checks each file on its own.** No more concatenation into one program; `-c`, `--idl`, `-r` take one main file.
- **Removed flags** (now `unknown option`): `--legacy-persistence`, `--copying-gc`, `--compacting-gc`, `--generational-gc`, `--incremental-gc`, `--legacy-actors`, `--default-persistent-actors`, `--require-persistent-actors`, `--experimental-stable-memory`, `--experimental-field-aliasing`, `--generate-view-queries`, `--trap-on-call-error`, `-no-system-api`, `-ref-system-api`, `--profile*`, ….
- **Removed primitives:** `Prim.stableMemory*`, `Prim.createActor`.
- **`moc.js`:** `Motoko.run([], file)`; `gcFlags` only `"force"` / `"scheduling"`.
- **Release artifacts:** no Intel-Mac binary, no `motoko-base-library.tar.gz`.

---

## 7. Formatting: `mo-fmt` + `mops format`

### What `mo-fmt` is

- Rust binary, built on the tree-sitter grammar in `caffeinelabs/tree-sitter-motoko`. Replaces the Node `prettier-plugin-motoko`.
- **Keeps your line breaks**: a list on one line stays on one line, a broken list gets one item per line.
- **Never changes meaning**: every run re-parses its own output and refuses to write code that parses differently.
- Two modes:

| `syntax` | Does |
|---|---|
| `preserve` (default) | indentation and spacing only; any moc version |
| `moc2` | also rewrites to moc 2 syntax: drops head and `case` parentheses, braces every control body, drops `;` after braced `case` arms |

### Setup

```bash
mops toolchain use mo-fmt 0.2.0
```

```toml
# mops.toml
[toolchain]
moc = "2.0.0-beta.4"
mo-fmt = "0.2.0"
```

```toml
# mo-fmt.toml (optional)
syntax = "moc2"
indent-width = 2
```

Without the `mo-fmt` pin, `mops format` still runs the Prettier plugin.

### Commands

| Command | Does |
|---|---|
| `mops format` | format in place |
| `mops format --check` | CI: list files that need formatting, exit 1 |
| `mops format -- --syntax moc2` | try the moc 2 rewrite without a config file |
| `mops format --verify` | run `mops check` afterwards, revert formatting if it fails |
| `// mo-fmt-ignore` | leave the next item as written |

### `moc2` mode on the demo file

Real `mo-fmt --syntax moc2` output on the moc 1 column. Semicolons outside `case` arms are kept.

```diff
   public query func describe(s : Status) : async Text {
-    switch (s) {
-      case (#open) { "open" };
-      case (#paused) { "paused" };
-      case (#closed(reason)) { "closed: " # reason };
+    switch s {
+      case #open { "open" }
+      case #paused { "paused" }
+      case #closed(reason) { "closed: " # reason }
     };
 ...
-    switch (Map.get(stock, Text.compare, item)) {
-      case (null) { return "unknown item" };
-      case (?n) {
-        if (n < qty) return "only " # Nat.toText(n) # " left";
+    switch Map.get(stock, Text.compare, item) {
+      case null { return "unknown item" }
+      case ?n {
+        if n < qty { return "only " # Nat.toText(n) # " left" };
         Map.add(stock, Text.compare, item, n - qty : Nat);
-      };
+      }
     };
 ...
-    for (p in prices.vals()) { sum += p };
+    for p in prices.vals() { sum += p };
 ...
-    if (n < 0) "negative" else if (n == 0) "zero" else "positive";
+    if n < 0 { "negative" } else if n == 0 { "zero" } else { "positive" };
```

---

## 8. Upgrading a project

```bash
mops toolchain use moc 2.0.0-beta.4
mops toolchain use mo-fmt 0.2.0
mops check --fix
mops format --verify -- --syntax moc2
mops check-stable
```

Then remove the [removed flags](#6-compiler-and-tooling) from `[moc].args` and fix what `mops check` still reports.
