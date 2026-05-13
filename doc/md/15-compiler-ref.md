---
sidebar_position: 15
---

# Compiler reference

The Motoko compiler (`moc`) is the primary tool for compiling Motoko programs into executable WebAssembly (Wasm) modules. The compiler runs in the background when you build projects using the [IC SDK](https://github.com/dfinity/sdk). If you invoke the compiler directly on the command-line, you can press CTRL-C to exit.

This section provides compiler command-line reference information.

## moc

Use the Motoko compiler (`moc`) to compile Motoko programs into executable WebAssembly (Wasm) modules.

### Basic usage

``` bash
moc [option] [file ...]
```

### Options

You can use the following options with the `moc` command.

| Option                                    | Description                                                                                                                                           |
|-------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--ai-errors`                             | Emit AI tailored error messages.                                                                                                                      |
| `--actor-idl <idl-path>`                  | Specifies a path to actor IDL (Candid) files.                                                                                                         |
| `--actor-alias <alias> <principal>`       | Specifies an actor import alias.                                                                                                                      |
| `--args <file>`                           | Read additional newline separated command line arguments from `<file>`.                                                                               |
| `--args0 <file>`                          | Read additional `NUL` separated command line arguments from `<file>`.                                                                                 |
| `-c`                                      | Compile to WebAssembly.                                                                                                                               |
| `--check`                                 | Performs type checking only.                                                                                                                          |
| `--compacting-gc`                         | Use compacting GC (only available with legacy/classical persistence).                                                                                 |
| `--copying-gc`                            | Use copying GC (only available with legacy/classical persistence).                                                                                    |
| `--debug`                                 | Respects debug expressions in the source (the default).                                                                                               |
| `--enhanced-orthogonal-persistence`       | Use enhanced orthogonal persistence (default): Scalable and fast upgrades using a persistent 64-bit main memory.                                      |
| `--enhanced-migration <dir>`              | Enable enhanced migration system: requires initializers for all stable variables, disallows side-effects in actor bodies; only available with enhanced orthogonal persistence. |
| `--error-detail <n>`                      | Set level of error message detail for syntax errors, n in \[0..3\] (default 2).                                                                       |
| `--experimental-stable-memory <n>`        | Select support for the deprecated `ExperimentalStableMemory.mo` library (n < 0: error, n = 0: warn, n > 0: allow) (default 0).                        |
| `-fno-shared-code`                        | Do not share low-level utility code: larger code size but decreased cycle consumption (default).                                                      |
| `--generational-gc`                       | Use generational GC (only available with legacy/classical persistence).                                                                               |
| `-fshared-code`                           | Do share low-level utility code: smaller code size but increased cycle consumption.                                                                   |
| `--generate-view-queries`                 | Auto-generate queries for stable variables; preferring applicable .view() methods (default false)                                                     |
| `-help`,`--help`                          | Displays usage information.                                                                                                                           |
| `--hide-warnings`                         | Hides compiler warnings.                                                                                                                              |
| `-Werror`                                 | Treat warnings as errors.                                                                                                                             |
| `-A <codes>`                              | Disable (Allow) comma-separated warning codes, e.g. `-A M0194,M0223`                                                                                  |
| `-W <codes>`                              | Enable (Warn) comma-separated warning codes, e.g. `-W M0223`                                                                                          |
| `-E <codes>`                              | Treat as error comma-separated warning codes, e.g. `-E M0217`                                                                                         |
| `--warn-help`                             | Show available warning codes, current lint level, and descriptions                                                                                    |
| `--incremental-gc`                        | Use incremental GC (default, works with both enhanced orthogonal persistence and legacy/classical persistence).                                       |
| `--idl`                                   | Compile binary and emit Candid IDL specification to `.did` file.                                                                                      |
| `-i`                                      | Runs the compiler in an interactive read–eval–print loop (REPL) shell so you can evaluate program execution (implies -r).                             |
| `--implicit-derivation-depth <n>`         | Maximum recursion depth for [implicit](./fundamentals/11-implicit-parameters.md) argument derivation (default 100). Raise if a complex derivation is rejected as depth-limited.                                                                                                |
| `--legacy-persistence`                    | Use legacy (classical) persistence. This also enables the usage of --copying-gc, --compacting-gc, and --generational-gc. Deprecated in favor of the new enhanced orthogonal persistence, which is default. Legacy persistence will be removed in the future.|
| `--map`                                   | Outputs a JavaScript source map.                                                                                                                      |
| `--max-stable-pages <n>`                  | Set maximum number of pages available for library `ExperimentStableMemory.mo` (default 65536).                                                        |
| `-no-system-api`                          | Disables system API imports.                                                                                                                          |
| `-no-timer`                               | Disables timer API imports and hides timer primitives.                                                                                                |
| `-o <file>`                               | Specifies the output file.                                                                                                                            |
| `-p <n>`                                  | Sets the print depth.                                                                                                                                 |
| `--package <package-name> <package-path>` | Specifies a `<package-name>` `<package-path>` pair, separated by a space.                                                                             |
| `--public-metadata <name>`                | Emit ICP custom section `<name>` (`candid:args` or `candid:service` or `motoko:stable-types` or `motoko:compiler`) as `public` (default is `private`).|
| `--omit-metadata <name>`                  | Omit ICP custom section `<name>` (`candid:args` or `candid:service` or `motoko:stable-types` or `motoko:compiler`).                                   |
| `--print-deps`                            | Prints the dependencies for a given source file.                                                                                                      |
| `-r`                                      | Interprets programs.                                                                                                                                  |
| `--release`                               | Ignores debug expressions in the source.                                                                                                              |
| `--stable-regions`                        | Force eager initialization of stable regions metadata (for testing purposes); consumes between 386KiB or 8MiB of additional physical stable memory, depending on current use of ExperimentalStableMemory. |
| `--stable-types`                          | Compile binary and emit signature of stable types to `.most` file.                                                                                    |
| `--stable-compatible <pre> <post>`        | Test upgrade compatibility between stable-type signatures `<pre>` and `<post>`.                                                                       |
| `--rts-stack-pages <n>`                   | Set maximum number of pages available for runtime system stack (only supported with classical persistence, default 32).                               |
| `--trap-on-call-error`                    | Trap, don't throw an [`Error`](./core/Error), when an IC call fails due to destination queue full or freezing threshold is crossed. Emulates behavior of moc versions < 0.8.0.                                                                                                                                           |
| `-t`                                      | Activates tracing in interpreter.                                                                                                                     |
| `-v`                                      | Generates verbose output.                                                                                                                             |
| `--version`                               | Displays version information.                                                                                                                         |
| `-wasi-system-api`                        | Uses the WASI system API (`wasmtime`).                                                                                                                |


