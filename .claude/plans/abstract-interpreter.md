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
- `process_block` iterates instructions, stops at branches but **continues through calls**
- Constant propagation through `i32.const`, `i64.const`
- Constant folding for `i32.add/sub/mul`, `i64.add/sub/mul`
- Correct stack depth tracking for all instruction categories
- `func_type` callback resolves function index → (n_params, n_results) arity
- `Call`: consumes params, produces results (all unknown), shifts LRU by net delta
- `CallIndirect`: same as `Call` but also consumes the table index (+1 param)
- Diagnostic: dumps LRU to stderr when a known zero is present at a call site
- Branches (`block`/`loop`/`if`/`br`/`br_if`/`br_table`) return `None` (stop iteration)
- Uses `Wasm_exts.Ast` types (not `Wasm.Ast`) — matches `linkModule.ml`'s namespace
- `dump` for debugging

### Integration with linkModule.ml — In Progress

A diagnostic pass is wired into `linkModule.ml` (line ~1127) inside the `link` function:
- Iterates over all defined functions in `em1` via `Array.of_list em1.module_.funcs`
- Builds a `func_type` callback from the module's type section and import count
- Calls `ConstTrack.process_block` with capacity-8 LRU on each function body
- Currently diagnostic-only: stderr dumps when zeros are found near call sites
- Next step: use the tracking results to drive zero-forwarder call-site rewriting

### Findings from implementation

- Wasm `Select` takes an argument (optional type annotation) — not nullary
- `Wasm_exts.Source.phrase` needs explicit `open` for `.it` field access
- `I32Op`/`I64Op` binop constructors need qualification to avoid warning 40
- The module lives in `src/linking/` alongside `linkModule` — natural home since
  it operates on the Wasm AST post-linking
- `dune` auto-discovers the new `.ml` — no build file changes needed
- Pure data structures throughout (list-based LRU) — ready for bifurcation
- `Call`/`CallIndirect` handling: results are unknown but depth tracking is precise,
  so constants surviving across calls (deeper on the stack) are preserved

## Open Questions

- What is the right LRU size? Too small misses opportunities, too large adds overhead
- Should we track through `memory.load`/`memory.store` at known constant addresses?
- How to handle `block`/`loop`/`if` — save/restore abstract state?
- ~~Should `Call` consume args and produce results (shifting the LRU) instead of stopping?~~
  **Resolved**: Yes, `Call` now shifts the LRU by `n_results - n_params`. This allows
  tracking through call sequences in straight-line code.
- Integration point: lives in `linking/` — operates on the merged Wasm AST during linking.
  The diagnostic pass in `linkModule.ml` confirms this is the right location.
