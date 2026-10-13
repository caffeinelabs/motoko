# Compute compiler plugins

This experimental native `moc` extension adds functions to `mo:⛔` from separate
plugin packages. Each package contains a compiler component (`plugin.cmxs`)
and, when needed, a position-independent Wasm64 runtime (`runtime.wasm`). The compiler
component supplies the function's type policy, specialization generator, and
reference implementation for the interpreters. The runtime supplies compute
kernels and temporary helpers.

Plugin implementations are maintained in
[Neutrinomic/motoko_plugins](https://github.com/Neutrinomic/motoko_plugins),
including CandidX (`Prim.to_candid_canonical`), SHA-256 (`Prim.sha256`),
and JSON (`Prim.to_json` and `Prim.from_json<T>`).
That repository owns their builds, tests, fuzz adapters, and PocketIC
benchmarks. It is a separate checkout, not a Motoko submodule or build
dependency. This repository contains the compiler API, loader, linker, and
small test-only plugins.

## Use

Follow the plugin repository's build instructions to produce packages for a
compatible dynamically linked native compiler. With sibling checkouts:

```sh
bin/moc --plugin ../plugins/dist/candidx \
        --plugin ../plugins/dist/sha256 \
        --plugin ../plugins/dist/json example.mo -o example.wasm
```

```motoko
import Prim "mo:⛔";

switch (Prim.to_candid_canonical({ height = 42; memo = ?"hello" })) {
  case (#ok bytes) { let digest = Prim.sha256(bytes) };
  case (#err message) { Prim.debugPrint(message) };
};
type User = { name : Text; age : Nat8 };
let user = Prim.from_json<User>("{\"name\":\"Ada\",\"age\":37}");
```

Pass the same `--plugin` options for `--check`, `-r`, or `-r -iR`. They add public
bindings when the compiler constructs its Prim module; no application-side
FFI declarations are needed. Only referenced runtimes are linked. Multiple
packages can be loaded in either order. Ordinary `to_candid` and `from_candid`
retain their existing behavior.

All plugin bindings are compiler-specialized intrinsics, including fixed-input
functions such as `Prim.sha256`. They preserve their identity through inferred
aliases and specialize calls using argument types and explicit type parameters.
Their type cannot be erased to an ordinary function annotation. Use an
explicitly typed wrapper when passing a plugin computation as a callback:

```motoko
func encodeHeight(x : Nat) : { #ok : Blob; #err : Text } {
  Prim.to_candid_canonical(x)
};
func hash(bytes : Blob) : Blob = Prim.sha256(bytes);
```

Argument lists and tuple values remain distinct:
`Prim.to_candid_canonical(1, true)` encodes two arguments;
`Prim.to_candid_canonical((1, true))` encodes one tuple argument.

## Loading prebuilt plugins during canister compilation

This is already supported. A compiler with the plugin SDK can load several
prebuilt packages on each invocation without rebuilding `moc`:

```sh
bin/moc --plugin /path/to/candidx \
        --plugin /path/to/sha256 \
        app.mo -o app.wasm
```

Each directory contains `plugin.cmxs` and its optional `runtime.wasm`. The CLI dynamically
loads `plugin.cmxs` using OCaml `Dynlink.loadfile_private` before typechecking.
The extension registers its Prim functions, type policy, interpreter reference,
and code generator. During canister compilation, generated code uses the
original Motoko types and the linker incorporates the referenced runtime Wasm.
The resulting canister is an ordinary Wasm module with direct calls. Loading
the compiler extension dynamically adds no runtime plugin dispatch and does
not change the GC borrowing contract.

The canister still needs compilation; the compiler executable does not need
rebuilding for each compatible plugin. Plugin sources and build tools live in
[the separate plugin repository](https://github.com/Neutrinomic/motoko_plugins).
They build against a matching compiler checkout's interfaces without copying
plugin source into the compiler tree. An installed, standalone SDK has not yet
been packaged.

Native `.cmxs` files must match the compiler/OCaml interfaces and host platform.
Loading them also requires a dynamically linked host compiler with `dlopen`
support. The Linux/musl `release-static` compiler builds cannot load these
native packages; use a dynamically linked native build such as the compiler
built in the Motoko development environment with the `release` profile.
The explicit plugin ABI version is checked, but is not a promise of binary
compatibility across compiler releases. Updating the compiler can require new
plugin compiler components. New plugins supported by the existing SDK need no
compiler modification; extending the SDK itself can require one. Stock `moc`
without this extension and the current JavaScript compiler cannot load these
native packages.

Incompatible native interfaces/implementations, native loader failures, and
unsupported SDK ABIs produce an `unsupported compute plugin` diagnostic and
exit status 1. The diagnostic preserves the concrete cause and names the
running compiler and OCaml version so the developer can select a matching
package build. ABI mismatches include the required and supported ABI numbers.
Arbitrary plugin initialization failures and duplicate registrations remain
separate errors. Native loader failures can also mean a missing dependency or
wrong host platform; their original reason is retained.

Compatibility is checked using OCaml's native interface/implementation checks
and the explicit plugin ABI, rather than an exact Motoko release allowlist.
The ABI remains 1 while the compiler PR is unmerged. Packages must be rebuilt
against the matching compiler checkout. After merging, a semantically
incompatible SDK change must advance its ABI version even if the OCaml
signatures remain unchanged. A successful compatibility check does not prove
that the plugin implementation is correct.

Package names, internal function IDs, and plugin Prim names must be distinct;
duplicates fail during loading. Separate packages may reuse private runtime
Wasm export names. Plugins remain trusted compiler extensions and share the
canister's memory once linked. The current loader has no in-process unload or
replacement operation; a new compiler invocation loads the desired packages.
Replacing a deployed canister's plugin code requires compiling and deploying
an updated canister module.

The implementation is in [`moc.ml`](../src/exes/moc.ml),
[`plugin_codegen.ml`](../src/codegen/plugin_codegen.ml), and
[`compute_plugin.ml`](../src/mo_values/compute_plugin.ml). Existing package
checks cover independent packages, unused runtimes, explicit `.cmxs` paths,
and invalid runtime diagnostics.

## Execution, allocation, and GC

There is one final Wasm module and one Motoko main memory. Plugin runtime
functions become direct Wasm calls during static linking. There is no runtime
plugin loader, secondary Wasm instance, or host dispatch per value. Generated
traversals use the compiler's heap layout and forwarding barriers, and results
use its normal allocation barrier before becoming observable Motoko values.

The raw-pointer borrowing interval is synchronous and must contain no GC
safepoint, callback into Motoko, suspension, or retained pointer. The current
runtime collects at explicit safepoints, not on every allocation. Memory growth
does not change Wasm addresses. Previously forwarded objects must be read
through the compiler's existing forwarding barriers.

`env.compute_plugin_alloc(size: i64, align: i64) -> i64` provides temporary
memory using unrooted Motoko Blob payloads, following the RTS
`EphemeralAllocator`. Alignment must be a power of two up to 65,536 bytes.
Deallocation is a no-op: unreachable scratch objects are reclaimed by a later
GC increment, after every plugin pointer is dead. This avoids a separate
persistent allocator or plugin heap. `env.compute_plugin_trap(ptr: i64,
len: i64)` terminates a failing computation. Neither function can trigger a GC
increment. Trapped computations must not depend on Rust destructors running.

Pure functions can still allocate garbage. Repeated synchronous calls can
accumulate scratch and discarded results before the next collection. The
allocator validates overflow and alignment; each plugin must enforce any
algorithm-specific allocation limit. Sharing memory means an incorrect offset
or length can overwrite another object without crossing Wasm memory bounds.
Compatibility checks do not prove memory safety or correct output.

Managed algorithms may call ordinary synchronous Motoko closures through
`Backend.Call.local`, using the full instantiated `Local`/`Returns` function
type and the compiler's existing closure calling convention. The compiler
preserves argument evaluation order and distinguishes one tuple-valued argument
or result from multiple sequence arguments or results. Current collection
points run after synchronous message frames return; a local callback cannot
run collection inside its caller. Keep only vanilla managed references across
callbacks, never borrowed addresses. Callbacks may allocate, reenter a plugin,
or schedule asynchronous work, but this interface cannot suspend the caller.
The interpreter protocol is continuation-based and invokes callbacks with the
ordinary actor context. `Compute_plugin.pure` lifts a callback-free reference
computation into this protocol.

`Backend.Compare.dispatch` lets an operation use scalar ordering automatically
when given an ordinary comparator, including through generic Motoko wrappers.
The compiler recognizes complete, effect-free comparison bodies for integers,
Bool, Char, Text, Blob and Principal. It checks emitted closure identity once
per operation; unrecognized comparators, including effectful functions and
custom orderings, use the original callback. Float ordering is not recognized.
Names and package paths provide no
proof, and callbacks are never executed to discover their behavior.

The generator supplies already evaluated arguments, the comparator's argument
index, Wasm result types, and a body generator. The body receives a fresh
environment and argument getters, plus `Some primitive` for proven scalar
ordering or `None` for callback fallback. The proof describes ordering, not
the original storage type: generic keys and comparator contravariance must
retain their existing representations and GC barriers. Bodies are generated
after ordinary functions, using only scalar kinds whose comparator closures
were materialized. Programs with only unknown callbacks get no classifier or
scalar branches. The mechanism adds no closure fields or persistent state.

Plugins that emit only managed Wasm code register `runtime = None` and no
runtime imports. They need no dummy runtime module. A plugin with imports and
no declared runtime is rejected. The pure-map implementation demonstrates
managed structural sharing, callbacks, scalar comparison helpers, and a
codegen-only package.

The no-safepoint invariant is part of this compiler ABI. A future collector
that moves objects during allocation would require changing the adapter or
introducing rooted handles. Plugin-specific allocation measurements, limits,
and validation reports are kept with the implementations in
[the plugin design document](https://github.com/Neutrinomic/motoko_plugins/blob/main/docs/design.md).

## Implementing another plugin

The versioned compiler interface is
[`Plugin_codegen`](../src/codegen/plugin_codegen.mli). A package registers:

1. A unique package name, ABI version, optional runtime filename, runtime import
   signatures, and a `types` list (empty when no protected types are needed).
2. Public Prim bindings, each with a unique internal identity, an argument
   policy, explicit type parameter modes, result-type function, and an
   interpreter reference implementation.
3. A generator functor over `Plugin_codegen.Backend`. It receives one getter
   for each already evaluated vanilla argument and returns generated code with
   an explicit result representation. `Managed` leaves one vanilla value;
   `Tuple n` leaves the fields of a statically known tuple on the Wasm stack.
   The compiler checks its exact arity (zero or at least two), without promoting
   opaque type bounds, and packs a tuple only when the surrounding context
   needs one. Ordinary sequence calling conventions avoid allocating argument
   tuples. Specialization contains explicit type arguments, input types, and
   the checked result type.

The package's required ABI must be fixed at build time, such as `abi = 1` for
the current SDK. Do not copy `Plugin_codegen.abi_version` from the running
host: a plugin built with opaque or interface-only dependencies can otherwise
adopt a newer host's ABI without having been rebuilt for its semantics.

The SDK has two argument policies: `Fixed` computes an input type list from
explicit type arguments, and `Shared` accepts variadic shared inputs. Parameter
modes are `Concrete` (known layout required) or `Opaque` (an arbitrary type may
be retained and passed to typed callbacks). Abstract parameters remain tied to
all their occurrences in inputs and results; no erasure to `Any` is involved.
Both policies use the same specialization API. Intrinsics can return records,
variants, arrays, options, text, scalars, and generic managed values.
`from_json<User>(text)` specializes both validation and construction for `User`;
its result is `{ #ok : User; #err : JsonError }`. The plugin supplies that
structural error type. Expected input failures are values, not interpreter
exceptions or Wasm traps.

### Prepared values and precomputed tables

Plugins may declare protected types for validated keys and precomputed tables.
The compiler gives each declared type a distinct nominal identity and exposes
it through Prim. Ordinary Motoko code can retain it in records, arrays, generic
collections, closures, or transient actor fields. It cannot manufacture one from
Blob bytes, inspect its representation, or use its prepared type in Candid,
shared calls, or stable fields. Widening to `Any` is legal but loses the usable
prepared type; it does not promise physical deletion of the backing bytes. A key
prepared by one algorithm or for one purpose cannot be supplied where another
protected type is required.
Plugin authors must expose factories that validate inputs and build the cache;
consumers can then omit repeated validation and precomputation.

The compiler component declares a descriptor and includes it in registration:

```ocaml
module C = Mo_values.Compute_plugin
let key = C.Prepared.create ~owner:"example" ~name:"ExamplePreparedKey"
(* Include [key] in the plugin's [types] list. Use [C.Prepared.typ key] in
   argument schemas and result types. The plugin name must be "example". *)
```

A public Motoko module can export `type PreparedKey = Prim.ExamplePreparedKey`.
The descriptor is different from an `Opaque` generic parameter: generic opaque
values are caller-supplied payloads; a protected type is minted only by its
owning plugin's validated factory. The underlying abstract bound is `Any`, not
`Blob`, and only registered constructor identities receive this treatment.
Aliases and compiler IR transformations preserve that identity.

The representation is an ordinary immutable managed Blob. A generator uses
`Backend.Prepared.wrap descriptor blob_code` after completing its payload and
`Backend.Prepared.unwrap descriptor argument_type value_code` before reading
it with Blob helpers. Both emit the supplied code unchanged. Ownership and
argument type are checked during compilation, including in deferred generators;
there is no per-call tag, handle lookup, copy, or wrapper allocation. Reference
interpreters use `Compute_plugin.Prepared.wrap` and `unwrap`, with the same
immutable Blob representation; private byte formats may differ if observable
typed behavior agrees. The ordinary compiler/OCaml compatibility checks
require rebuilding plugin compiler components after SDK changes.

Payloads must consist entirely of initialized pointer-free bytes. Use an
explicit byte format or audited fixed-width limbs; do not copy native structs
with padding, pointers, or allocator ownership. Payload alignment is eight
bytes. Every borrow resolves normal GC forwarding, lasts only for the existing
synchronous borrowing interval, and must not survive a callback or call return.
Once published, a prepared payload must never be mutated. The ordinary GC owns
its lifetime and reclaims it after its last managed reference is dropped; no
new native heap, root table, or GC tag is involved. Precomputed public constants
that do not vary by key can stay in immutable plugin Wasm data segments instead
of being copied into every prepared value.

Fields retaining the prepared type must be transient across upgrades, so persist
the original serialized key when rebuilding is needed. Normal GC reclamation does
not promise secure erasure of secret bytes; a prepared secret intentionally retains secret
material for as long as the application retains it. Each plugin documents its
format, validation contract, retained size, and measured preparation/reuse costs.

Managed algorithms can also mutate ordinary Motoko arrays and record fields.
`Arr.tabulate_mut` creates `[var T]`; `Arr.set` requires that mutable array type
and the stored value's type. `Object.load` dereferences mutable fields,
`Object.set` requires a mutable field, and `Object.lit` constructs a concrete
record, creating a fresh mutable cell for each `var` field. Container shape,
field labels, mutability and assignment types are checked while generating code.
Generic payloads stay ordinary managed values. Setters evaluate all supplied
operands once, left-to-right, before resolving the destination and checking
array bounds, following function-call evaluation order. They leave no Wasm
result. Reads follow forwarding pointers; writes use the same pointer write
barrier and statically proven barrier elision as ordinary Motoko assignments.
The interpreter implementation must update existing `Value.Mut` refs in arrays
and records, preserving aliases. No hidden plugin heap or retained raw pointer
is needed. Borrowed-pointer Wasm kernels remain read-only with respect to their
inputs; managed mutation goes through these compiler-owned helpers.

The backend provides managed constructors with forwarding, allocation, and
write barriers. Record fields and variant payloads are evaluated before their
containers are allocated. Dynamic arrays start with initialized slots, allowing
early error returns without leaving invalid heap fields. These constructors
still require the synchronous no-GC-safepoint contract.

[SHA-256](https://github.com/Neutrinomic/motoko_plugins/tree/main/plugins/sha256)
is a small example; [CandidX](https://github.com/Neutrinomic/motoko_plugins/tree/main/plugins/candidx)
demonstrates recursive type specialization. Kernel modules are Wasm64 shared libraries with `dylink.0`
metadata, imported compiler memory/table/PIC globals, and only the approved
allocation and trap function imports. Read-only constants may have data
segments. The linker reserves their static data and table space alongside the
RTS, loads their data at initialization, resolves calls, then removes private
exports from each plugin after its imports have been resolved. Different
plugins may use the same private Wasm export names; their package names and
public Prim bindings must remain distinct.

Active data and element segments must fit their declared `dylink.0` memory
and table reserves. Their offsets must use the corresponding imported PIC
base, optionally adding nonnegative `i64` constants. The compiler rejects
inconsistent reserves and out-of-bounds linking indices before relocation.
`GOT.mem` symbols must resolve to defined immutable `i64` globals containing
constant offsets between zero and the memory reserve's size, inclusive.
Imported or already relocated addresses are not supported as relative offsets.

Plugins are trusted compiler extensions. Native components execute inside
`moc`, and linked Wasm shares memory with the application. The import validator
checks the runtime ABI; it is not a sandbox or a proof that an implementation
is pure. Borrowed-pointer runtime kernels must not mutate inputs, retain heap
pointers or private mutable state between operations, or call a collector.
Managed algorithms may retain and update ordinary caller-owned Motoko state
through the typed helpers above; it remains visible to the normal GC and
persistence machinery. Kernel algorithms remain in the packages rather than
the compiler's ordinary runtime.

Runtime authors must validate the Wasm binary before packaging it, for example
with `wasm-validate --enable-memory64 runtime.wasm`. The external plugin build script
validates the packages it produces. The compiler checks the plugin ABI and linking
metadata; it does not perform complete Wasm instruction-stack typechecking.

## Validation

The compiler's tests use small local fixtures and require no external plugin
repository. From the Motoko root inside `nix develop`:

```sh
make -C rts
test/compute-plugins/build.sh
python3 test/compute-plugins/packaging.py
python3 test/compute-plugins/prepared_types.py
```

The [compute-plugin workflow](../.github/workflows/compute-plugins.yml) also runs
source, IR, Wasm, and diagnostic fixtures plus linker regression tests and
executes composed Wasm modules. Production plugin tests and performance reports
live in the [plugin repository](https://github.com/Neutrinomic/motoko_plugins).

The JavaScript compiler remains buildable but cannot load native `.cmxs`
packages. Supporting it would require packaging compiler components with the
JS build or a different compile-time component ABI.

## Relationship to Wasm components

Compiler specialization and component packaging solve different parts of this
integration. A separately instantiated component normally transfers variable
length values through its interface and isolated memory; core modules within
one component can share memory and direct calls. A compatible shared-memory
core ABI could retain this plugin system's access pattern and GC contract.

There is no measured component-versus-plugin slowdown. The recorded PocketIC
comparisons are against Motoko libraries. The plugin repository keeps the
[component discussion and benchmark methodology](https://github.com/Neutrinomic/motoko_plugins/blob/main/docs/design.md#relationship-to-wasm-components)
with the implementations.
