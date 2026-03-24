Stable CLI for dfx
==================

An important way of using the Motoko compiler is via the the `dfx` command,
provided by the [IC SDK](https://internetcomputer.org/docs/current/developer-docs/setup/install), which provides project and package management
support.

This document describes the interface that `moc` and related tools provide to
`dfx`. The goal is that
 * the Motoko developers know which command line flags have to
   be kept stable in order to not break `dfx`, and that
 * the SDK developers have a single place to read about the `moc` interface, and
   a place to express additional requirements (by collaborating on a PR against
   this document.)

This interface includes:
 * nix derivations imported by the IC SDK
 * binaries executed
 * command line arguments and environment variables passed to these binaries
 * where these binaries read files and
 * where these binaries write files, output or temporary
 * where they do _not_ write to, so that upgrading `moc` doesn’t suddenly leave
   artifacts where `dfx` does not expect them

It does not replace proper documentation, but should be kept rather concise.

Nix derivations
---------------

The `motoko` repository defines the following nix derivations, as attributes of
the top-level `default.nix`:

* `moc`: contains `bin/moc`
* `mo-doc`: contains `bin/mo-doc`
* `didc`: contains `bin/didc`
* `base-src`: contains the base package, directly in the top level directory,
  as `*.mo` files. It does not contain extra files (test files, for example)


The `default.nix` file itself takes an optional `system` parameter which is
either `"x86_64-linux"` or `"x86_64-darwin"`, and defaults to
`builtins.currentSystem`.

All binaries are either built statically (Linux) or only use system libraries (macOS).

Compiling Motoko Files to Wasm
------------------------------

In order to compile a Motoko file, `dfx` invokes `moc` with
```
moc some/path/input.mo            \
    -o another/path/output.wasm   \
    { --package pkgname pkgpath } \
    { --actor-alias alias url }   \
    [ --actor-idl actorpath ]     \
    { --public-metadata name }
```
This _reads_ the following files
 * `some/path/input.mo`
 * any `.mo` file referenced by `some/path/input.mo`, either relatively, absolutely or via the provided package aliases
 * for every actor import `ic:⟨canisterid⟩` imported by any of the Motoko files, it reads `actorpath/⟨canisterid⟩.did`, see section “Resolving Canister aliases” below. Here `⟨canisterid⟩` is the textual representation (e.g. `em77e-bvlzu-aq`).

The package name `prim` is special and should not be set using `--package`.

No constraints are imposed where imported files reside (this may be refined to prevent relative imports from looking outside the project and the declared packages)

This _writes_ to `another/path/output.wasm`, but has no other effect. It does
not create `another/path/`.

Compiler warnings and errors are reported to `stderr`. Nothing writes to `stdout`.

Resolving Canister aliases
--------------------------

For every actor imported using `import "canister:alias"`, the Motoko compiler treats that as `import "ic:⟨canisterid⟩"`, if the command line flag `--actor-alias alias ⟨canisterid⟩` is given. Here, `⟨canisterid⟩` is the textual representation (e.g. `em77e-bvlzu-aq`).

The first argument to `--actor-alias` is the alias without the URL scheme. The second argument must be a [textual representation] of the canister id.

Alternatively, `--actor-env-alias alias ⟨envvar⟩ ⟨idl-file⟩` resolves the alias at runtime by reading the IC environment variable named `⟨envvar⟩`, whose value must be the textual representation of the canister id. This is useful when the target canister id is only known at deployment time.

`--actor-id-alias alias ⟨canisterid⟩ ⟨idl-file⟩` is like `--actor-alias` but accepts an explicit IDL file path as a third argument, bypassing the `--actor-idl` search path. This is useful when the IDL file lives outside the standard IDL search path.

The given aliases must be unique (i.e. no `--actor-alias a aaaaa-aa --actor-alias a em77e-bvlzu-aq`, no `--actor-env-alias a MY_CANISTER ⟨idl-file⟩ --actor-alias a em77e-bvlzu-aq`).

[textual representation]: https://sdk.dfinity.org/docs/interface-spec/index.html#textual-ids

Resolving Canister types
------------------------

For every actor imported using `import "ic:⟨canisterid⟩"` (or `import "canister:alias"` if `alias` resolves to `ic:⟨canisterid⟩` as described above), the Motoko compiler assumes the presence of a file `⟨canisterid⟩.did` in the actor idl path specified by `--actor-idl`. This file informs Motoko about the interface of that canister, e.g. the output of `moc --idl` for a locally known canister, or the IDL file as fetched from the Internet Computer.

The `⟨canisterid⟩` here refers to the “textual representation“, e.g. `em77e-bvlzu-aq`.

This file informs Motoko about the interface of that canister. It could be the output of `moc --idl` for a locally known canister, or the IDL file as fetched from the Internet Computer, or created any other way.

Open problem: how to resolve mutual canister imports.

Exporting Canister Metadata
---------------------------

The compiler generates various metadata about the canister via command line flags.
The compiled Wasm module also includes these metadata in the custom sections.
The compiler flag `--public-metadata <name>` controls if the custom section is publicly accessible.
If `<name>` is in the `public-metadata` flag, the custom section name will be `icp:public <name>`.
Otherwise, it will be `icp:private <name>`. The `--public-metadata` flag applies to the imported actor classes as well.
In case the emission of a certain metadata section is not desired (e.g. to suppress discoverability, or to make the build
product more reproducible), `--omit-metadata <name>` can be applied to refrain from emitting it. This option should  be
used only in very specific cases.

* Candid interface.
  + Compiler flag `--idl` generates the Candid interface for the entry actor. The main service
    is always a service constructor, which contains the initialization arguments for installing the canister.
  + Custom section `candid:service` stores the interface for the running (initialized) canister, which removes
    the initialization arguments.
  + Custom section `candid:args` stores the initialization arguments. The argument types can refer to
    types defined in the `candid:service` custom section.
* Stable variable.
  + Compiler flag `--stable-types` generates the signatures for stable variables for the entry actor.
  + Custom section `motoko:stable-types` stores the signatures for stable variables.

The above metadata is stored in the Wasm module, and is only accessible by the controllers of the canister, unless the
metadata name is specified in the `--public-metadata` flag.

Moreover, the compiler generates a special marker custom section `"enhanced-orthogonal-persistence"` if the new orthogonal 
persistence support is enabled, see [Orthogonal Persistence](OrthogonalPersistence.md). This section is always private and
always emited independent of the compiler flags `--public-metadata` or `--public-metadata`.

Checking stable type compatibility
----------------------------------

The command
```
moc --stable-compatible old.most new.most
```
checks if the stable interface can evolve from `old.most` to `new.most` in
a type safe way without unintentional data loss.

If the check succeeds, nothing will be printed. 
If the check fails, the error message will be printed in stderr and the command returns with exit code 1.
The check can also emit warning messages, e.g. if stable variables are dropped.

With [enhanced orthogonal persistence](OrthogonalPersistence.md), the stable compatibility is also integrated in the runtime
system, to atomically guarantee memory compatibility during an upgrade.

Listing dependencies
--------------------

The command
```
moc --print-deps some/path/input.mo
```
prints to the standard output all URLs _directly_ imported by
`some/path/input.mo`, one per line. Each line outputs the original
URL, and optionally a full path if `moc` can resolve the URL, separated by a space.
For example,
```
mo:base/List
mo:other_package/Some/Module
ic:em77e-bvlzu-aq
canister:alias
./local_import some/path/local_import.mo
./runtime some/path/runtime.wasm
```
This _reads_ only `some/path/input.mo`, and writes no files.

By transitively exploring the dependency graph using this command (and
resolving URLs appropriately before passing them as files to `moc`), one can
determine the full set of set of `.mo` files read by the two compilation modes
described above (to wasm and to IDL).

Generating documentation
------------------------

In order to generate documentation for a given Motoko package `dfx` invokes
```
mo-doc
    [ --source source_dir ]
    [ --output output_dir ]
    [ --format html|adoc ]
```
The default source directory is `src`, the default output is `docs`, and the default format is `html`.
`mo-doc` will then generate documentation in the output directory mirroring the directory/file structure of the source directory.
