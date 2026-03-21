# Plan: Masked `br_table` Dispatch for Variant Switches in `moc`

## Motivation

Motoko variant tags are 32-bit hashes of string labels. A `switch` on a
variant with `n` arms currently compiles to a linear chain of `n` hash
comparisons (`orsPatternFailure`). The existing special cases are:

- **1-arm**: skip tag check entirely (`single_case`)
- **2-arm**: skip the second comparison (`simplify_cases`)
- **n-arm**: linear chain — O(n) comparisons

This plan adds an O(1) dispatch for the n-arm case using Wasm `br_table`
and a compile-time-chosen bitmask.

## Core Idea

Given variant hashes `h₁, h₂, ..., hₙ`, find a mask `M` at compile time
such that `hᵢ & M` are all distinct. Also compute `S = ctz(M)` (count of
trailing zero bits in `M`) at compile time. Then emit at runtime:

```wasm
local.get $tag
i32.const M     ;; compile-time constant
i32.and         ;; runtime: mask the tag
i32.const S     ;; compile-time constant: S = ctz(M) — both lines
i32.shr_u       ;; omitted entirely when S = 0 (would be a nop)
br_table $tbl[0] $tbl[1] ... $default
```

The right-shift eliminates the trailing zero bits of `M`, reducing the
effective table size from `max(hᵢ & M) + 1` to
`(max(hᵢ & M) >> S) + 1` — potentially much smaller.

## Finding M and S (OCaml, all at compile time)

**Algorithm:** iterate integers `n` from 1 upward, filtering for
`popcount(n) = ceil(log₂(arms))` (the minimum bits needed to distinguish
`arms` values — exactly `k` bits can represent 2^k distinct indices).
For each candidate mask, check if masking all hashes is injective.
Return the first that passes — it is automatically compact because small
integers concentrate their set bits in low positions, keeping `max(hᵢ & M)`
small and thus the table size small after the `>> S` shift.

Note: `floor(log₂(n)) + 1` equals `ceil(log₂(n))` only when `n` is *not*
a power of 2; for powers of 2 it overshoots by 1. Use `ceil` directly.

```ocaml
(* All computation below happens during Wasm code generation, not at runtime *)

let popcount32 m = (* count set bits in int32 *) ...
let ctz32 m = (* count trailing zeros; 32 if m = 0l *) ...

(* ceil(log₂(n)) — minimum bits to index n distinct values *)
let bits_needed n =
  let rec f k = if 1 lsl k >= n then k else f (k + 1) in
  f 1

let is_injective mask hashes =
  let masked = List.map (Int32.logand mask) hashes in
  List.length masked = List.length (List.sort_uniq compare masked)

let find_mask arms hashes threshold =
  let required_bits = bits_needed arms in
  (* Iterate n = 1, 2, 3, ... filtering for popcount = required_bits *)
  let rec loop n =
    if n = 0l then None  (* wrapped around — give up, use linear *)
    else if popcount32 n = required_bits && is_injective n hashes then
      let s = ctz32 n in
      let tbl_size = (* max((hᵢ & n) >> s) + 1, computed at compile time *) ... in
      if tbl_size <= threshold then Some (n, s, tbl_size)
      else loop (Int32.add n 1l)  (* mask valid but table too big: keep trying *)
    else loop (Int32.add n 1l)
  in
  loop 1l
```

If no mask with `required_bits` bits fits in the threshold, try
`required_bits + 1`, etc., up to a small maximum (e.g. 8 bits).

**Threshold:** e.g. `max(64, 4 * n)`.

## Wasm Block Structure (runtime)

`br_table` with index `i` exits the `i`-th enclosing block.

```wasm
block $exit
  block $default
    block $arm_{n-1}
      ...
        block $arm_0
          local.get $tag
          i32.const M          ;; compile-time constant
          i32.and              ;; runtime
          ;; only emit next two instructions when S > 0:
          i32.const S          ;; compile-time constant, S = ctz(M)
          i32.shr_u            ;; runtime — omitted entirely when S = 0
          br_table $tbl[0] .. $tbl[tbl_size-1] $default
        end ;; $arm_0
        <body of arm_0>
        br $exit
      ...
    end ;; $arm_{n-1}
    <body of arm_{n-1}>
    br $exit
  end ;; $default
  unreachable   ;; type-safe: dead code
end ;; $exit
```

`$tbl[j]` = `$arm_k` where `(hₖ & M) >> S = j`, else `$default`.

## Break-even Analysis

### Dynamic instruction count

Each arm in the current linear chain costs ~6 instructions:
`local.get` + 2 heap loads (forwarding ptr + tag field) + `i32.const` + `i32.eq` + `br_if`.
The chain stops at the first match, so:

| Path | Instructions executed |
|------|-----------------------|
| Linear (worst case, last arm) | `6n` |
| Linear (average, uniform input) | `3n` |
| br_table, S = 0 | **5** (get + 2 loads + const M + and + br_table) |
| br_table, S > 0 | **7** (+ const S + shr_u) |

**Worst-case break-even:** n ≥ 2 for both variants (5 < 12, 7 < 12).
**Average break-even:** n ≥ 3 (5 < 9, 7 < 9).

Since n = 1 and n = 2 are already handled by `single_case` / `simplify_cases`,
the `br_table` path is a strict win for every case it applies to (n ≥ 3).

### Static code size

Approximating ~4 bytes/instruction and ~4 bytes/br_table entry:

| Path | Bytes |
|------|-------|
| Linear | `~26n` (6 instrs/arm + block overhead) |
| br_table, S = 0 | `~20 + 4 × table_size + 2n` |
| br_table, S > 0 | `~28 + 4 × table_size + 2n` |

Code-size break-even (S = 0): `table_size ≤ 6(n − 1)` → e.g. n=3 allows table_size ≤ 12.
Code-size break-even (S > 0): `table_size ≤ 6n − 8` → e.g. n=3 allows table_size ≤ 10.

The threshold `max(64, 4n)` is well within these limits, so code size is
never worse when the threshold is respected.

### Empirical measurement (TODO)

Add a `bench` test that calls a large variant switch (e.g. 8-arm, 16-arm)
in a tight loop, comparing the old linear output (forced via flag or saved
`.wat`) against the optimised one. The existing `bench` package in the
test suite can measure Wasm instruction counts or wall-clock cycles.

Note: Wasm JIT compilers (wasmtime, V8) typically lower `br_table` to a
hardware jump table, giving an additional constant-factor speedup over
what static instruction counts suggest.

## Implementation Steps

### Step 1 — Mask-finding utility (compile time)

Add near the `Variant` module in `compile_classical.ml`:

```ocaml
(* Returns Some (mask, shift, dispatch_table) or None if too large.
   dispatch_table.(j) = Some arm_index | None (hole → default). *)
val find_variant_dispatch : int32 list -> int ->
  (int32 * int * int option array) option
```

### Step 2 — New dispatch path in `SwitchE`

Extract `(TagP (lᵢ, _), body_i)` for each case, compute hashes via
`hash_variant_label env lᵢ`, call `find_variant_dispatch`. If `Some`:

1. Emit `get_tag ^^ compile_unboxed_const M ^^ G.i (Binary (I32 And))`
2. If S > 0: emit `compile_unboxed_const (Int32.of_int S) ^^ G.i (Binary (I32 ShrU))`
3. Build `BrTable` target list from dispatch table
4. Wrap in nested blocks, emit arm bodies with `Br $exit`

If `None`, fall through to `orsPatternFailure`.

### Step 3 — Payload extraction

Call `Variant.project env` at the start of each arm body (same as current
path) to load the variant payload after dispatch.

### Step 4 — `compile_enhanced.ml`

Check for a parallel `SwitchE` handler; apply the same optimisation.

### Step 5 — Tests

- 3-arm, 5-arm, 10-arm, 20-arm variant switches
- Cases where S > 0 (mask has trailing zeros)
- Edge case: threshold exceeded → confirm fallback to linear chain
- Inspect `.wat` output to verify `br_table` + shift are emitted correctly

## Key Files

| File | Role |
|------|------|
| `src/codegen/compile_classical.ml` | Main: `SwitchE`, `simplify_cases`, `Variant` module |
| `src/codegen/compile_enhanced.ml` | Parallel backend, may need same change |
| `src/wasm-exts/ast.ml` | `BrTable` already defined (line 102) |
| `test/run/` | New test cases |

## Open Questions / Risks

1. **`FakeMultiVal.block_` interaction.** The multi-value block wrapper
   tracks nesting depth. The new nested arm blocks must be introduced
   *inside* `FakeMultiVal.block_` so depth accounting stays correct.

2. **Hash collisions between labels.** Essentially impossible (32-bit hash
   over distinct strings within one type), but the algorithm degrades
   gracefully — it simply needs more bits in `M`. Ultimate fallback:
   `M = ~0l`, `S = 0`, table size = 2^32 → threshold exceeded → linear.

3. **31-bit vs 32-bit hashes.** Confirm the range of `E.hash` — if the
   MSB is never set, the mask search can skip bit 31.

4. **Threshold tuning.** `max(64, 4n)` is a starting point; may need
   benchmarking to confirm the right code-size / speed trade-off.

5. **GC forwarding pointers.** `get_variant_tag` already calls
   `load_forwarding_pointer` — no change needed here.

## Non-goals

- Nested/wildcard patterns in variant arms (handled by `compile_pat_local`)
- `switch` on non-variant types
- JS backend (`moc.js`)
