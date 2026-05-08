# Tail Recursion Modulo Constructor (TRMC)

Plan to extend the `--experimental-tailcalls` pass (PR #6043, branch `gabor/wasm-exts-sync`) with destination-passing rewrites that recover tail-call status for recursive functions whose result flows into a freshly constructed heap value.

## Motivation

The naïve `map` over `?(<head>, <tail>)`:

```motoko
type List<T> = ?(T, List<T>);

func map<A, B>(xs : List<A>, f : A -> B) : List<B> =
  switch xs {
    case null         null;
    case (?(h, t))    ?(f(h), map<A, B>(t, f));
  };
```

is *almost* tail-recursive: the recursive call is inside `OptPrim (TupPrim _ _)`, the second slot of a fresh heap pair. Without rewriting, neither the existing self-tail-loop transform nor `return_call` applies.

OCaml ships this as `[@tail_mod_cons]` (Filliâtre & Pottier). The transformation builds the result list bottom-up at allocation sites and threads the *not-yet-filled* slot to the recursive call, which closes it on its way to the next iteration.

## IR shape we are matching

After desugaring, the recursive arm of `map` is:

```
PrimE OptPrim
  (PrimE TupPrim
    (PrimE (CallPrim) (VarE f) (VarE h))            -- (B) head
    (PrimE (CallPrim A B) (VarE map) (TupPrim t f)) -- (List<B>) tail = recursive call
  )
```

Crucial observations:

- `OptP` / `OptPrim` is operationally the identity on heap-allocated values (tuples, records, arrays, objects, …). The `?(B, List<B>)` value *is* the tuple pointer, with `null` as a distinguished sentinel. So `?(h, t)` matching desugars to `OptP (TupP h t)` but at runtime is just the tuple-projection pair. Both reads and writes degenerate to working on the underlying pair.
- The recursive call sits in a fixed slot of a freshly-allocated tuple. Once the tuple is allocated, the slot is the only place its eventual value needs to land.

So the candidate constructors that put a self-call into "tail position modulo allocation" are at minimum:

- `PrimE TupPrim …` (one field is the self-call)
- `PrimE OptPrim (PrimE TupPrim …)` (the `Opt` is free)
- `PrimE OptPrim (NewObjE …)` and `NewObjE …` (a record field is the self-call)
- `PrimE ArrayPrim …` (one element is the self-call) — likely lower priority, less common

## Translation

Source:

```
func map<A, B>(xs : List<A>, f : A -> B) : List<B> =
  switch xs {
    case null         null
    case (?(h, t))    ?(f h, map<A, B>(t, f))
  }
```

Wrapper + tail-recursive *worker* (`map'`), well-typed throughout, no closure captures:

```
func map<A, B>(xs : List<A>, f : A -> B) : List<B> =
  switch xs {
    case null         null
    case (?(h, t)) ->
      let root : (B, List<A>) = (f h, t);            // bullet := input tail
      map'<A, B>(root, f);                            // ordinary call, returns ()
      (root : List<B>)                                // typed nop (heap-pointer cast)
  }

// Local, no captures — codegen lifts to a top-level function (no closure cell)
func map'<A, B>(parent : (B, List<A>), f : A -> B) : () =
  switch parent.1 {                                   // ordinary projection
    case null         ()                              // bullet already null
    case (?(h, t)) ->
      let cell : (B, List<A>) = (f h, t);             // new bullet := next input tail
      StorePrim (parent.1, ?cell);                    // close previous output cell
                                                      //   T = List<A>, T' = ?(B, List<A>)
                                                      //   (OptPrim is a nop)
      return_call map'<A, B>(cell, f)                 // tail call → return_call
  }
```

### Why `parent.1` initialised to the input tail

Initialising the bullet to `t` (the input tail at this position) makes the slot a well-typed `List<A>` pointer that the GC traces normally. The recursive callee reads its input from there and overwrites with its output. One field in the heap subsumes both "destination slot" and "input list cursor" — `map'`'s arity stays at 2.

### Why no closure capture

`map'` takes `parent` and `f` as explicit arguments. With no free variables, the function has an empty environment; codegen can lift it to a top-level static function (the simpler case of the static-closure work, PR #5964). No per-call closure allocation.

Capturing `root` to make `map'` return it from the base case (so the wrapper could `return_call`) was considered and rejected: the closure cell would cost a heap allocation per outer `map` call. Instead, the wrapper holds `root` in its own local across an *ordinary* call to `map'` — one stack frame stays alive for the whole operation, which is fine.

### One typed nop cast site

`StorePrim` is typed flexibly (`T` for the load slot, `T'` for the new value — see below), so the worker body needs no explicit cast at the store. The only cast in the rewrite is at the wrapper tail:

- `(root : List<B>)` — `(B, List<A>) → List<B>`; both are `?(_, _)` heap pairs at runtime. Rides on `OptPrim`'s typed-identity property; no allocation, no instruction.

So we may not need a new `UnsafeCast` IR node at all.

## IR primitive: `StorePrim`

One new primitive. Shape:

```
PrimE (StorePrim, [load_expr; new_value])
```

where `load_expr` is *syntactically* a load expression — for v0, `PrimE (ProjPrim n, [obj])`. The first child of `StorePrim` looks like the load that would have read the slot; the codegen contract is to flip that trailing load into a store of `new_value`.

### Type rule

```
load_expr : T      new_value : T'
—————————————————————————————————
       StorePrim (…) : ()
```

`T` and `T'` are independent. The IR primitive is intentionally lax: at runtime, all heap-pointer types share the same wasm-level representation (`i32`), and the constructor-spine rewrite legitimately stores values of type `T'` into a slot statically typed `T` (e.g., `?(B, List<A>)` into a slot of `List<A>`). Type-narrowing the rule would force ceremonial casts in the rewriter that buy nothing at runtime. Soundness comes from the rewrite predicate (the spine matches a constructor whose slot we own), not from `T = T'`.

### Codegen contract (non-incremental GC)

For `PrimE (StorePrim, [ProjE obj n; v])` the backend retags `obj.note.typ` to `[var Any]` (tuples and mutable arrays share heap layout) and dispatches via the existing `AssignE (IdxLE _ _) _` codegen path. No IR-level `CastPrim` is synthesised — `check_ir` does not re-run on backend-internal IR, so the note-only retag is sufficient and avoids a wasted node.

Concretely the address-store sequence emitted by `compile_lexp`'s `IdxLE` arm is:

1. Emit `obj`'s value-producing code (push base pointer).
2. Emit address-of-slot-`n` math (the existing `Arr.idx_bigint` path, with the index as a `Construct.natE n`).
3. Emit `v`'s value-producing code.
4. Emit either `i64.store` or `call $write_with_barrier`, selected by the existing `running_gc` runtime check.

Two consequences land for free:

- **Incremental GC barrier** — the `running_gc` selector means we get the barriered store under incremental GC without writing any new code; (3) below for v0 is therefore *not* a "non-incremental only" restriction in practice.
- **Bounds check** — the `IdxLE` path traps on out-of-bounds. For our compile-time-constant slot index this is unreachable, but the check is emitted. v1 fast-path elides it.

Generalisation to `IdxPrim` (arrays) and `DotPrim` (records) as the first child of `StorePrim` is mechanical via the same dispatch table.

Future-flexibility note: should a different IR shape turn out cleaner — e.g. exposing `ProjLE` as a real `lexp'` case and letting the rewriter emit `AssignE (ProjLE _) _` directly — the change is local. `StorePrim` is intentionally a thin escape-hatch primitive so the alternative remains open.

### Codegen contract (incremental GC) — punted

Incremental needs a barriered store: follow `obj[0]` (forwarding pointer) before computing slot offset. The same `StorePrim` IR shape works; the backend chooses barriered vs. raw based on RTS variant. This step assumes non-incremental.

## GC analysis (recap, condensed)

For the v0 (non-incremental) target:

- Bullet is the `parent.1` slot inside a properly typed `(B, List<A>)` heap pair. GC traces it as a `List<A>` field.
- Between the producer's set of `cell.1` (at allocation) and the consumer's read of `parent.1` in the next iteration, no allocation runs — `return_call` doesn't allocate. So the slot's value is stable across the call boundary.
- All loads/stores raw; no barriers needed.

Incremental GC details (forwarding-pointer indirection for the store, raw load for the read) are deferred to a follow-up commit. The `StorePrim` IR is reusable; only the codegen rule changes.

## Recognition

Hook into the self-call detector that `--experimental-tailcalls` uses to identify self-recursive calls eligible for `return_call`. Today it accepts a self-call in tail position. Extend it to also accept a self-call sitting under a *modulo-constructor* spine:

```
modulo_constructor_spine ::=
  | TupPrim  e1 … self_call … en
  | OptPrim  (modulo_constructor_spine)
  | NewObjE  fields_with_one_self_call_in_a_field         -- v1
  | ArrayPrim e1 … self_call … en                          -- maybe later
```

The detector returns `(spine_constructor_chain, slot_index)` — the chain of allocators wrapping the self-call and the index of the slot the call's result occupies in the innermost allocator.

Constraints on accepting the rewrite:

- Exactly one self-call in the spine. (Multiple recursive calls inside one constructor is a different problem — fork/join, not TRMC.)
- The self-call's other arguments must not depend on values computed *after* the constructor allocation site, since we're hoisting the allocation before the recursive call. (For `?(f h, map(t, f))`, `f h` and `t, f` are all available before allocation — fine.)
- The function's other tail positions (the `null` arm in our example) must produce a value the bullet can be pre-initialised to. For `null`, this is trivially `null`. For non-`null` base cases, we may need a more general bullet-init scheme, or punt for v0.

**Status:** detector implemented and committed (`191754d6a`); when `--experimental-tailcalls` is set, candidates are reported via `eprintf` to stderr. No rewrite yet.

## Synthesis

For each accepted function `f`, emit:

1. A **wrapper** `f` retaining the original signature. On non-recursive arms, evaluates as before. On the modulo-constructor arm: allocates the root cell with the bullet pre-set to the input tail, calls the worker (ordinary call), returns `(root : ResultTy)` — a typed nop.
2. A **worker** `f'`, also a `LetD` bound to a fresh `Local` `FuncE` (sibling of the rewritten wrapper, no captures), taking `(parent, …other_args)`. Reads its real "input list" via `parent.<slot_index>`. On the modulo-constructor arm: allocates the new cell, emits `StorePrim (parent.<slot_index>, cell)`, `return_call`s itself with the new cell as parent. On the base arm: the bullet is already null (set at root allocation), so no work; returns `()`.

The worker's recursive call is in genuine tail position — the existing `--experimental-tailcalls` arm in `tailcall.ml` (line 104) tags it as `TailCallPrim`, which lowers to wasm `return_call`. No new tailcall machinery beyond the spine rewrite + worker extraction.

### Code shape after rewrite

Conceptually the IR `LetD … FuncE` for `f` becomes a small block of two `LetD`s: one for the original `f` (now a wrapper body), one for the synthetic `f'` (the worker). Both are visible at the same scope; `f'` is referenced by name from `f`'s body and from its own tail call.

### Cost

Per outer `f` call:
- 1 stack frame (wrapper, alive across worker call).
- 1 heap alloc (root cell).
- 1 ordinary call into the worker.

Per element processed by the worker:
- 1 heap alloc (cell).
- 1 raw store (`StorePrim`).
- 1 `return_call` setup.

No closures, no `call_indirect`, no GC barriers (in v0).

## Open questions

- **Multi-arity constructors / slot index ≠ 1.** Generalise the bullet to "initialise slot `S` to the corresponding input field; pass parent + `S`". The `map` case (`S = 1`, init = `t`) is the simplest; other shapes (e.g., `Cons`-like records, n-tuples) have analogous initialisations.
- **Multiple TRMC sites in the same body.** If `f` has two recursive arms each in modulo-constructor position, the worker has both rewritten arms; no fundamental issue, just bookkeeping.
- **Type-checking the worker.** `parent` has type `(B, List<A>)` — well-typed in IR. The flexible `StorePrim` type rule (`T` and `T'` independent) absorbs the slot/value type mismatch; the only cast (`root : List<B>` at the wrapper tail) sits on `OptPrim`'s typed-identity property. Whether `check_ir` already accepts that or needs a small extension is the only typing question for v0.
- **Interaction with `--experimental-tailcalls` outer flag.** TRMC stays implied by `--experimental-tailcalls` rather than gated on a separate flag, to keep the surface small. Worth confirming in PR review.
- **Mutual recursion modulo constructor.** Out of scope for v0; the `return_call_indirect` slice already in `gabor/wasm-exts-sync` is the prerequisite.
- **Counter-examples / soundness.** What happens if `f` captures the bullet and stores it elsewhere (e.g., logging) — should be ruled out by the "exactly one occurrence in the spine, no escape" constraint, but we want a clear predicate.
- **Incremental GC barrier policy.** Deferred. The IR shape (`StorePrim`) is unchanged; only the codegen lowering rule for `StorePrim` differs (barriered store via `parent[0]` indirection).
- **Worker's unit return / wrapper-side `drop`.** The worker's wasm signature is `(result i64)` (Motoko's Local-function calling convention picks `SR.Vanilla` for the return slot regardless of source-level type), so the wrapper has to `drop` after `call $$map'/0` and the worker's base-case arms still emit `i64.const 0`. A `Tup()` / `SR.UnboxedTuple 0` regime would map to wasm `(result)` and erase the drop entirely. Snag: wasm `return_call`'s callee-return-type must match the caller-return-type, so a no-result worker breaks its own self-tail-call against an i64-returning wrapper. Two ways out — (a) make the worker return `List<B>` (e.g., return `parent` cast) so the chain is uniformly i64-returning and the wrapper-side `drop` becomes a useful pass-through (and possibly upgrades the wrapper itself to a `return_call`); or (b) abandon `return_call` inside the worker for a plain self-call + tail-loop. (a) is the appealing one but reintroduces "what flows back is the *last* cell, not `root`" — needs a closure capture or root-threading we deliberately avoided. v1 codegen optimisation territory; the `drop` is one wrapper-side instruction, not per-iteration, so it amortises to zero in the hot path.
- **`call $running_gc` on the per-store hot path** (not TRMC-specific, but `Tagged.write_with_barrier`'s outer guard is exercised by every TRMC iteration). `running_gc` is one i32 load + comparison wrapped in a Rust `extern "C"` function — wasm-call overhead dominates the actual check on every heap-pointer store. Better design: the **codegen** defines a wasm `(global $gc_running i32 (i32.const 0))` (default 0 = `Phase::Pause` = not running, so zero-init is correct out of the box) and exports a `set_gc_running(state: i32)` function for the RTS to call at phase transitions. Mutator code reads via `global.get $gc_running` — one instruction, no call, no indirection. RTS-wide win, applies to every barriered store; orthogonal to TRMC's branch but worth filing as a standalone issue because TRMC amplifies its visibility (the worker's hot loop hits this guard once per element).
- **Typed `if (param i64 i64)` for the barrier branch — and no locals.** Same hot path. The current `Tagged.write_with_barrier` spills `write_location` and `written_value` into locals *only* so each arm of the `running_gc` `if/else` can re-push them. Using the multi-value typed `if (param i64 i64) ... end`, the args ride the operand stack across `call $running_gc` (which takes no args, so the stack underneath is preserved) and flow directly into whichever arm runs. The locals disappear entirely:
  ```
  ;; stack already has [write_location, written_value]
  call $running_gc                      ;; pushes bool, leaves loc+val below
  if (param i64 i64)                    ;; pops bool; loc+val flow into the arm
    call $write_with_barrier            ;; consumes loc+val
  else
    i64.store                           ;; consumes loc+val
  end
  ```
  Saves two `local.get`s per store site *and* both wasm locals (`written_value`, `write_location`) per call site, smaller wasm, one less instruction-counter tick on the hot path. The codegen helper `E.if0` only supports `[] → results` block signatures today; would need a sibling `E.ifP` (or similar) that emits a `(param ...)` block type and trusts the arms to consume from the stack. Same RTS-barrier-hot-path issue as the global trick — they pair naturally in one standalone fix.

## Branch / PR strategy

- Base on `gabor/wasm-exts-sync` (PR #6043) which already wires `return_call`.
- Branch: `gabor/modulo-constructor`.
- v0: `TupPrim` + `OptPrim (TupPrim …)` only; single-self-call spines; non-incremental GC; raw `StorePrim` codegen; `OptPrim` typed-identity for nop casts.
- v1 (follow-up): `NewObjE`, `ArrayPrim`, multi-element constructors with the bullet at any slot; barriered `StorePrim` lowering for incremental GC.

## Implementation roadmap

1. ✅ **Detector** — recognise modulo-constructor spines containing a self-call; eprintf the function name. Committed `191754d6a`.
2. ✅ **`StorePrim` IR primitive** — declaration + plumbing in `Ir` / `arrange_ir` / `check_ir` (lax `T`/`T'` rule). Committed `af0df0ffe`.
3. ✅ **Worker synthesis** — `tailcall.ml`'s `LetD … FuncE` arm now emits the wrapper + nested-worker pair, rewrites the spine to call the worker, and recognises the v0 spine `?(head, self<Ts>(t, f))`. Committed `493e90b8f`.
4. ✅ **Codegen (enhanced)** — `StorePrim` lowered via `IdxLE` + `AssignE` over an `obj` whose `note.typ` has been retagged to `[var Any]`. Reuses the existing array-store path, including the incremental-GC write barrier. No IR-level `CastPrim` needed — `check_ir` doesn't re-run on backend-synthesised IR, so a note-only retag suffices. Committed `51aad1630` + later cast-removal.
5. **Tests** — `test/run/trmc-map.mo` and friends; FileCheck the IR; benchmark vs. naïve `map` on the IC instruction counter.
6. **Multi-spine generalisation (v1)** — extend recognition + synthesis to `NewObjE`, `ArrayPrim`, arbitrary slot indices, head expressions beyond `f h`.
7. **Codegen fast-path (v1)** — bypass the `IdxLE` boxed-Nat unbox dance + bounds check for compile-time-constant slot indices; emit a direct constant-offset store like `Tuple.load_n` does for loads.
8. **Classical backend** — currently only enhanced has the `StorePrim` codegen. Classical needs a parallel arm.

## Compiled output (v0, enhanced backend)

End-to-end check on a small actor that calls a TRMC `map` over `List<Nat>`:

- Wasm validates with `--enable-memory64 --enable-tail-call`.
- 25 `return_call` instructions in the worker's hot path.
- `i64.store offset={1,9,17}` for the `StorePrim` sites (heap header + bullet writes).
- Without `--experimental-tailcalls`: zero `return_call` (no regression on the default path).

### Heap layout (enhanced GC, skew = 1)

For a 2-element tuple/cell:

| slot | byte offset | content |
|---|---|---|
| 0 | 1  | tag = 7 (Array) |
| 1 | 9  | forwarding pointer (deref point under incremental GC) |
| 2 | 17 | array length (= 2) |
| 3 | 25 | field 0 — head |
| 4 | 33 | field 1 — tail / **bullet** |

So `i64.load offset=9` follows the forwarding pointer; the next `i64.load offset={25,33}` reads field 0 / 1 of the canonical object.

### Wrapper `$map (clos, xs, f) -> root`

```wat
(func $map (param $clos i64) (param $xs i64) (param $f i64) (result i64)
  ;; case null → return null (-5 is the null sentinel)
  ;; case ?(h, t) →
  i64.load offset=9  i64.load offset=25  ;; h := xs.0
  i64.load offset=9  i64.load offset=33  ;; t := xs.1

  ;; f(h)
  i64.load offset=9  ...  call_indirect (type 14)

  ;; Allocate root = (f(h), t)  — bullet := input tail
  i64.const 5  call $alloc_words
  i64.const 7   i64.store offset=1       ;; tag
  ... i64.store offset=9                 ;; forwarding := self
  ... i64.const 2  i64.store offset=17   ;; length
  ... i64.store offset=25                ;; root.0 := f(h)
  ... i64.store offset=33                ;; root.1 := t  ← bullet
  call $allocation_barrier

  ;; Call worker (NOT a return_call — wrapper still has work)
  i64.const 0  local.get $$root/0  local.get $f
  call $$map'/0
  drop                                    ;; discard worker's () return

  ;; Return root.  CastPrim (B, List<A>) → List<B> is a wasm nop.
  local.get $$root/0)
```

### Worker `$$map'/0 (clos, parent, f) -> ()`

```wat
(func $$map'/0 (param $clos i64) (param $$parent/0 i64) (param $f i64) (result i64)
  ;; Read input tail FROM the bullet:  switch_in := parent.1
  local.get $$parent/0
  i64.load offset=9                       ;; deref parent's forwarding
  i64.load offset=33                      ;; load slot 1 = the bullet

  ;; case null → return ()  (bullet already null, no work)
  ;; case ?(h, t) →
  i64.load offset=9  i64.load offset=25   ;; h
  i64.load offset=9  i64.load offset=33   ;; t

  ;; f(h)
  ...  call_indirect (type 14)

  ;; Allocate cell = (f(h), t)
  i64.const 5  call $alloc_words
  i64.const 7   i64.store offset=1
  ... i64.store offset=9                  ;; forwarding := self
  ... i64.const 2  i64.store offset=17
  ... i64.store offset=25                 ;; cell.0 := f(h)
  ... i64.store offset=33                 ;; cell.1 := t  ← next bullet

  ;; ===== StorePrim (parent.1, ?cell) =====
  ;; Lowered via CastPrim + IdxLE + AssignE.
  ;; Boxed-Nat dance to recover idx = 1 from the encoded value 6:
  i64.const 6  ...  i64.shr_s             ;; 6 >> 2 = 1

  ;; Bounds check (idx < array.length); traps on OOB. Overhead inherited
  ;; from going through IdxLE — v1 fast-path elides this for constant idx.
  ...  i64.lt_u  if  else  ...trap... end

  ;; Compute write address: base + (header + idx) * 8 + skew
  i64.const 3  i64.add  i64.const 3  i64.shl
  i64.load offset=9  i64.add
  i64.const 1  i64.add                     ;; skew
  local.get $$cell/0

  ;; Incremental-GC-aware store: barriered when GC is running, raw otherwise.
  call $running_gc
  if    call $write_with_barrier           ;; ← barrier inherited for free
  else  i64.store                          ;; raw
  end

  ;; ===== Tail call =====
  i64.const 0  local.get $$cell/0  local.get $f
  return_call $$map'/0)                     ;; ← the win
```

### What this buys us

- **Per element**: 1 heap alloc (5 words), 1 `StorePrim` (raw or barriered store), 1 `return_call`. Linear in list length, no stack growth.
- **Incremental-GC barrier for free**: by lowering `StorePrim` through `compile_lexp`'s `IdxLE` path, the worker's bullet store becomes a `call $running_gc → write_with_barrier else i64.store` runtime selector — exactly the policy live mutators already use.
- **Wrapper-side `call`, worker-side `return_call`** — exactly the design: one wrapper frame alive across the whole list, the worker self-recurses without stack growth.

### Remaining cost (v1 work)

- The cast-to-array trick pays for: a boxed-Nat unbox at runtime (`shr_s` of the literal 6 → 1), and a per-iteration bounds check that always succeeds. Both are fixed-cost, both can be elided by a constant-index fast-path in the `StorePrim` codegen (skip the IdxLE indirection, emit `Tuple.load_n`-style address math directly).
- Classical backend doesn't yet implement `StorePrim`.
