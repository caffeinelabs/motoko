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

For `PrimE (StorePrim, [ProjE obj n; v])`:

1. Emit `obj`'s value-producing code (push base pointer).
2. Emit address-of-slot-`n` math (constant-offset add) — same as `ProjPrim` codegen would have.
3. Emit `v`'s value-producing code.
4. Emit `i32.store` (instead of the `i32.load` `ProjE` would have ended with).

The "flip trailing load to store" lives entirely in the backend — middle-end emits `StorePrim` regardless of GC variant. Generalisation to `IdxPrim` (arrays) and `DotPrim` (records) is mechanical.

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

## Branch / PR strategy

- Base on `gabor/wasm-exts-sync` (PR #6043) which already wires `return_call`.
- Branch: `gabor/modulo-constructor`.
- v0: `TupPrim` + `OptPrim (TupPrim …)` only; single-self-call spines; non-incremental GC; raw `StorePrim` codegen; `OptPrim` typed-identity for nop casts.
- v1 (follow-up): `NewObjE`, `ArrayPrim`, multi-element constructors with the bullet at any slot; barriered `StorePrim` lowering for incremental GC.

## Implementation roadmap

1. ✅ **Detector** — recognise modulo-constructor spines containing a self-call; eprintf the function name. Committed `191754d6a`.
2. **`StorePrim` IR primitive** — declaration + plumbing in `Ir`, `arrange_ir`, `check_ir` (admit `load_expr : T, new_value : T → ()`), interpreter (for `-iR` testing).
3. **Codegen for `StorePrim`** — non-incremental: emit address-computation + `i32.store`, suppressing the trailing load that `ProjPrim` codegen would have emitted.
4. **Worker synthesis** — extend `tailcall.ml`'s `LetD … FuncE` arm: when the body has TRMC candidates, emit the wrapper + worker pair and rewrite the spine into `let cell = … ; StorePrim (parent.S, cell) ; return_call f'(cell, …)`.
5. **Tests** — `test/run/trmc-map.mo` and friends; FileCheck the IR; `-iR` verify semantics; benchmark vs. naïve `map` on the IC instruction counter.
6. **Multi-spine generalisation (v1)** — extend recognition + synthesis to `NewObjE`, `ArrayPrim`, arbitrary slot indices.
7. **Incremental GC barrier (v1)** — codegen variant for `StorePrim` that follows `parent[0]`.
