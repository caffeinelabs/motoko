# Compiler plugin infrastructure tests

These small fixtures test the compiler's plugin API independently of production
plugins. `echo` is a fixed-input intrinsic that copies a Blob through shared
Wasm64 memory; `typed` specializes a variadic intrinsic using its argument types
and exercises managed scratch allocation, explicit output types, and
records/variants/arrays/options. Both use the same specialization API. Their
runtimes intentionally export the same function name with different signatures,
so composition also checks symbol isolation.

The `callbacks` fixture is a codegen-only package. It tests dependent argument
schemas, opaque generic values, compiler-owned scalar ordering, and ordinary
local Motoko callback dispatch. Callback coverage includes sequence arities,
effect order, ignored results, traps, reentry, mutable/closure payload sharing,
and opaque async values without suspension inside the plugin. A scalar callback
loop verifies that plugin argument passing allocates no managed tuples.
Nested argument suppliers check closure/argument order and trap propagation
while earlier managed arguments remain on the operand stack across callbacks.
Known tuple results also travel on the stack: immediate pair/triple destructuring
and unit callbacks allocate no temporary result tuples, while tuples retained in
arrays or across forced GC receive normal managed storage. Generic opaque
results retain their ordinary single-value representation.
Tabulation coverage includes empty arrays, nested callbacks, allocations that
grow memory during construction, and retaining callback results across GC.
Array construction initializes all slots before invoking callbacks and keeps
the normal allocation and pointer write barriers, while reusing an element
cursor under the SDK's no-GC-safepoint contract.

The `comparators` fixture exercises automatic scalar dispatch through generic
wrappers, all supported scalar representations, callback effects and traps,
contravariance, tuple/unit results, and nested deferred generators.
`comparator_codegen.py` also checks that scalar helpers actually appear in
emitted Wasm: unknown-only programs emit no classifier, and a Nat64-only
program emits only its Nat64 arm, including when an unused Text comparator is
present in the source. These checks complement semantic tests that would also
pass with the optimization disabled.

The `mutation` fixture exercises typed mutable array and record construction,
reads and writes, generic payload sharing, field-cell aliases, recursive nodes,
width subtyping, operand order, reentry and early returns. An IC fixture retains
more than 96 MiB across messages so writes occur during an incremental GC cycle,
with fragmented dead/live allocations, sanity checks and retained old/new
payloads. Packaging checks deliberately
invalid generator types and fields fail at compilation with normal diagnostics.

The `prepared` fixture declares distinct protected types backed by immutable
managed Blob bytes. It checks alias and generic passage, preparation/reading,
callback reentry and allocation, no wrapper allocation on reuse, and retention
in transient fields and async locals across GC. Negative tests cover forged
Blob/record values, wrong types, generic bounds, representation access, Candid,
shared signatures, and stable fields. Registry/owner checks run at compilation;
there is no runtime prepared-value registry or new GC representation.

The frontend validates intrinsic specializations, and the IR checker validates
their transformed metadata. Interpreters use that checked metadata without
repeating type specialization on every execution; dynamic argument arity and
callback calling conventions remain checked. Tuple fields already use constant
offset loads. Record fields still require hash lookup because width subtyping
allows actual records to contain additional fields.

From the Motoko repository root, in its Nix development shell:

```sh
test/compute-plugins/build.sh
test-runner -b test/compute-plugins/run
test-runner -b test/compute-plugins/fail
test-runner -b test/compute-plugins/trap
test-runner -b test/compute-plugins/run-drun
python3 test/compute-plugins/packaging.py
python3 test/compute-plugins/prepared_types.py
python3 test/compute-plugins/comparator_codegen.py
```

The build uses a dynamically linked native `moc` and matching compiler
interfaces. Pass `--skip-motoko-build` when that compiler is already built.
Fixtures are emitted to `_build/compute-plugin-tests`; no Rust build or external
plugin repository is required. Source/IR interpreters and Wasm execution must
agree, diagnostics use normal expectations, and the IC test retains plugin
results across forced GC, including recoverable errors during construction and
values retained across nested callbacks.
Alias and callback tests cover intrinsic identity and ordinary Motoko wrappers.
`packaging.py` checks loader compatibility and normal diagnostics for invalid
packages, unresolved result types, invalid result stack shapes, and runtime-less
packages with runtime imports. Linker-specific validation lives in
`src/linking/test_linkModule.ml`.

Production CandidX/SHA-256/JSON implementations, conformance tests, and benchmarks
live in [motoko_plugins](https://github.com/Neutrinomic/motoko_plugins).
