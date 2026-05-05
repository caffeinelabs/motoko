# Plan: Update Wasm_exts to WebAssembly/spec 2.0.2

## Motivation

The `src/wasm-exts/` module is a fork of the WebAssembly spec interpreter's
OCaml modules. It has diverged significantly — upstream now covers GC, SIMD,
exceptions, tail calls, and multi-memory. Motoko doesn't need all of these,
but staying current avoids bitrot and unblocks future features (e.g. Wasm
exceptions for Motoko try/catch, tail calls for async).

## Strategy: Selective Port

We don't need to support every upstream instruction. The approach:

1. **Import the new type algebra** — this is unavoidable since everything
   depends on `types.ml`
2. **Import the expanded AST** — take all new constructors, even if Motoko
   doesn't emit them. This keeps the AST compatible with upstream tooling
   and allows round-tripping through `wasm-opt` without losing information
3. **constTrack / linkModule**: add `shift_and_evict` stubs for new
   instructions (correct depth tracking, no constant propagation)
4. **Codegen**: only emit the subset Motoko actually needs — no GC instructions,
   no SIMD. New instructions adopted one-by-one as features require them
5. **Re-graft Motoko extensions** onto the new structures

## Files to Update

### Phase 1: Type foundations

| File | Upstream name | Work |
|---|---|---|
| `types.ml` | `types.ml` | Full rewrite. New: `numtype`, `vectype`, `reftype`, `heaptype` (14+ variants), `comptype`, `subtype`, `rectype`, `deftype`, `tagtype`, `localtype`, `instrtype`. Old flat `value_type` becomes `numtype` subset. |
| `values.ml` | `value.ml` (renamed) | Add `V128` for SIMD values. Keep our `op` parameterised type but add the vector case. Rename file or keep as `values.ml` with a note. |

### Phase 2: AST and operators

| File | Upstream name | Work |
|---|---|---|
| `ast.ml` | `ast.ml` | Expand `instr'` from ~25 to 80+ constructors. Key groups: GC (15), SIMD (30), exceptions (3), tail calls (3), multi-memory (memory ops gain `memoryidx`), table ops (8), `Select` gains `valtype list option`. `func'`/`global'` change to constructor form. `module_'` gains `tags`, `elem` type changes. |
| `operators.ml` | `mnemonics.ml` (renamed) | Replace with upstream. Used for pretty-printing, not semantics. |

### Phase 3: Motoko-specific re-grafts

| Extension | Location | Work |
|---|---|---|
| `CustomModule` | `customModule.ml` | Update `module_'` references for new fields (`tags`, new `elem` type). |
| `Meta`/DWARF | `ast.ml` additions | Re-add `Meta` instruction variant to expanded `instr'`. |
| `StableMemory*` ops | `ast.ml` additions | Re-add stable memory instruction variants. |
| Passive segments | `ast.ml` | Verify `segment_mode` still compatible or update. |
| Encode/Decode | `customModuleEncode.ml`, `customModuleDecode.ml` | Update for new instruction opcodes, multi-byte prefixes (0xFB for GC, 0xFD for SIMD, 0x08/0x09 for exceptions). |

### Phase 4: Consumers

| Consumer | Work |
|---|---|
| `linkModule.ml` | Update all `instr'` pattern matches. Multi-memory: `Load`/`Store` etc. gain `memoryidx` arg. `Select` gains optional type. New constructors need wildcard or explicit handling. |
| `constTrack.ml` | Add cases for new instructions in `step`. Most are `shift_and_evict` with the right delta. SIMD vec ops: net -1 (binary), 0 (unary). GC struct/array ops: varies. Table ops: varies. Can bail (`None`) initially for complex ones. |
| Codegen (`compile_*.ml`) | Only update instruction construction that changed shape (e.g. `Load`/`Store` with `memoryidx`, `Select` with type). Don't emit new instructions yet. |
| `mo_ld.ml` | Same as linkModule — pattern match updates. |

## Breaking Changes Requiring Care

### Multi-memory (high impact syntactically, but semantically no-op)
Every `Load`, `Store`, `MemorySize`, `MemoryGrow`, `MemoryFill`, `MemoryCopy`,
`MemoryInit` gains a `memoryidx` parameter. Motoko uses memory 0 everywhere,
so all call sites get `0l @@ no_region` as the memory index. Mechanical but
pervasive — grep for every `Load`/`Store` construction.

Note: the IC does **not** support multi-memory via Wasm instructions.
Stable memory is implemented as a second memory under the hood, but accessed
via system calls (`ic0.stable*`), not Wasm memory instructions. The IC
runtime rewrites these for performance. So `memoryidx` will always be 0 in
Motoko-generated code — the parameter is purely for AST compatibility with
the spec, not for actual multi-memory use.

### Select (low impact)
`Select` becomes `Select of valtype list option`. Motoko emits untyped
`Select` — pass `None`. Pattern matches add the `_` or `None`.

### func'/global' constructors (medium impact)
`{ ftype; locals; body }` becomes `Func { ftype; locals; body }` (or similar).
Search-and-replace across codegen.

### elem segments (medium impact)
`var list segment list` becomes typed `elem` with `const list`. Affects
linking and module construction.

## What We Do NOT Need (initially)

- **GC instructions**: Motoko has its own GC. No `StructNew`/`ArrayNew` etc.
- **SIMD instructions**: No vectorisation in Motoko codegen.
- **Exception instructions**: Future work (Motoko try/catch). Take the AST
  constructors but don't emit.
- ~~**Tail calls**: Future work (async optimisation). Same — take constructors,
  don't emit yet.~~ **Delivered** end-to-end on `gabor/wasm-exts-sync` —
  see § *Tail-call instructions (delivered)* below.

These exist in the AST for compatibility but the codegen doesn't touch them.

## Ordering and Dependencies

```
types.ml  →  values.ml  →  ast.ml  →  operators.ml
                                    ↓
                              customModule.ml
                              encode/decode
                                    ↓
                              linkModule.ml
                              constTrack.ml
                              codegen
```

`types.ml` must go first — everything depends on it.

## Tail-call instructions (delivered)

Implemented on branch `gabor/wasm-exts-sync` ahead of the broader 2.0.2 sync,
so the rest of that work can land later without re-touching this slice.

### Stack

| commit | layer | what |
| --- | --- | --- |
| `f2c69f3e7` | wasm-exts | `ReturnCall` / `ReturnCallIndirect` AST variants, smart constructors, encoder (opcodes `0x12`/`0x13`), decoder (illegal-list updated). |
| `a7ebc369d` | CLI | `--experimental-tailcalls` flag wired in `flags.ml` + `moc.ml`. |
| `dd2d8337a` | IR | New `prim` constructor `TailCallPrim of Type.typ list`; consumer arms in interpreter, type-checker, effect inference, async lowering, both backends merged via or-patterns; `arrange_ir.ml` renders it. |
| `ef2c7fb0c` | codegen | `compile_classical.ml` and `compile_enhanced.ml` direct-call arm: `is_tail` derived from prim, emits `ReturnCall <fi>` and skips `FakeMultiVal.load` (control never returns). |
| `4dbdb1712` | producer | `tailcall.ml` gains a third arm: in tail position, when the self-recursion loop-rewrite doesn't apply and `--experimental-tailcalls` is set, emit `TailCallPrim` instead of `CallPrim`. |
| `fb38c1b24` | bench | `test/bench/tailcall.mo` — Hutton/Bahr-style stack VM running `fak`. Mutually tail-recursive dispatcher (`step` → `opPush` → `step` → …), first-order, no closures. |

### Design notes

- **Pipeline order matters.** `tailcall_optimization` runs *after*
  `async_lowering` (`pipeline.ml:796-797`), so by the time the producer arm
  sees a `CallPrim`, awaitable/IC calls have already been desugared. The
  producer therefore cannot mistakenly tag a shared call as a tail call.
- **Codegen scope: direct calls only.** `TailCallPrim` is honoured in the
  `SR.Const Const.Fun (..., mk_fi, _)` arm (known function index → emit
  `ReturnCall <fi>`). Closure calls (`Type.Local` via `call_indirect`) and
  shared calls fall through to the regular path even if the prim is
  `TailCallPrim`. Extending to `return_call_indirect` is a follow-up if the
  cost/benefit ever justifies it.
- **Validation.** Wasm `return_call` requires the callee's wasm result type
  to match the enclosing function's. Motoko's all-multival-via-side-channel
  ABI gives every wasm function the result type `[]` (or a single `i64`),
  so the constraint holds trivially for direct calls between Motoko
  functions.
- **Self-recursion still loops.** The pre-existing self-tail-call → loop
  rewrite is strictly cheaper than `return_call` to self (no call mechanism
  at all), so the producer arm only fires when the loop rewrite *cannot*
  apply: cross-function tail calls, mutually-recursive cycles, or self-calls
  with mismatched type instantiation.

### Empirical: IC instruction-counter cost

Measured against `test/bench/tailcall.mo` (fak(10) ×1000), same compiler
tree, same EOP setting, only the flag differs:

| build | cycles |
| --- | --- |
| no `--experimental-tailcalls` | 26_052_088 |
| `--experimental-tailcalls` | 25_416_088 |
| **delta** | **−636_000 (TCO cheaper, ~2.5%)** |

The diff between the two wasm outputs is exactly 26 bytes — every one of
them flipping `0x10` (`Call`) to `0x12` (`ReturnCall`) at one of 26
static call sites in the dispatcher. Same length, same operand encoding,
no other deltas. The ~600k cycle reduction matches the prediction from
`instruction_to_cost` (`Call=5 → ReturnCall=3`, ~170k dynamic dispatches
per bench × 2 cycles ≈ 340k–680k saved).

**Implication:** the IC's metering does what `instruction_to_cost` says.
TCO is mildly *cheaper* on the cycle axis; not a perf regression. But
the more interesting property is **bounded stack** — code that would
otherwise blow the wasm stack (VM dispatchers, CPS-transformed programs,
deep mutual recursion) runs in constant frame depth. Pitch
`--experimental-tailcalls` as a *bounded-stack opt-in*, with the ~2.5%
cycle reduction as a small bonus.

(Earlier read of this section claimed a "65% regression"; that was a
methodology error — it compared a stale committed `.ok` from before
the producer + backend landed against the with-flag run on the current
tree, conflating intervening codegen drift with the flag effect. The
honest same-tree comparison is the table above.)

### Future work (not part of this slice)

- **Source-level annotation `(with tailcall)`** — the most useful next
  step; supersedes the "producer heuristics" idea below by giving the
  user direct control. See § *Source-level annotation* below.
- **Producer heuristics.** Today the producer emits `TailCallPrim` for
  every tail-positioned non-self `CallPrim` under the flag. Smarter
  heuristics (e.g. limit to functions known to be in a mutual-recursion
  cycle) would help, but explicit annotation is strictly better — it
  documents intent and verifies it.
- **`return_call_indirect` for computed tail-calls.** See §
  *`return_call_indirect` — enabling computed tail-calls* below. Today
  closure-typed tail calls silently degrade to non-tail
  `call_indirect`; emitting the indirect tail-call form covers the
  complement of the direct-call case (the callee chosen at runtime,
  not statically known).

## Source-level annotation: `(with tailcall)` — *proposed*

### Motivation

Per-call granularity matches the actual *property* `return_call`
provides — bounded stack at a metered cost — which is a property of
individual recursive call sites, not of whole modules. The
`--experimental-tailcalls` flag is a coarse module-wide knob; the
annotation lets users pay the IC's cycle premium *only* where stack
safety matters.

Equally valuable as a **declarative diagnostic**: the compiler can
verify the annotated call is in tail position and warn otherwise.
Same role as Scala's `@tailrec` or Haskell's `{-# RULES #-}` —
intent meets verification. Today users have no way to express "I
*meant* this to be a tail call"; with this annotation, that intent
is checked at compile time.

**Near-term value:** a workable opt-in for the `core` library's
recursive algorithms (list/tree traversals, fold/reduce on deep
structures) where stack safety beats cycles, while we wait for the
IC's per-instruction cost of `return_call` to come down.

### Surface syntax

The `parenthetical` non-terminal is already in the grammar
(`parser.mly`, used today for `(with cycles = …)`,
`(with migration = …)`, etc.). Two spellings reduce to the same IR:

- **Explicit:** `(with tailcall = true) f(args)`
- **Pun:** `(with tailcall) f(args)` — desugars to
  `(with tailcall = tailcall)` via record punning, requires
  `let tailcall = true` (or import `tailcall` from a constant-bearing
  module) in scope.

The pun form needs no grammar change — it's the punning shorthand
the language already supports for record fields. The cost is just a
new attribute key in the typechecker's table of recognised
`with`-fields.

### Typecheck constraint

The `tailcall` field must resolve to a **compile-time-known `Bool`**.
Rationale: codegen needs to decide statically whether to emit `Call`
or `ReturnCall` — it cannot dispatch on a runtime value. This
contrasts with `(with cycles = expr)`, which accepts a runtime `Nat`
because the value is passed at the call boundary.

Const-folding already covers literals, identifiers bound to
literals, and trivial expressions; rejecting non-const values is a
small one-shot check at the typecheck of the parenthetical. Error
message: *"`tailcall` must be a compile-time-known Bool (literal or
constant binding)"*.

### Lowering

When the parenthetical is present and resolves to `true`, lower the
call straight to `PrimE (TailCallPrim insts, [e1; e2])` — bypassing
the flag-gated producer arm in `tailcall.ml`. The codegen path
already in place (commit `ef2c7fb0c`) does the rest. When `false`
(or absent), lower to `CallPrim` as today.

### Open design questions

1. **Flag interaction.** Three plausible policies:
   - *Annotation always wins* — flag becomes redundant for new code,
     kept only as a coarse migration knob. Cleanest end-state.
   - *Annotation only with flag* — keeps `--experimental-tailcalls`
     as a gate during the experimental window. Conservative.
   - *Annotation overrides default* — explicit `tailcall = false`
     blocks producer-arm rewrites even with flag on, useful for
     proving "this call site is intentionally not a tail call."

2. **Tail-position warning: where to run the analysis.** Tail
   position is well-defined but subtle (early returns inside switch
   arms, both branches of an `if`, etc.). The producer in
   `tailcall.ml` already does this analysis correctly. Two designs:
   - *Frontend approximation* — warn on obvious non-tail (call not
     last in its enclosing block); let `tailcall.ml` confirm and emit
     the precise warning later.
   - *Defer to IR* — the annotation lowers to `TailCallPrim`
     unconditionally; `tailcall.ml` (or a new IR pass) checks tail
     position and warns if the annotation was misplaced. Avoids
     duplicating the analysis.

3. **Self-recursion.** A `(with tailcall) self(args)` annotation:
   honour by emitting `TailCallPrim` (cycle cost, no loop), or keep
   the existing self-tail → loop rewrite (cheaper)? The user
   expressed *intent* (tail-call), and the loop rewrite satisfies it
   semantically (bounded stack, no overflow). Suggested behaviour:
   keep the loop, emit a *note* (not warning) that a cheaper form
   was applied.

4. **Indirect calls.** Today codegen specialises only direct calls.
   For closure calls (`Type.Local` via `call_indirect`), should the
   annotation force `return_call_indirect`? Requires extending
   `Closure.call_closure`. Suggested interim behaviour: emit warning
   *"annotation honoured only for direct calls; closure-call tail
   support pending"*.

## `return_call_indirect` — enabling computed tail-calls — *proposed*

### Motivation

The wasm-exts AST/encoder/decoder for `ReturnCallIndirect` already exist
(commit `f2c69f3e7`), but no codegen path emits the instruction. The
natural use case is **computed tail-calls**: tail-positioned invocations
where the callee is selected *at runtime*, not statically known.

Direct `return_call` (delivered) handles the case where the callee is
known at compile time — the existing bench's `step → opPush → step`
pattern is the canonical example, and the codegen specialisation lives
in the `SR.Const Const.Fun (..., mk_fi, _)` arm.

Computed tail-calls are the complementary case. Examples:

- VM dispatchers that **index into a table of opcode handlers** and
  tail-call the looked-up handler (rather than `switch`-ing on the
  opcode tag and tail-calling a static function).
- **CPS-transformed code** where the next continuation is a closure
  value computed at runtime.
- **Trampoline-free dynamic dispatch** in interpreters, combinator
  libraries, and effect handlers.

Today, when a `TailCallPrim` reaches the closure-call arm
(`_, Type.Local` → `Closure.call_closure`), it silently degrades to
non-tail `call_indirect`. The user (or producer) requested a tail
call, but the stack-bounded property is lost.

### What changes

- **`Closure.call_closure`**: add an `is_tail` parameter (or paired
  `tail_call_closure`).
- **Inside the closure-call sequence**: replace
  `G.i (CallIndirect (table_idx, type_idx))` with conditional
  `ReturnCallIndirect` emission, gated on `is_tail`.
- **Trailing `FakeMultiVal.load`**: omit when tail-calling, same logic
  as the direct case (control never returns).
- Both `compile_classical.ml` and `compile_enhanced.ml` need symmetric
  treatment; the closure-call helper module is shared in spirit, and
  may need symmetric edits per backend.

### Wasm validation gotcha

`return_call_indirect` requires the type-table entry referenced by the
indirect call to have a *result* type that matches the enclosing
function's. Motoko's all-multival-via-side-channel ABI gives every
wasm function the result type `[]` (or a single `i64`/`i32` for
arity-1 returns), so the constraint should hold in practice — but
it's a *precondition the codegen must enforce*, not an invariant
we get for free. Worth a check at emission time, with a clear error
if violated, rather than a runtime trap.

### Bench coverage

`test/bench/tailcall.mo`'s dispatcher is a `switch` over the opcode
variant and tail-calls top-level non-capturing functions, so 100% of
its tail-call sites are *direct*. To exercise computed tail-calls,
add a sibling bench — say `tailcall-computed.mo` — that stores
opcode handlers as closures in a record or array (one entry per
opcode) and dispatches through closure invocation. The Bahr/Hutton
calculation supports both shapes; only the *Motoko encoding* of the
dispatcher decides whether the wasm-level call is direct or indirect.

That makes the comparison clean: same VM, same number of dispatched
opcodes, different wasm-level call shape. Differences in cycle count
between the two benches isolate the indirect-call premium.

### Cycle-cost expectation

On the IC, `call_indirect` is already meaningfully pricier than `call`
(the extra dispatch-table indirection has a per-instruction cost
multiplier). The *relative* tail-call premium of `return_call_indirect`
over `call_indirect` may be smaller (the per-instruction baseline is
already higher), the same, or larger — not predictable from first
principles. Worth measuring once the codegen path is in place; the
result informs whether `(with tailcall)` on closure calls is a
sensible recommendation for the `mo:core` library or only for niche
deep-recursion cases.

### Interaction with `(with tailcall)` annotation

Once this work lands, the annotation honours both direct and indirect
calls uniformly. The interim warning sketched in the annotation
section's open question 4 — *"annotation honoured only for direct
calls; closure-call tail support pending"* — becomes unnecessary and
should be dropped at the same time.

## Risks

- **Upstream keeps moving**: Pin to a specific spec commit (tag `wasm-2.0`
  or the release commit for 2.0.2)
- **Encode/decode complexity**: Multi-byte prefixes for GC/SIMD opcodes
  add complexity to the binary format handlers
- **Test coverage**: Many tests match on compiler stderr/stdout — instruction
  format changes may require baseline updates
- **wasm-opt compatibility**: Ensure the version of `wasm-opt` in nixpkgs
  understands the new instruction set

## Estimate

| Phase | Effort |
|---|---|
| Phase 1 (types + values) | 2–3 days |
| Phase 2 (AST + operators) | 3–5 days |
| Phase 3 (re-grafts) | 2–3 days |
| Phase 4 (consumers) | 3–5 days |
| Testing + baseline updates | 2–3 days |
| **Total** | **~2–3 weeks** |

Can be done incrementally — Phase 1+2 as one PR, Phase 3+4 as another.

## Design Opportunity: GADT Value Types

The 2.0.2 update is a natural moment to consider making the `op`/`value` type
a GADT with a phantom type parameter distinguishing scalar from float values:

```ocaml
type scalar
type float_

type _ value =
  | I32 : Int32.t -> scalar value
  | I64 : Int64.t -> scalar value
  | F32 : Float32.t -> float_ value
  | F64 : Float64.t -> float_ value
```

**Benefits:**
- `constTrack.ml`'s `const_val` type becomes `scalar value` — no separate type needed
- Parameterised modules over the scalar type become natural: `IntOps` with
  `type t` + `val extract : scalar value -> t option` — the type parameter
  threads through, no runtime tag dispatch, no constructor ambiguity
- I32/I64 handler duplication in constTrack (Binary, Compare, Test) could be
  collapsed into a single parameterised handler
- Functions that only accept scalar values (e.g. `to_const_val`) become
  type-safe: float cases ruled out statically, no catch-all needed

**Costs:**
- `value` (holding any variant) needs existential wrapping:
  `type any_value = Value : _ value -> any_value`
- Every pattern match on heterogeneous collections (LRU entries, value lists)
  must unwrap the existential
- All consumers across the compiler need updating (same blast radius as the
  2.0.2 update itself — so do it together, not separately)

**Interaction with constTrack's `FromLocal`:**
`FromLocal` tracks unknown values from locals — it's not a Wasm value at all.
With the GADT approach, `FromLocal` would be a separate variant in a
constTrack-local sum type that wraps `scalar value`:
```ocaml
type tracked = Known of scalar value | FromLocal of Int32.t
```
The LRU holds `tracked`, not raw values. This cleanly separates "Wasm values
we know" from "analysis metadata."

See also: abstract-interpreter.md § Design Review Findings for the
deduplication motivation.
