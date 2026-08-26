# Migration objects: freezing compiled migrations instead of frozen sources

Status: PROPOSAL (draft for discussion)
Audience: moc compiler team, Caffeine platform (mops, caffeine-ai, caffeine-db, CaLM)
Supersedes: `design/TruncatedMigrationChains.md` (chain trimming is retained as a
pressure-relief mechanism, § Epochs and trimming)

## Summary

Under `--enhanced-migration`, every migration ever deployed is recompiled from
source into every future wasm. This proposal makes the compiler emit, alongside
each build, a **chain object** (`.mco`): a container holding one *compiled,
closed, linkable* entry per migration in the chain, plus the type metadata the
chain checks need. The next build consumes the previous chain object, compiles
only the *pending* migration sources, links the frozen entries binarily, and
emits an extended chain object.

Consequence: once a migration is frozen into a chain object, its **source is no
longer a build input**. A mops package bump, a mo:core release, or a moc
frontend change cannot break it, because nothing ever parses or typechecks it
again — its compiled bytes are reused verbatim. The chain stays as long as we
want inside a single wasm, so one install still fast-forwards a canister across
arbitrarily many versions, and fresh installs still replay from `{}`. Nothing
changes at runtime, in the `.most` format, or in the deploy protocol.

The price is one new, deliberately narrow stability commitment: a versioned
**migration ABI** — the set of runtime-system entry points and conventions that
frozen code links against. This document argues that commitment is small,
mechanically testable, and mostly already implied by enhanced orthogonal
persistence.

## The problem, restated

A migration file, once deployed, is semantically frozen: re-running a changed
version on canisters that already applied it is not an option (the runtime
skips applied migrations by name), and its behavior on canisters that have
*not* yet applied it must never drift. Today we freeze the **bytes** (read-only
files, deploy gates, agent guards) but keep **recompiling** those bytes with a
moving toolchain:

- imports resolve against the *current* mops package set — a bump can make a
  frozen file fail to compile or, worse, change its behavior;
- the *current* moc frontend must keep accepting it — every compiler release is
  a risk against files nobody may edit;
- the whole history is re-typechecked and re-code-generated on every build,
  for code that is byte-identical since the day it was deployed.

The invariant we actually need is *frozen semantics*. The only representation
of a migration whose semantics cannot drift under a moving toolchain is the one
that no part of the moving toolchain processes: its already-compiled code.

## The idea, from first principles

### What a compiled migration is

Each chain file is a module exporting `migration : Dom -> Rng`, where `Dom` and
`Rng` are stable-typed records. The desugarer (`src/lowering/desugar.ml`)
generates, per chain level `k`, a step function that:

1. checks `was_migration_performed(lab_k)` and short-circuits via
   `ICStableRead(mem_typ_k)` if so;
2. otherwise obtains state `k-1` recursively, **extracts** the fields of
   `Dom_k` (trapping on absence), **calls `migration`**, **merges** its output
   with the carried fields into state `k`, and registers `lab_k`.

The call in step 2 is an ordinary function call on heap values. Everything
around it — extract, merge, the step scaffolding, the boundary state types
`mem_typ_k` — is generated glue.

### The freeze boundary is exactly the user's function

The glue cannot be frozen, for a fundamental reason: `mem_typ_k` is computed by
*reverse-folding* from the **current** actor's stable fields through the chain
suffix (`Type.pres`). Boundary state types legitimately change as the chain
grows (a later migration may consume a field that was previously merely
carried), so any frozen code that bakes in a `mem_typ_k` is unsound against
future chains.

`Dom` and `Rng`, by contrast, are fixed forever the moment the migration is
frozen — they are the migration's *own* type, recorded in every `.most` v4
signature since (`Multi`'s chain fields are precisely label + function type).

So the cut is forced, and it is fortunate: **freeze the leaf function the user
wrote, on its own fixed types; regenerate all glue every build.** The frozen
artifact is a closed compilation of `migration` — the function plus every
library function it transitively reaches, tree-shaken in at freeze time. Its
interface to the surrounding wasm is one exported function on heap values.

### Why binary freezing is feasible now

Three facts, all already true of this codebase:

1. **moc ships a static wasm linker and uses it on every build.**
   `src/linking/linkModule.ml` links the precompiled `mo-rts.wasm` into every
   generated module today — resolving imports by name, renaming function and
   global index spaces, and rebasing the table. It is not a general object
   linker (it is a one-shot, two-module merge tailored to the RTS's PIC
   dialect — see the audit section), but the index-renaming machinery that a
   multi-object extension needs already exists and is exercised on every
   build. Crucially, moc-produced objects do **not** need the RTS's LLVM PIC
   dialect at all: moc-generated code bakes no static linear-memory addresses
   (see fact 4 below), so the linking problem reduces to three well-defined
   index spaces.
2. **Enhanced orthogonal persistence already froze the value representation.**
   The persistent heap must be readable by the next compiler release's output;
   heap object layout (tagging, headers, field layout, 64-bit words) is already
   a compatibility boundary that moc maintains, breakable only in rare,
   coordinated events. A frozen migration's inputs, outputs, and every value it
   allocates internally live in exactly that layout. The hardest half of
   "binary compatibility" is a commitment we already keep for other reasons.
3. **Migration code is the easy fragment of the language, by type discipline,
   not by convention.** `check_migration_function` requires a `Local`,
   non-generic function (`tbs = []`), so the body types under `NullCap`
   (`src/mo_frontend/typing.ml`, `infer_async_cap`): no `async`/`await`, no
   `try/catch`, no message sends, no `system` capability (hence no timers, no
   `ExperimentalCycles`). That mechanically excludes the four nastiest
   whole-program couplings — the continuation table, the global Candid type
   table, prelude-by-name internals (`@add_cycles` etc.), and serialization
   tag stamping. The remaining runtime-system footprint is allocation, GC
   barriers, and value primitives (§ The migration ABI) — a small, enumerable
   slice of the RTS. (One caveat the emission check must cover: a *nested*
   local function of `async` type inside the body gets its own scope binder
   and re-enables the async machinery — reachability is checked at emission,
   not assumed from the boundary type.)
4. **moc-generated code is already position-independent in linear memory.**
   Static data segments are emitted *passive* and materialized into heap
   blobs via `memory.init <segment-index>`; text/blob constants, function
   values, and all other constants are reached through the static object
   pool (`get_static_variable(index)`), never through baked addresses. Field
   and variant identity is a pure 31-bit hash of the label string
   (`src/mo_types/hash.ml`, documented in `design/Custom-Sections.md`), with
   per-object hash blobs scanned at runtime — no per-program field tables.
   The only whole-program immediates in ordinary code are three index spaces:
   **object-pool indices**, **function-table indices**, and **data-segment
   indices**. Those three are what object emission must make relocatable;
   nothing else in the data path is program-relative.

## Design

### The chain object (`.mco`)

One artifact per build, consumed by the next build. Append-mostly: frozen
entries are carried forward byte-identically; pending entries are (re)compiled.

Container layout (concrete encoding TBD; a custom-section-style TLV or CBOR —
it is a compiler-private format, versioned by a leading format number):

```
mco :=
  format_version
  abi_epoch                    -- the migration ABI generation this file targets
  entry*                       -- in chain (lexicographic label) order

entry :=
  label                        -- file basename, e.g. "20260814_101500"
  kind                         -- FROZEN | (reserved)
  dom_rng                      -- the migration function type, .most-fragment
                               -- syntax incl. any type decs it needs
                               -- (printed with ParseableStamps, reparsed with
                               -- parse_stab_sig machinery)
  code                         -- closed wasm object (see § Codegen and linking)
  source                       -- the full original source text (audit +
                               -- epoch-regeneration input; never compiled on
                               -- the normal path)
  source_hash                  -- SHA-256 of source, whitespace-normalized
  code_hash                    -- SHA-256 of code
  moc_version, flags_fingerprint  -- provenance: compiler and the codegen-
                               -- relevant flag set that produced `code`
```

Notes:

- `dom_rng` is deliberately *all* the type information the static pipeline
  needs (§ Static checking). It reuses the `.most` printer/parser rather than a
  new type serialization, so type identity across builds is judged by exactly
  the machinery that already judges cross-version compatibility.
- Embedding `source` costs little (sources are small), keeps the artifact
  self-describing, and is the regeneration input at ABI epoch bumps
  (§ Epochs and trimming). It is not used by `--check`, `--compile`, or any
  routine path.
- The `.mco` contains executable code and must be trusted like source. The
  platform stores it with the build artifacts it already stores per draft;
  `code_hash`/`source_hash` give integrity and provenance checks, not
  authentication — that remains the artifact store's job.

### Build flow

New flag:

```
--migration-objects <file.mco>      (requires --enhanced-migration <dir>)
```

Chain assembly (replaces the current sources-only `infer_migration_chain`):

1. Read entries from the `.mco` (if given) and migration modules from the
   chain directory. The **chain** is the union, ordered by label
   (`String.compare`, unchanged).
2. Per label, resolve which representation is authoritative:

   | in `.mco` | source present | deployed? (label ∈ `--stable-baseline` chain) | resolution |
   |---|---|---|---|
   | yes | no | — | frozen entry (the steady state) |
   | yes | yes, hash matches | — | frozen entry; source is vestigial, may be deleted |
   | yes | yes, hash differs | no | source wins: the migration is *pending*, still editable; recompile, replace entry |
   | yes | yes, hash differs | yes | **error** (M0270): a deployed migration was modified |
   | yes | yes, hash differs | unknown (no baseline) | **error** (M0270), conservatively; message says to pass `--stable-baseline` if the migration is genuinely pending |
   | no | yes | — | compile from source (today's path) |
   | no | no | label ∈ baseline chain | **error** (M0270): deployed migration missing from both sources and objects |

   Row 4 is worth savoring: the frozen-file guard, currently enforced with
   chmod bits and platform preflights, becomes a compiler-checked property.
3. Emit the new `.mco`: frozen entries carried forward **byte-identically**
   (code, hashes, provenance untouched — this is the property the whole design
   exists for), pending entries freshly compiled and appended/replaced.

Bootstrap: a build with `--enhanced-migration` and no `--migration-objects`
compiles all sources (today's behavior) and — under a new
`--emit-migration-objects <file.mco>` output flag — emits the first chain
object covering them. From then on the platform passes the previous build's
`.mco` and may delete any source whose label the deployed baseline contains.

Epoch guard: if the `.mco`'s `abi_epoch` does not match the compiling moc's
supported set, the build fails up front (M0271) with the regeneration
instructions — never a downstream linker error.

### Static checking without sources

Everything the frontend does with a chain entry today needs only its label and
function type:

- chain ordering and adjacency checks (`check_enhanced_migration_chain`,
  M0251/M0253-class): operate on `dom`/`rng` — supplied by `dom_rng`;
- `--stable-baseline` resume-point checking (M0254/M0267/M0169/M0170/M0216):
  operates on the chain fields and the actor's stable fields — unchanged;
- `.most` emission: `Multi`'s chain fields *are* label + function type —
  emitted identically whether an entry came from source or object. **The
  `.most` format, version 4.0.0, and every consumer of it (caffeine-db EM
  detection, CaLM, dfx) are untouched by this proposal.**

Frozen entries surface in the typing environment as opaque migration modules of
recorded type; no parsing, no import resolution, no typechecking of their
bodies ever happens again.

### Codegen and linking

**Driver side.** For a frozen entry, desugaring emits, in place of the direct
call to the compiled-in module's `migration` field, a new IR primitive
`FrozenMigration lab : Dom -> Rng`. Codegen compiles it to a plain function
call against an undefined symbol `mco_migration_<lab>`; `linkModule` resolves
it against the object's export, exactly as RTS imports resolve against
`mo-rts.wasm` exports today. Extract/merge/step glue around the call is
generated as today, against the freshly computed `mem_typ_k`.

**Object side.** A new object-emission mode compiles one migration module
closed:

- exports: `mco_migration_<lab>` (the function, vanilla-representation i64
  arguments/results — the internal calling convention, which becomes part of
  the ABI) and `mco_init_<lab>` (static initialization, below);
- imports: only migration-ABI symbols (§ next section); emission fails if
  anything outside the ABI is reachable — which, per the language fragment
  argument, cannot happen for valid migration code, so this check is an
  assertion, not a user-facing restriction;
- all internal symbols are namespaced by label, so multiple objects link into
  one module without collision;
- the three program-relative index spaces are made relocatable as follows
  (this is the substance of "object mode"; everything else is today's
  codegen):

**1. Object-pool indices.** Every constant (text, blob, bigint, boxed
words, floats, constant records/arrays, and function values) lives in the
static object pool, read via `get_static_variable(<immediate index>)` and
written once at module `start` by `initialize_root_array`. Two separately
compiled objects cannot share one index space, and the RTS enforces a
**strictly sequential initialization invariant** (`set_static_variable`
asserts `index == INITIALIZED_VARIABLES`; reads assert
`index < INITIALIZED_VARIABLES` — `rts/motoko-rts/src/gc/incremental/roots/
enhanced.rs`). Scheme: object mode routes every pool access through a
per-object immutable base global (`global.get $mco_pool_base_<lab>` + const),
and exports `mco_init_<lab>` running the object's allocation thunks in order.
The linker assigns bases, extends the main module's
`initialize_static_variables` count to the total, and **chains the object
inits into the module `start` sequence, eagerly, in base order, immediately
after the main module's own pool init** — satisfying the sequential
invariant. Lazy per-object init is *not* possible under the current RTS
asserts (an earlier draft of this document proposed it; it would trap in a
debug RTS and read `NULL_POINTER` in release). The cost of eager init —
applied migrations' constants are materialized on every upgrade — is small
and accepted; relaxing the RTS invariant to regain laziness is a possible
later optimization, not a dependency. (Fallback if base-global indirection
proves invasive: object mode disables the pool and materializes constants at
each use — semantically invisible, slower, acceptable for one-shot code.)

**2. Function-table indices.** Closures bake their table index as an i64
*data* immediate (`Closure.constant` / `add_fun_ptr`), and any
function-as-value in a migration (`Array.map`, a comparator into
`Map.fromIter`) allocates one — this is the norm, not an edge case. Data
immediates cannot be found by a linker traversal, so object mode must emit
them base-relative: `global.get $mco_table_base_<lab>` + const, with the
object's elem entries appended and the base set at link time. (The RTS link
already does `__table_base` patching, so the linker-side pattern exists.)

**3. Data-segment indices.** Object segments are passive, like all moc
segments; the `memory.init <segidx>` *instruction operand* is renamed by a
linker traversal, exactly analogous to how `Call` indices are renamed today.
This traversal does not currently exist (`rename_funcs`/`rename_globals`
have no `MemoryInit`/`DataDrop` cases — `linkModule.ml` says so explicitly:
"if Rust would also use passive data segments in future, the segment load
indices need to be renumbered") — it is new but mechanical work. Because moc
objects use passive segments materialized to the heap, they consume **none**
of the fixed 512 KiB static-data window below the persistent heap that
constrains the RTS link (`TooLargeDataSegments`) — object data lives on the
ordinary heap.

Statics are per-upgrade, reinitialized like all static variables under EOP;
nothing about their lifecycle changes.

**Linker restructuring.** The current `link` is a one-shot, two-module merge:
it requires the base to export `__heap_base`, requires the library to carry a
`dylink.0` section in the RTS's LLVM PIC dialect, and ends by stripping all
non-IC exports ("only sane if no additional files get linked in"). Migration
objects need a distinct, additional path: `link_objects : base -> (label *
object) list -> base` that (a) runs **before** the single RTS link and before
export stripping, (b) merges objects using the existing function/global
renaming plus the new segment-index traversal and base-global assignment,
(c) detects export collisions (today's `join_modules` concatenates exports
unchecked), and (d) leaves the objects' `rts.*` imports symbolic so the one
existing RTS link at the end resolves the union of main-module and object
imports. moc objects import nothing from the main module itself — migration
code calls only the RTS and its own internals; the main module calls *into*
objects — so there is no circular resolution and no `dylink.0`/PIC dialect
requirement on the object side.

**Duplication.** Each object carries its own tree-shaken copy of the library
code it uses; two objects (and the main module) may embed duplicate copies of,
say, core `Map` internals. This is semantically invisible — Motoko modules are
stateless (no `var` at module level) and the duplicated values are immutable —
and costs wasm size only. See Edge cases for the size discussion and Future
work for content-hash deduplication.

### The migration ABI

The imports a frozen object may make, frozen as a versioned interface
(`abi_epoch`). From the full `E.call_rts` catalog (~130 symbols), the fragment
reachable from migration code is, by category:

| category | representative symbols |
|---|---|
| allocation & GC | `alloc_words`, `alloc_array`, `alloc_blob`, `allocation_barrier`, `write_with_barrier`, `read_with_barrier` |
| arbitrary-precision ints | `bigint_add`, `bigint_mul`, …, `bigint_of_word64`, `bigint_to_word64_trap`, … |
| text & blob | `text_concat`, `text_compare`, `text_iter*`, `text_len`, `text_of_ptr_size`, `blob_of_text`, `utf8_valid`, `memcmp`, … |
| float math | `sin`, `cos`, `pow`, `exp`, `log`, `fmod`, `float_fmt`, … |
| stable regions | `region_new`, `region_load_*`, `region_store_*`, … (stable types include `Region`; these also touch the shadow-stack globals below) |
| mutable globals | `__stack_pointer` / `__frame_pointer` / `__stack_min` — the RTS shadow stack, reachable from `Region` operations; imported by objects, defined by the base module |
| conventions | vanilla i64 calling convention **including the leading dummy-closure i64 parameter** (local 0 of every local function; callers pass 0 for closed functions) and n×i64 multivalue returns; heap layout — object headers (2 words: tag, forwarding pointer), **the object-tag numbering** (duplicated between `compile_enhanced.ml`'s `int_of_tag` and `rts` `TAG_*` with *no automated cross-check today* — one must be added), the **MutBox indirection for `var` fields** (its removal is mused about in a codegen comment; doing so is an epoch bump), the BitTagged scalar/pointer discipline, per-object sorted field-hash blobs with the `field_lower_bound` scan-start convention; **the inlined GC barrier protocol** — `running_gc` fast-path tests and barrier call sequences are compiled *into* function bodies, so a change to the barrier discipline (not just the symbol list) is an epoch bump; trap semantics |

Two empirical calibrations from the audit (2024-08 → 2026-08): the symbols in
the categories above had **zero signature changes in two years** (the only
removal anywhere in the RTS import surface was `memcpy`, dropped by a
dependency-upgrade *chore* commit — exactly the accident the alias namespace
exists to absorb), while the **barrier protocol changed twice in three
years** — most recently #6296 (2026-08), which added a read barrier for weak
references. The ABI is a good bet *because* it is enforced, not because
nothing ever moves.

Explicitly *excluded and rejected at object emission* (reachability-checked
over the compiled body, not assumed from the boundary type):

- stabilization/destabilization, IDL/Candid (`idl_*`, serialization
  `leb128`), continuation tables, actor/system API,
  `get_migrations`/`set_migrations` (driver-side only). Most of this is
  already unreachable by the `NullCap` typing of migration functions; the
  reachability check exists for the residue — notably `to_candid`/
  `from_candid`/actor references (legal Motoko in a migration body, and they
  drag in the non-relocatable global Candid type table) and *nested local
  functions of `async` type*, which acquire their own scope binder and
  re-enable the async machinery despite the outer `NullCap`.
- **`weak T` in `Dom`/`Rng` (and weak-reference dereference in bodies) is
  banned at freeze time.** `weak` is a stable type, but its load path embeds
  the GC read-barrier protocol that #6296 just changed: an object frozen
  before such a change and linked after it would *silently* reintroduce the
  fixed unsoundness (premature collection of a resurrected weak target) — a
  miscompile, not a link error. Banning `weak` in migrations costs nothing
  real (a migration has no use for weakness) and removes the sharpest
  epoch-coupling in the design. If a use case ever appears, the alternative
  is strict epoch-bumping on every barrier change, which #6296 proves will
  actually fire.

Mechanism: `mo-rts` gains an `mco_abi_v1` export namespace — thin aliases of
the current internals. When internals change signature or semantics, the
aliases become shims; when a shim can no longer be honestly maintained (heap
layout break, GC barrier discipline change, word-size change), the **epoch**
increments and old objects are rejected at build time (M0271), never miscompiled.

The exact v1 symbol list is fixed during implementation by measuring
reachability over a corpus of real migrations (Caffeine has the corpus); the
category boundary above is the design commitment.

Enforcement is mechanical, which is the point: CI keeps golden `.mco` files
built by past moc releases and links + runs them under every candidate release
(§ Testing). "Old objects still link and pass their drun suites" is a far
crisper gate than "old sources still typecheck under whatever the frontend
became".

### Runtime semantics: unchanged, by argument

The emitted upgrade logic is behaviorally identical to today's because the
transformation is confined to *where the code of `migration` comes from*:

- the driver (step functions, applied-list checks, `ICStableRead` boundaries,
  registration order, trap-on-missing-field) is generated exactly as today;
- the frozen function's observable behavior is its compiled behavior — which
  is precisely what ran (or would have run) when it was frozen;
- fresh installs replay the full chain from `{}` as today — frozen entries
  execute like any other; converted projects' legacy adoption is untouched;
- the applied list, `.most` signatures, M0255/mixed-persistence traps, and the
  CaLM install protocol see no difference at all.

There is no new runtime state and no RTS behavioral change; the RTS change is
additive (the ABI alias namespace).

## Soundness

What must hold, and why it does:

**(S1) Frozen semantics are preserved verbatim.** A frozen entry's code bytes
are carried forward unchanged across builds (build-flow rule 3) and its
behavior depends only on those bytes, the heap layout, and the ABI symbols it
imports. Layout is the EOP commitment; ABI symbols are shimmed per epoch and
their behavioral stability is CI-enforced against golden objects; epoch
mismatch is a build error. So within an epoch, a frozen migration's behavior is
bit-for-bit the behavior at freeze time — a strictly stronger guarantee than
today's "same source text under a newer compiler".

**(S2) The glue remains correct as the chain grows.** All suffix-dependent
computation (`mem_typ_k`, extract/merge, resume checks) is regenerated per
build from the actor and the chain *types* — and the types of frozen entries
are recorded, immutable, and consumed by the same checker that consumes source
migrations' types. A frozen entry can never be silently retyped: `dom_rng` is
part of the byte-carried entry.

**(S3) No configuration silently diverges source from object.** The
authoritative-representation table makes every source/object disagreement
either a well-defined recompile (pending migration) or a hard error (deployed
migration modified or missing). The deployed/pending distinction is delegated
to `--stable-baseline`, the artifact that already encodes deployment truth;
absent a baseline the compiler errs conservative.

**(S4) The call boundary is exactly the previous internal boundary.** Today
the migration module is a top-level const `let`; the const analysis resolves
`module.migration` entirely at compile time and the call compiles to
`i64.const 0 (dummy closure); <args>; call $f` — a direct wasm call
(`compile_enhanced.ml`, the `Const.Fun` direct-call case). Replacing `$f`
with an imported symbol is shape-identical: same instruction sequence, same
convention, same heap values of `Dom`/`Rng`. Linking changes symbol
resolution, not the contract. `moc -r` ignores migration expressions
entirely; `moc -iR` *does* evaluate the chain initializer, so the
`FrozenMigration` prim carries an explicit "frozen migrations cannot be
interpreted" diagnostic there rather than falling through to the
unknown-prim trap (typechecking needs only `dom_rng` in every mode).

## Edge cases

**Chain assembly**

- *Source and object disagree*: covered by the resolution table (M0270 on the
  dangerous rows). Whitespace-normalized hashing avoids spurious mismatches on
  formatting-only churn during the pending window.
- *A new source sorts before an existing frozen entry*: error — history is
  append-only; a frozen entry can never gain a predecessor.
- *Entry order in `.mco` vs label order*: entries are validated to be sorted
  and duplicate-free on read; the file order is normative-redundant.
- *Two `.mco` files / merging lineages (forks)*: out of scope; one lineage,
  one chain object. A fork copies the parent's `.mco` like it copies sources.
- *`check-limit`*: a mops-level cap on **pending** migrations; frozen entries
  are by definition not pending, so semantics are unchanged and, pleasantly,
  the "one pending migration per build" rule becomes structurally visible
  (pending = has authoritative source).

**Compilation & linking**

- *Object-pool indices*: the base-global scheme above; top implementation
  risk, with the dynamic-constants fallback specced.
- *Symbol collisions*: label-namespacing; labels are unique in a chain by
  construction (file basenames in one directory).
- *Wasm size*: duplicated library closures across objects. Bounded in practice
  by (a) migrations importing little (self-containment discipline), (b) chain
  trimming keeping the frozen population finite (§ next section), (c) the IC's
  chunked install ceiling being far above realistic chain sizes. Content-hash
  dedup of identical function bodies is a compatible later optimization.
  Honest note: a long chain's wasm will be somewhat larger than today's
  whole-program-deduplicated equivalent; the `motoko:stable-types` 1 MiB
  custom-section ceiling (the ~700-migration cliff) is **unchanged** by this
  proposal — trimming, not objects, is the answer to unbounded chain length.
- *Flags drift*: an object compiled under codegen-relevant flags (GC flavor,
  sanity checks) different from the current build's — `flags_fingerprint`
  mismatch within an epoch is a build error with a regeneration hint, not a
  link attempt.
- *New wasm features in newer moc output*: forward-compatible; objects use the
  feature set of their freeze-time moc, the linked module's requirements are
  the union — both target the same IC.

**Toolchain paths**

- *`moc --check` / mops check*: works from `dom_rng` metadata; no object code
  touched; no linker involved.
- *`moc -r` and the IR interpreter*: cannot execute frozen code. `moc -r`
  never evaluates migration expressions (explicit in `mo_interpreter`); but
  `moc -iR` evaluates the desugared chain initializer, so `FrozenMigration`
  gets an explicit, friendly diagnostic in `interpret_ir` instead of the
  generic unknown-prim trap. Interpreter-based tests of a chain use sources
  (which the `.mco` embeds, if ever needed for a repro). This limitation is
  documented rather than engineered around.
- *`mo-doc`, IDE/LSP*: frozen entries have no source in the workspace to
  analyze; they appear only through the chain metadata. No change needed.

**Runtime**

- *Trap inside frozen migration*: identical to today — upgrade rolls back.
- *Instruction limits on long replays (fresh installs)*: identical to today —
  the replay executes the same migrations; per-step function splitting is
  driver-side and regenerated.
- *Reference identity / aliasing*: frozen code allocating duplicated immutable
  library constants cannot be observed — no identity operations on immutables.

**Platform**

- *`.mco` lost*: sources (if retained) or the embedded sources in any newer
  `.mco` regenerate it — but regeneration recompiles under the current
  toolchain, which is exactly the exposure this design retires; so the `.mco`
  must be stored with the same durability as the wasm artifacts (same store,
  same lifecycle). Fork/remix/zip flows carry it like they carry `mops.toml`.
- *Zip import/export*: the `.mco` travels in the archive; the import
  container's full-chain validation runs the standard build (metadata checks +
  link), no special-casing.

## Epochs and trimming

The residual, named honestly: an **epoch bump** — a heap-layout break, a GC
barrier-discipline change, a word-size change — invalidates every chain object
at once. This cannot be engineered away at the object level; it is the moment
the "frozen bytes" representation itself expires.

The answer is the division of labor between this design and chain trimming
(previously specced in `design/TruncatedMigrationChains.md`, absorbed here):

- **Chain objects solve the every-build problem**: routine mops bumps, mo:core
  releases, and ordinary moc releases can no longer touch frozen history.
- **Trimming solves the every-epoch problem**: before an epoch rolls out, the
  platform forces a live deploy and trims the chain at the deployed frontier
  (dropping prefix entries from sources *and* from the `.mco` — the deployed
  baseline's wasm, already stored, remains the bootstrap artifact for fresh
  installs of the trimmed lineage, exactly the existing converted-project
  `[base, target]` payload shape). The corpus needing regeneration at the
  epoch boundary is then near-empty; whatever remains is regenerated from the
  embedded sources in one controlled platform batch with canary CI — never in
  a user's build.

Trimming therefore stays in scope as an operational mechanism the platform
will need regardless, at whatever hardening level is warranted (at minimum,
today's ability to drop deployed prefixes; the base-guard hardening from the
superseded doc can be revisited independently if silent-skip protection for
trimmed chains is wanted).

## Compatibility and rollout

- **Formats**: `.most` unchanged (4.0.0, all consumers unaffected). `.mco` is
  new and compiler-private, versioned by `format_version` + `abi_epoch`.
- **Old moc, project with `.mco`**: older mocs don't know the flag; the
  platform's toolchain floor (`MIN_MOC_VERSION`) gates rollout as usual. If
  sources have been deleted, an old moc cannot build the project at all —
  loud, and prevented by the floor preceding any source deletion.
- **New moc, no `.mco`**: today's behavior, bit for bit. The feature is
  dormant until the flag is passed.
- **mops**: new keys in the canister's migrations table (platform-owned,
  protected like the rest of the table), e.g.
  `objects = "src/backend/migrations/chain.mco"`, translated to
  `--migration-objects` / `--emit-migration-objects`.
- **Rollout order**: moc release with object emission+consumption → platform
  stores `.mco` per draft and threads it through build containers → enable
  per-project: first build emits the bootstrap `.mco`, and after the next
  successful live deploy the platform deletes deployed sources (the compiler's
  M0270 rules then enforce what chmod used to). Agents simply stop having
  frozen files to see.

## Testing plan

Repo conventions apply (expectation-based tests, `make accept`; EM tests under
the `em-*` naming).

- **Round-trip**: emit `.mco`, rebuild consuming it, assert the linked wasm's
  behavior equals the whole-program build's on the full drun suite; assert
  frozen entry bytes are carried forward identically across builds.
- **Equivalence corpus**: for each existing `em-*`/`enhanced-migration-*`
  drun test, a mirrored variant where the already-deployed prefix comes from a
  `.mco` — same `.ok` expectations.
- **Assembly matrix**: one `fail/` (or drun-check) test per row of the
  authoritative-representation table, incl. the modified-deployed-migration
  M0270 and the no-baseline conservative error.
- **Statics**: objects with heavy constant usage (text literals, literal
  arrays, nested modules) exercising the base-global pool scheme; lazy-init
  ordering (applied entries' inits never run).
- **Duplication semantics**: two frozen entries embedding the same library
  code, plus the main actor using it too.
- **Cross-version CI (the ABI gate, load-bearing)**: a growing corpus of
  golden `.mco` files built by each released moc, linked and drun-executed by
  every candidate release. A red run here is the signal that a change needs an
  ABI shim — or an epoch.
- **Epoch guard**: fabricated epoch/format/flags mismatches produce M0271-class
  errors before any linker involvement.

## Implementation plan (revised after the 2026-08-26 code audit)

Phased, with an explicit go/no-go gate. Rough effort assumes one engineer who
knows the backend; the linker and codegen phases dominate.

**Phase 0 — design close-out (≈1–2 weeks).** Decide the open restrictions
(reject Candid/actor refs — settled above; ban `weak` in `Dom`/`Rng` —
settled above; Region in or out of ABI v1); enumerate the epoch definition
precisely (persistence `VERSION`, tag table, MutBox indirection, barrier
protocol, calling convention, ABI symbol list, `flags_fingerprint` fields);
write the `.mco` encoding. Output: this document at "accepted" status.

**Phase 1 — the spike (≈2–3 weeks). Go/no-go gate.** Hand-carry one trivial
and one collection-using migration through the whole story with minimal
hacks: object-mode compile (pool/table base-global indirection), a
hand-driven `link_objects` pass, RTS link, drun upgrade test. This
de-risks the only two items with real unknowns — the base-global statics
scheme against the RTS sequential-init invariant, and segment/table
renumbering — before any production code is written. If the pool scheme
fights the codegen structure, fall back to no-pool object mode and re-cost.

**Phase 2 — object emission (≈4–6 weeks).** New unit case in
`compile_enhanced.ml` (`LibU` is rejected today; `ProgU`/no-actor mode is the
closest precedent): closed compile of a library's `migration` export,
label-namespaced symbols, base-global relocation for pool and table indices,
export `mco_migration_<lab>`/`mco_init_<lab>`, ABI import allowlist, and the
emission-time reachability rejection (Candid, nested-async, `weak`).

**Phase 3 — linker (≈3–5 weeks).** `link_objects` as specced in § Codegen and
linking: multi-object merge before the RTS link and before export stripping;
`MemoryInit`/`DataDrop` index traversal; base-global assignment;
`start`-sequence stitching for eager pool init in base order; export-collision
detection.

**Phase 4 — pipeline & frontend (≈3–4 weeks).** `.mco` read/write; chain
assembly with the authoritative-resolution table (M0270/M0271); frozen
entries as opaque typed modules in the env; `FrozenMigration` IR prim +
driver emission + the `interpret_ir` diagnostic; flags
`--migration-objects`/`--emit-migration-objects`.

**Phase 5 — RTS (≈1–2 weeks, parallelizable).** `mco_abi_v1` alias
namespace; a static cross-check that `compile_enhanced.ml`'s `int_of_tag`
matches `rts` `TAG_*` (none exists today); optionally, relaxing the
sequential-init assert if lazy object init is ever wanted.

**Phase 6 — tests & CI (≈3–4 weeks, overlapping).** Equivalence corpus
(every existing `em-*` drun test mirrored with a frozen prefix); the
assembly-matrix `fail/` tests; statics/table/segment stress objects; and the
**behavioral** cross-version golden-`.mco` CI job — golden objects built by
each release, linked and drun-executed by every candidate, with the corpus
deliberately exercising float/text formatting (ABI symbols have had
*semantic* drift historically; a signature-only check would miss it).

Total: roughly one to one-and-a-half quarters of focused work after the
spike gate passes.

## Phase 1 spike: RESULT — GO (2026-08-26, branch `worktree-mco-spike`)

The spike specced above was executed end to end. **A migration compiled as a
separate wasm object by one moc invocation, merged binarily into an
independently compiled main EM build and RTS-linked, is behaviorally
byte-identical to the whole-program compile on pocket-ic — on both the
fresh-install path (full chain replay through the frozen migration) and the
upgrade path (frozen migration executed during upgrade).** The test migration
exercised all three relocatable index spaces: text literals (data segments +
object pool), a runtime-selected closure (`call_indirect` through relocated
table entries), and bigint/text RTS imports. Accepted `.ok` files for the
reference and frozen-object variants are identical (`spike/` on the branch).

What was built (spike quality, behind `--spike-*` flags): object-mode
compilation in `compile_enhanced.ml` (pool/table/segment offset relocation,
`mco_migration_<lab>`/`mco_init_<lab>` exports, no own memory/table/start,
mco-only export filtering), the `FrozenMigration` call substitution in
`desugar.ml`, `link_mco` in `linkModule.ml`, and `mo-ld -mco`. The offset
bootstrap ran as a three-pass script; production replaces it with link-time
base globals as specced.

Findings that feed back into this design:

1. **Import deduplication at merge is a correctness requirement, not an
   optimization.** The IC's instrumentation of stable-memory system calls
   rewrites `ic0.stable64_*` call sites and stubs the imports with traps
   (every EOP install calls `stable64_size` via the RTS's persistence-version
   check); it handles one import entry per name, so a duplicate `ic0.*`
   import introduced by naive import concatenation traps at the first
   install. `link_mco` resolves object imports against same-named main
   imports. Hard rule for the Edge cases section: *the merged module must
   contain at most one import entry per (module, name).*
2. **Three pre-existing custom-section decode bugs** broke any
   `moc -no-link` + `mo-ld` round-trip of real actor wasms (silent loss of
   `motoko:stable-types`, `motoko:compiler`, and
   `enhanced-orthogonal-persistence` sections; the last makes the test
   runner omit `wasm_memory_persistence` on upgrade → IC0504). Fixed on the
   spike branch; worth upstreaming immediately and independently
   (`customModuleDecode.ml`: `utf8` over-read, `motoko_section_content`
   folding from empty, `is_unknown` missing two section names).
3. **The eager, in-base-order pool init works exactly as respecced** against
   the RTS sequential-init invariant (`initialize_static_variables` extended
   count; object slice filled by `mco_init` chained right after the main pool
   init in `rts_start`). The RTS null-fills the root array, so reserved slots
   are GC-safe before their object init runs.
4. **The direct-call boundary held with zero friction** — the frozen call is
   `i64.const 0; <arg>; call $mco_migration_<lab>`, resolved by the linker,
   instruction-shape-identical to the whole-program build. Const-analysis even
   optimized away a first attempt at a closure test (a statically-known
   function passed as an argument compiles to a direct call — object-boundary
   tests must select functions at runtime).

Size measurement over a private corpus of the heaviest real production
migrations (worst compile-weight cases; corpus not in-tree), compiled
standalone as objects against the repo's core package:

| migration | source | object wasm |
|---|---|---|
| identity baseline (per-object floor) | 122 B | 8.9 KB |
| corpus #1 (11 mo:core imports, collection rebuild) | 59 KB | 86 KB |
| corpus #2 | 61 KB | 45 KB |
| corpus #3 | 39 KB | 32 KB |
| corpus #4 | 39 KB | 47 KB |

So a frozen object costs roughly 0.7–1.5× its source size, with a ~9 KB
floor — against production actor wasms of several MB, the worst measured
migration adds on the order of 1–2% per frozen entry. (Three further corpus
entries were skipped: the ad-hoc harness lacked their mops flag setup; they
compile fine in their projects.)

## Implementation audit and readiness (2026-08-26)

A three-track audit (linker capabilities; codegen call/memory model;
whole-program couplings) was run against the tree at `217f8bdca`. Verdict:
**the idea is sound and implementable — no hard blocker was found — but this
document required the corrections now folded in above, and implementation
should not start before the Phase 1 spike validates the statics scheme.**

What the audit *confirmed* (the load-bearing facts, all held):

- Field/variant identity is a pure content hash of the label — no
  per-program tables in ordinary data paths. This is the single fact the
  whole proposal stands on, and it holds by design (documented in
  `design/Custom-Sections.md`).
- Heap layout is an explicitly versioned boundary (`persistence.rs`
  `VERSION`), never bumped in the two years since EOP shipped; new
  persistent roots were added additively three times rather than breaking
  layout. Tags: zero renumberings in two years.
- Generics are compiled uniformly (`typ_binds` is discarded by codegen) — a
  closed object needs no per-instantiation code from outside.
- The `NullCap` typing of migration functions excludes the continuation
  table, the Candid global type table, and prelude-by-name internals by
  construction.
- The driver's call to `migration` is already a direct call with the dummy
  closure — the import-based replacement is instruction-shape-identical.
- `eq`/`show` IR passes generate their helpers *per compilation unit*
  ("keeps closed actors closed" — verbatim comment), so structural equality
  and `debug_show` inside a frozen object are self-contained.
- RTS import surface: zero signature changes to in-ABI-category symbols in
  two years; the one removal (`memcpy`, in a dependency chore) is precisely
  what the alias namespace absorbs.

What the audit *corrected* in this document:

1. The linker is not "just another input away" — it is a one-shot two-module
   merge with an RTS-specific PIC contract and a final export strip. The
   `link_objects` restructuring above is real, bounded work; the missing
   `MemoryInit`/`DataDrop` renumbering is called out in `linkModule.ml`'s own
   comments. Offsetting this: moc objects need no PIC dialect (no static
   addresses exist to relocate).
2. Lazy per-object pool init was unsound against the RTS's sequential-init
   asserts; the scheme is now eager, in base order, chained into `start`.
3. Function-table indices are baked as *data* immediates and functions-as-
   values are the norm in migration code — table relocation is mandatory and
   must be base-global-relative, not traversal-based.
4. The ABI gained: the dummy-closure leading parameter, the shadow-stack
   mutable globals (Region ops), the MutBox `var`-field indirection, the tag
   numbering (with a new CI cross-check), and — critically — the *inlined GC
   barrier protocol*.
5. One silent-miscompile class was found and designed away: `weak` values in
   migrations, whose read path embeds a barrier protocol that changed as
   recently as #6296 (2026-08-13). Banned in `Dom`/`Rng` at freeze time.
6. `moc -iR` does reach migration bodies; `FrozenMigration` needs its own
   diagnostic there.
7. Candid exclusion is a *user-facing, enforced* restriction (with the
   nested-async caveat), not an assertion.

Residual risks accepted going in: per-object duplication of library slices
and `share_code` helpers (size, bounded by trimming; § Q&A in review thread);
semantic drift of ABI symbols is caught only by the behavioral golden-corpus
CI, so that job is load-bearing, not optional; and epoch bumps remain the
platform-coordinated regeneration event described in § Epochs and trimming —
the barrier-protocol history (two changes in three years) says epochs are
real and the trim-then-regenerate motion must actually be rehearsed.

## Alternatives considered

- **Source sealing** (inline transitive imports at freeze time): removes the
  mops axis only; frozen sources still face every future frontend. Strictly
  dominated by objects except in implementation cost.
- **Truncation as the primary mechanism**: solves recompilation by shortening
  the chain, but sacrifices the long-chain-in-one-wasm property that
  enhanced migration exists for (fast-forward and fresh replay need the code).
  Retained as the epoch/pressure-relief mechanism, not the mainline.
- **The deployed wasm as the artifact** (export migration functions from the
  main wasm with a manifest section; next build extracts them from the
  previous build's wasm): philosophically ideal — the frozen thing is
  literally the deployed bits — but extraction from a fully linked,
  pool-assigned, dedup-shared module is substantially harder than consuming a
  purpose-built relocatable object, for no additional guarantee. The `.mco`
  *is* that manifest, shipped beside the wasm instead of inside it.
- **Per-migration object files** (`.mob` each) instead of one chain object:
  more filesystem surface for agents and zip flows to mishandle; the chain is
  the unit of meaning, so the chain is the unit of artifact.
- **Serialized typed AST/IR as the artifact**: dodges the ABI question but
  commits us to IR stability (never promised, historically churny) and still
  runs frozen code through the moving backend — semantic drift risk survives.
  Compiled bytes are the only representation with a bit-for-bit argument.

## Open questions

1. ABI v1 surface: exact symbol list after corpus measurement; do we admit
   `Region` operations from day one or reject regions in migrations first?
2. Statics: base-global relocation vs dynamic-constants fallback — decided by
   the spike (item 4).
3. Should `--emit-migration-objects` be default-on under
   `--enhanced-migration` (always produce the artifact, adopt it when the
   platform is ready) or opt-in until rollout completes?
4. Epoch policy: how many epochs does a moc release support simultaneously
   (proposal: current + previous, giving the platform one release cycle to run
   the trim-and-regenerate motion)?
5. Hash normalization: whitespace-stripped (matches the platform's existing
   deploy-gate comparison) vs exact bytes for `source_hash`.
