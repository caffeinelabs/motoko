# Plan: Wasm Abstract Interpreter for Constant Tracking

## Goal

An abstract interpreter that tracks constant integral values on the Wasm operand stack, enabling constant propagation optimizations in the Motoko compiler's codegen.

## Core Design

### Stack Constants LRU Cache

- **n-slot LRU cache** keyed by stack depth (0 = top of stack)
- Only tracks **integral values** (i32, i64) — no floats
- A `const.i32 42` or `const.i64 7` pushes the value into depth key 0
- Non-constant-producing instructions mark their result depth as unknown (evict from LRU if present)

### High Water Mark

- Each instruction has a **key-independent high water mark**: the net stack depth change
  - e.g. `i32.add` pops 2, pushes 1 → watermark = -1 (net), but consumed depth 2
  - `i32.const` pushes 1 → watermark = +1
  - `call $f` with signature `[i32, i32] -> [i32]` → watermark depends on signature
- The watermark tracks the **maximum consumption depth** before the instruction's result is pushed

### Call Instruction Handling

At each `call` instruction:
1. Compute the net depth change from the callee's type signature
2. **Update all depth keys** in the LRU by the watermark (shift them)
3. **Evict entries with negative depths** (they were consumed by the call)
4. Return values (if any) are unknown unless we inline/specialize

### LRU Eviction Policy

When the LRU is full and a new constant needs to be inserted:
1. Update all depths by the current watermark
2. Evict entries with negative depths (consumed)
3. If still full, evict the **deepest** entry (largest depth key — furthest from TOS)

### Instruction Categories

| Category | Stack effect | LRU action |
|----------|-------------|------------|
| `i32.const N` / `i64.const N` | push 1 | Insert N at depth 0, shift others +1 |
| `i32.add`, `i32.mul`, etc. | pop 2, push 1 | If both operands constant → compute & insert result; else evict depth 0, shift |
| `local.get i` | push 1 | If local `i` is tracked constant → insert at depth 0; else unknown |
| `local.set i` | pop 1 | If depth 0 is constant → track local `i` (Phase 2); evict depth 0 |
| `call` | pop args, push results | Shift by net depth, evict negatives, results unknown |
| `drop` | pop 1 | Remove depth 0, shift others -1 |
| `select` | pop 3, push 1 | If condition constant → propagate selected operand; else unknown |

## Phase 1: Stack-only constant tracking

- LRU cache with n slots (configurable, e.g. 8)
- Track constants through pushes, arithmetic, drops
- Handle calls by signature-based depth adjustment
- No local tracking

## Phase 2: Local constant tracking (later)

- When a constant is written to a local (`local.set`/`local.tee`), record it
- When a local is read (`local.get`), propagate the constant if tracked
- When a local is compared to a constant (`i32.eq` etc.), the local becomes that constant on the true branch (control flow sensitivity)
- Locals lose their constant status when overwritten with a non-constant

## Phase 3: Control flow (later)

- At branches, fork the abstract state
- Merge states at join points (constants agree → keep; disagree → unknown)
- Dead branch elimination when branch condition is constant

## Implementation Notes

- This lives in the compiler's codegen pipeline (OCaml), operating on the Wasm instruction stream
- The LRU can be a simple array with linear scan (n ≤ 8)
- The watermark is computed per-instruction from the Wasm spec's stack typing rules
- Integration point: after instruction selection, before final Wasm emission

## Implementation Status

### Phase 1 — Done (src/linking/constTrack.{ml,mli})

- Pure LRU cache keyed by stack depth, configurable capacity
- `process_block` iterates instructions, continues through calls and control flow
- Constant propagation through `i32.const`, `i64.const`
- Constant folding for `i32.add/sub/mul`, `i64.add/sub/mul`
- Smart `Select`: known condition picks operand, equal operands propagate regardless
- Correct stack depth tracking for all instruction categories
- `func_type` callback resolves function index → (n_params, n_results) arity
- `Call`: consumes params, produces results (all unknown), shifts LRU by net delta
- `CallIndirect`: same as `Call` but also consumes the table index (+1 param)
- `on_call` callback: fires at each Call/CallIndirect with LRU state, instruction
  index, and arity — enables the zero-forwarder rewriter in linkModule
- `Block`: processes body recursively, evicts result slots conservatively at join
- `If`: pops condition, forks into then/else, intersects LRU states at join
- `Loop`: conservatively flushes (back-edges make iteration unknown)
- `BrIf`: continues on fall-through path (pop condition), taken path exits to outer
- `Br`/`BrTable`/`Return`/`Unreachable`: terminators, return `None`
- `block_arity`: resolves `ValBlockType` directly, `VarBlockType` via optional
  `type_section` callback
- Uses `Wasm_exts.Ast` types (not `Wasm.Ast`) — matches `linkModule.ml`'s namespace
- `dump` for debugging

### Integration with linkModule.ml — Done

The flat-array heuristic rewriter (`arr[i - param_count]`) has been replaced with
constTrack-based sound stack-depth tracking:
- `on_call` callback checks `ConstTrack.lookup lru (n_params - 1)` at each call
  to a known zero-forwarder
- Any constant at the closure-arg depth triggers the rewrite
- Fixpoint iteration handles chains (foo → bar → quux)
- Diagnostic `eprintf` on each rewrite (for development; remove before merge)

17 test files show rewrites, up from 6 before Block/If/BrIf handling was added.

### Findings from implementation

- Wasm `Select` is nullary in this AST (no type annotation argument)
- `Wasm.Source.phrase` needs `open Wasm.Source` for `.it` field access
- `I32Op`/`I64Op` binop constructors need qualification to avoid warning 40
- `open Wasm_exts.Values` inside `step` — can't open at top because `I32`/`I64`
  constructors clash with our `const_val.I32`/`I64`
- The module lives in `src/linking/` alongside `linkModule` — natural home since
  it operates on the Wasm AST post-linking
- `dune` auto-discovers the new `.ml` — no build file changes needed
- Pure data structures throughout (list-based LRU) — ready for bifurcation
- `Call`/`CallIndirect` handling: results are unknown but depth tracking is precise,
  so constants surviving across calls (deeper on the stack) are preserved
- Block/If body processing: body's depth tracking is authoritative, no exit shift
  needed.  Result slots (depths 0..n_results-1) evicted conservatively at join.

### Known pessimisations (Block/If join points)

- **BrIf-less Blocks with constant results**: result slots evicted even though
  there's only one path — no branch can disagree
- **All-commensurable branches**: when every BrIf-taken path and the fall-through
  agree on the result values, we still evict

Both require accumulating branch states to fix (see Phase 3 below).

## Phase 2: Local constant tracking (later)

- When a constant is written to a local (`local.set`/`local.tee`), record it
- When a local is read (`local.get`), propagate the constant if tracked
- Locals lose their constant status when overwritten with a non-constant

## Phase 3: Precise branch joins via algebraic effects

The current join-point handling conservatively evicts result slots because
`BrIf`-taken paths may carry different values. A precise solution requires
collecting LRU states from all paths that reach a join point.

OCaml 5.x algebraic effects provide a clean mechanism.  All three branch
instructions emit `May_leave` with the LRU state and target depth:

- **`BrIf n`**: perform `May_leave (n, lru')` where `lru' = shift_and_evict (-1) lru`
  (condition popped), then continue on fall-through.
- **`Br n`**: perform `May_leave (n, lru)`, then return `None` (terminator).
- **`BrTable (targets, default)`**: pop the index, then iterate all targets
  (`List.iter (fun t -> perform (May_leave (t, lru'))) (targets @ [default])`),
  then return `None`. Just a `fold_left` / `iter` — each target block collects
  the same LRU state since the stack is identical at that point.

Each `Block` handler installs an effect handler that:
1. Collects `May_leave (0, lru)` — branch targeting *this* block
2. Re-raises `May_leave (n-1, lru)` — branch targeting an outer block

At block exit, intersect the fall-through LRU with all collected branch LRUs.
Only constants that agree across all incoming paths survive.  This eliminates
both known pessimisations (BrIf-less blocks and all-commensurable branches).

```ocaml
(* Phase 3 sketch *)
type _ Effect.t += May_leave : int * t -> unit Effect.t

(* In step: *)
| BrIf n ->
    let lru' = shift_and_evict (-1) lru in
    perform (May_leave (n.it, lru'));
    Some lru'  (* fall-through *)

| Br n ->
    perform (May_leave (n.it, lru));
    None

| BrTable (targets, default) ->
    let lru' = shift_and_evict (-1) lru in  (* pop index *)
    List.iter (fun t -> perform (May_leave (t.it, lru'))) (targets @ [default]);
    None

(* In Block handler: *)
| Block (bt, body) ->
    let branch_states = ref [] in
    match_with (process_block_inner ...) inner_lru body
    { effc = fun (type a) (eff : a Effect.t) ->
        match eff with
        | May_leave (0, lru) ->
          Some (fun (k : (a, _) continuation) ->
            branch_states := lru :: !branch_states;
            continue k ())
        | May_leave (n, lru) ->
          Some (fun k ->
            continue k ();
            perform (May_leave (n - 1, lru)))  (* re-raise for outer block *)
        | _ -> None }
    (* join: intersect fall-through with all collected branch states *)
    let result = List.fold_left intersect fall_through !branch_states in
    ...
```

The depth decrement (`n - 1`) happens naturally at each Block boundary.
No accumulator threading, no return-type changes. Pure control flow.

For BrIf-less blocks, `branch_states` stays empty — the fall-through LRU
is used as-is with no eviction.  For all-commensurable branches, the
`fold_left intersect` preserves agreeing constants.  Both pessimisations
resolved.

**Prerequisite**: OCaml 5.3 migration (draft PR exists).

## Open Questions

- What is the right LRU size? Too small misses opportunities, too large adds overhead
- Should we track through `memory.load`/`memory.store` at known constant addresses?
- ~~Should `Call` consume args and produce results (shifting the LRU) instead of stopping?~~
  **Resolved**: Yes, `Call` now shifts the LRU by `n_results - n_params`.
- ~~How to handle `block`/`loop`/`if`?~~
  **Resolved**: Recursive processing with conservative result-slot eviction.
  Precise joins deferred to Phase 3 (algebraic effects).
- Integration point: lives in `linking/` — operates on the merged Wasm AST during linking.
