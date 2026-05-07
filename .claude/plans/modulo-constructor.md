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

OCaml ships this as `[@tail_mod_cons]` (Filliâtre & Pottier). The transformation builds the result list bottom-up at allocation sites and threads a *destination* — the address (or proxy) of the not-yet-filled slot — into the tail call.

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

- `OptP` / `OptPrim` is operationally the identity on heap-allocated values (tuples, records, arrays, objects, …). The `?(B, List<B>)` value *is* the tuple pointer, with `null` as a distinguished sentinel.
- The recursive call sits in a fixed slot of a freshly-allocated tuple. Once the tuple is allocated, the slot is the only place its eventual value needs to land.

So the candidate constructors that put a self-call into "tail position modulo allocation" are at minimum:

- `PrimE TupPrim …` (one field is the self-call)
- `PrimE OptPrim (PrimE TupPrim …)` (the `Opt` is free)
- `PrimE OptPrim (NewObjE …)` and `NewObjE …` (a record field is the self-call)
- `PrimE ArrayPrim …` (one element is the self-call) — likely lower priority, less common

## Translation

Source:

```
func map(xs, f) =
  switch xs {
    case null         null
    case (?(h, t))    ?(f h, map(t, f))
  }
```

Wrapper + tail-recursive helper:

```
func map(xs, f) =
  switch xs {
    case null         null
    case (?(h, t))    { let root = (f h, t);     -- bullet := input tail
                        map'(root, f);           -- could be regular call
                        ?root }
  }

func map'(parent, f) =                            -- tail-recursive
  switch (parent.1) {                             -- read input tail FROM the bullet
    case null         { /* parent.1 already null */ }
    case (?(h, t))    { let cell = (f h, t);      -- new bullet := next input tail
                        parent.1 := ?cell;        -- close the previous output cell
                        return_call map'(cell, f) }
  }
```

### Why `parent.1` initialised to the input tail

The bullet does *not* need to be uninitialised garbage or a separate non-traced address parameter. Initialising it to `t` (the input tail at this position) makes the slot a well-typed `List<A>` pointer that the GC traces normally. The recursive callee reads its input from there and overwrites with its output. One field in the heap subsumes both "destination pointer" and "input list cursor" — `map'`'s arity stays at 2.

### Static type slack

In flight, the bullet temporarily holds `List<A>` while the surrounding tuple type is `(B, List<B>)`. Statically heterogeneous, runtime-identical (both are `?(_, _)` heap pairs). `check_ir` will need to admit this transient via either an explicit cast, a `parent : (B, ?Any)` typing of the helper, or a sealed pass-internal IR extension.

## GC analysis

Two regimes by RTS variant.

### Non-incremental (copying, generational stop-the-world)

GC fires only at allocation points and is atomic. Between the producer's allocation of `cell` and the consumer's read of `cell.1`, the heap is quiescent; no relocation can occur. The bullet address is stable for the duration of one `return_call`. The original "unskewed even address" framing works directly: pass a raw pointer to the slot, write to it, recurse.

### Incremental

GC progress is interleaved with mutator steps. Heap objects may be relocated; the canonical location is reached via the **forwarding pointer** in the first header word.

- **Write** to `parent.1` (closing the previous output cell): the preceding `cell` allocation may have triggered a GC step, potentially moving `parent`. Must follow `parent[0]` (the indirection) then offset `+C` for the bullet. **Barriered store.**
- **Read** of `parent.1` at entry of `map'`: between the producer's set of `cell.1` (at allocation time, in the previous frame) and this load (entry of the next frame), the only operations are the closing store, the `return_call` setup, and entry — none of which allocate. No GC step can have run. Direct `+C` load — **no barrier**.

This is precisely the *argument-passing* contract: parameters written by the caller and read by the tail-called callee straddle no GC point. The optimisation is recovering that contract for the bullet, treated as an extra "argument" delivered through the heap rather than the stack.

### Dispatch matrix

| GC variant       | Bullet protocol                          | Read     | Write     |
|------------------|------------------------------------------|----------|-----------|
| Copying          | unskewed bullet address as i32 arg       | raw      | raw       |
| Generational STW | unskewed bullet address as i32 arg       | raw      | raw       |
| Incremental      | parent heap pointer, fixed offset `C`    | raw `+C` | indirect  |

The incremental case can use the *same* IR as the non-incremental: pass the parent pointer (a real Motoko value, GC-tracked across the call). What differs is the codegen of the closing store — barriered on incremental, raw on the others. The pass emits the IR; the backend's existing barrier policy does the right thing.

## Recognition

Hook into the self-call detector that `--experimental-tailcalls` uses to identify self-recursive calls eligible for `return_call`. Today it accepts a self-call in tail position. Extend it to also accept a self-call sitting under a *modulo-constructor* spine:

```
modulo_constructor_spine ::=
  | TupPrim  e1 … self_call … en
  | OptPrim  (modulo_constructor_spine)
  | NewObjE  fields_with_one_self_call_in_a_field
  | ArrayPrim e1 … self_call … en           -- maybe later
```

The detector returns `(spine_constructor_chain, slot_index)` — the chain of allocators wrapping the self-call and the index of the slot the call's result occupies in the innermost allocator.

Constraints on accepting the rewrite:

- Exactly one self-call in the spine. (Multiple recursive calls inside one constructor is a different problem — fork/join, not TRMC.)
- The self-call's other arguments must not depend on values computed *after* the constructor allocation site, since we're hoisting the allocation before the recursive call. (For `?(f h, map(t, f))`, `f h` and `t, f` are all available before allocation — fine.)
- The function's other tail positions (the `null` arm in our example) must produce a value the bullet can be pre-initialised to. For `null`, this is trivially `null`. For non-`null` base cases, we may need a more general bullet-init scheme, or punt for v0.

## Synthesis

For each accepted function `f`, emit:

1. A wrapper `f` retaining the original signature. On non-recursive arms, evaluates as before. On the modulo-constructor arm, allocates the root cell, calls the helper, and returns the (possibly `Opt`-wrapped) root.
2. A helper `f'` taking `(parent, …other_args)`. Reads its real input from `parent[slot_index]`. On the modulo-constructor arm, allocates the new cell, stores it into `parent[slot_index]`, `return_call`s itself with the new cell as parent. On other arms, the bullet has already been initialised to a value that is the correct output (e.g., `null` for the null case).

The helper goes through the existing `--experimental-tailcalls` machinery and emits `return_call f'`. No new RTS primitives needed; reuses heap allocator and existing barrier policy on stores.

## Open questions

- **Type system in IR.** Either widen `parent`'s slot to `?Any`, introduce a pass-internal `unsafe_cast` IR node, or accept that `check_ir` runs *before* the rewrite and the rewrite is a backend-only transform. (Current `--experimental-tailcalls` is a codegen-time decision, so probably the latter.)
- **Multi-arity constructors.** Generalise the bullet to "initialise slot S to the corresponding input field; pass parent + S". The `map` case (S = 1, init = `t`) is the simplest; other shapes (e.g., `Cons`-like records) have analogous initialisations.
- **Interaction with `--experimental-tailcalls` outer flag.** TRMC should be implied by `--experimental-tailcalls` rather than gated on a separate flag, to keep the surface small. Worth confirming in PR review.
- **Mutual recursion modulo constructor.** Out of scope for v0; the `return_call_indirect` slice already in `gabor/wasm-exts-sync` is the prerequisite.
- **Counter-examples / soundness.** What happens if `f` captures the bullet and stores it elsewhere (e.g., logging) — should be ruled out by the "exactly one occurrence in the spine, no escape" constraint, but we want a clear predicate.

## Branch / PR strategy

- Base on `gabor/wasm-exts-sync` (PR #6043) which already wires `return_call`.
- New branch: `gabor/modulo-constructor`.
- v0: `TupPrim` + `OptPrim (TupPrim …)` only; single-self-call spines; reuse existing barrier policy.
- v1 (follow-up): `NewObjE`, `ArrayPrim`, multi-element constructors with the bullet at any slot.
