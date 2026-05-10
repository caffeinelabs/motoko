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

## Where to Apply the Optimisation

Three candidate insertion points, evolved over the life of the PR:

**Option A — IR level** (`SwitchE` with `TagP` arms, in `compile_classical.ml`)
— the shipped V1. Easy to reach labels, hashes, and exhaustiveness,
but only sees one IR shape: a flat list of `TagP` arms with distinct bodies.

**Option B — Wasm peephole** (scan `i32.const hash / i32.eq / br_if`
chains and rewrite). Rejected: labels must be re-decoded from operands,
exhaustiveness is not locally known, `Wasm.AST` is functional (replacement
is unnatural), and no existing peephole infrastructure to attach to.

**Option C — pattern-code EDSL with an algebraic-effect strategy query**
— the V2 proposal. `compile_enhanced.ml:10635` composes pattern-matching
via `patternCode` values combined with `(^^^)`. Both flat variant arms
and or-pattern arms end up producing the *same* kind of "compare tag
hash H, branch to body B" fragment at this level — the IR-shape
distinction that Option A is blind to has already been elaborated away.

### Why V1 is not enough

`isWeekday` (7 flat arms: `#mon → true; #tue → true; ...; #sun → false`)
optimises to `br_table`. The semantically-equal `isWeekdayOr`
(or-pattern: `case (#mon | #tue | ... | #fri) true; case _ false`)
does not — at IR the or-pattern is a *single* arm with a disjunctive
pattern, so `SwitchE` sees 2 arms and the 4-arm threshold rejects it.
Both should dispatch identically.

### Architecture: Handler / Recognizer split

Two roles, connected by an algebraic-effect protocol (OCaml 5.3 effects,
same machinery as ConstTrack Phase 3). Neither role is variant-specific
— the mechanism is a general matching-EDSL facility for any dispatch
decision. Variant switches are just its first application; AND-patterns
and literal-match chains are future applications of the same protocol.

**Handler** — installed at IR dispatch nodes (`SwitchE` and friends).
Sees the IR node and its type. Knows what kind of decision is being
compiled (variant tag match, integer literal match, record projection,
tuple component match, …), so it knows which strategies *could* apply
and what cost model is meaningful for this decision shape. When the
recognizer asks, the handler computes and returns the chosen strategy.

**Recognizer** — lives inside the matching EDSL — *the procedural
combinator calls* (`fill_pat`, `compile_pat_local`, `(^^^)`,
`orElse`, `orsPatternFailure`, …) that together emit Wasm block
structures for pattern matching. `patternCode` itself is opaque
(`CannotFail of G.t | CanFail of (G.t -> G.t)`) — no walkable AST
survives composition — so the recognizer must observe decisions
*during* emission, not after. At each point where a tag-hash
comparison (or, later, a literal comparison, component projection,
…) is about to be emitted, the combinator `perform`s an effect
— `Match_decision { token; body_compiler; scrutinee_repr }` —
to the enclosing handler. The handler collects incrementally and,
when the arm set is complete, commits on a strategy and emits.

The protocol is deliberately generic:

- `token` is opaque to the recognizer — for variants it's a tag
  hash, for integer literals it's an immediate, for constructor
  matching it's a nominal ID. The handler interprets it.
- `body_compiler` is a thunk the handler can invoke (or not) to
  emit the arm body's Wasm. Giving the handler *control over
  emission* — rather than just asking it for a strategy to feed
  back in — is what unlocks AND-patterns later (the handler can
  return `No_op` and suppress emission entirely when a component
  is already known from an outer context).
- `scrutinee_repr` says how to obtain the discriminating value
  (already loaded on the stack, loadable from a known local, computable
  by a callback). The handler chooses strategies compatible with that
  shape.

### Concrete emission points in the EDSL

In `compile_enhanced.ml`, the `perform`-sites are:

- `fill_pat env ae (TagP (l, p))` — surfaces `Match_decision` with
  `token = hash_variant_label l`, `body_compiler = compile-tail-of-arm`.
- `fill_pat env ae (AltP (p1, p2))` when both legs are `TagP` or
  nested `AltP` chains bottoming out in `TagP` — surfaces one
  `Match_decision` per leaf, sharing the same `body_compiler`
  (this is what makes or-patterns auto-fold).
- Later, literal-pattern arms (`LitP`) would surface `Match_decision`
  with `token = immediate`.

No changes to `(^^^)`, `orElse`, or `orsPatternFailure` — they stay
pure G.t manipulation. The effects are attached at the *leaf*
combinators that know what kind of value is being discriminated.

### Strategy for V2 launch

Gosper-based `MaskShift` only — same cost model as V1. The multi-
strategy batched search (ModPrime, RotLow, …) remains listed below
as future work; the protocol is forward-compatible with them since
strategies are plan-values the handler returns, but the V2 milestone
is purely "make V1's `br_table` dispatch work through the effect
protocol, so or-patterns get it for free".

### Success criterion

An or-pattern switch must compile to byte-identical Wasm as its
hand-expanded flat-arm equivalent. Concretely: `isWeekdayOr` (case
`(#mon | #tue | ... | #fri) true; case _ false`) and `isWeekday`
(7 flat arms) must disassemble to the same `i32.and; [shr_u];
br_table` dispatch, with identical mask, shift, table, and arm
blocks (modulo block-label numbering). A FileCheck test pinning
this equivalence should live in `test/run/variant_switch.mo` (or
a sibling) and is the acceptance gate for V2.

### What this buys

- **or-patterns auto-fold.** Distinct arms with identical bodies
  naturally perform the same payload and the handler sees `k`
  equivalence classes for free. The "Same-body arm merging" section
  below becomes automatic under V2.
- **Generic.** The handler/recognizer split applies to any
  multi-way decision. Future AND-patterns — where matching a
  product pattern may want to *skip* components already tested by an
  outer match — fit the same protocol: the outer handler installs
  "component `k` is known to be `v`" context, the inner recognizer
  asks, and the handler returns `No_op` instead of a dispatch plan.
- **Testable in isolation.** The recognizer can be exercised with a
  synthetic handler that records the effect trace; the handler can
  be exercised on hand-built `token_set` inputs without a Wasm
  backend attached.
- **Context-sensitive overrides.** Outer handlers (debug flag,
  size-budget pass, `--preserve-switch-shape` for disassembly) can
  intercept and force a specific plan without touching call sites.

### Migration path

V1 (IR-level, shipped) stays in place for `compile_classical.ml`. V2
lands first in `compile_enhanced.ml` where the matching EDSL lives,
then — if the architecture proves out — backports to the classical
backend. The `Match_decision` effect surface is narrow enough that
future handlers at different entry points (AND-pattern compilation,
literal-match chains, an IR pass that pre-folds or-patterns) can
slot in without changes at the recognizer side.

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

### Step 4 — `compile_enhanced.ml` *(TODO)*

Apply the same optimisation to the EOP backend. Check for a parallel
`SwitchE` handler in `src/codegen/compile_enhanced.ml` and port the helpers
(`find_variant_mask` etc.) and the new dispatch path there.

Note: the bench tests (`test/bench/`) run under EOP by default — once this
step is done, those benchmarks will reflect the real instruction savings.

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

6. **Gosper iteration cutoff.** Without a bound, `iter_masks_with_popcount`
   may exhaust all C(32,k) candidates (e.g. C(32,4) = 35,960) before
   returning `None`.  A cutoff of 2^16 iterations per popcount level is
   applied in both backends to keep compile time bounded.

7. **`None` fallback correctness — implemented.** The `SwitchE` br_table
   branch builds arm codes using `known_tag_pat` (outer tag check stripped).
   A `None` return from `find_variant_mask` (whether from the cutoff, no
   valid mask, or duplicate labels) must fall through to the regular linear
   `SwitchE` handler — not call `orsPatternFailure` with tag-check-free arms
   (which would let any arm match any outer tag).

   **Fix:** `find_variant_mask` is called in the `when` guard itself.  If it
   returns `None`, the guard fails and OCaml's pattern match falls through to
   the ordinary `SwitchE` handler with full patterns.  The `None` match arm
   in the body is eliminated entirely.  The distinct-labels workaround is
   also removed — it is fully subsumed: duplicate labels cause `is_injective`
   to fail for every mask, so `find_variant_mask` returns `None` and the
   guard fails safely.

3. **31-bit vs 32-bit hashes — resolved.** `Mo_types.Hash.hash` always
   returns `logand 0x7fffffff sum`, so bit 31 is never set.  Masks with
   bit 31 set are irrelevant (would never help distinguish hashes) and
   must be excluded: `Nat32.of_int32` raises `Invalid_argument` on
   negative int32 values (bit 31 set).  Fix: change the Gosper loop
   guard from `!m <> 0l` to `!m > 0l` — stops at zero (wrapped) AND
   at any mask with bit 31 set.  C(31,k) < 2^16 for k≤4 so this guard
   is now the binding constraint for small popcount levels.

4. **Threshold tuning.** `max(64, 4n)` is a starting point; may need
   benchmarking to confirm the right code-size / speed trade-off.

5. **GC forwarding pointers.** `get_variant_tag` already calls
   `load_forwarding_pointer` — no change needed here.

## Future Optimisation: Same-body arm merging

### Scope decision (2026-04-23)

Under V2, or-patterns (`case (#mon or #tue or … or #fri) false`) already
collapse to one arm block via the recognizer's `flatten_tag_leaves`
helper — all leaves of a single case contribute multiple entries to the
same sub-list in `Dispatch.Query`. That's the incentive channel: users
who recognize same-body structure and write an or-pattern get the
benefit.

**We deliberately do NOT auto-merge arms with structurally-equal bodies
across distinct cases.** The reasoning:

- Leaving cross-case merging to the user incentivises writing or-patterns,
  which communicate intent (this group of tags genuinely shares an
  outcome). Syntactic or-patterns are also stable under refactoring in a
  way that "two cases happen to produce identical IR" is not.
- Auto-merging at the IR level is brittle: two arms with the same *IR*
  expression may have different *typing contexts*, scope, or side-effect
  behaviour in edge cases that an equality check might miss.
- More importantly, the right equivalence is **Wasm-instruction-sequence
  equality**, not IR equality (see refinement below) — and that requires
  ahead-of-time arm compilation, which is a larger restructuring.

### Why same-body merging matters for strategy choice

Same-body merging is **not** a speedup for the already-dispatched case
— each br_table slot still lands in its own arm block that executes the
same instructions regardless of whether the blocks are physically
shared. It's a code-size saving of that one duplicated body-block.

But it **does** affect dispatcher choice upstream. The handler's
strategy space is parameterised by `N = number of distinct outcome
classes`, not by the raw arm count:

- `ModPrime`: smaller `p` when classes merge — the br_table shrinks
  linearly with class count. `mod 3` for 3 classes vs `mod 7` for 7
  (same per-op cost, but half the table bytes).
- `MaskShift`: fewer injectivity constraints → Gosper has more masks to
  pick from → may land on smaller popcount (fewer SHR bits, more compact
  table).
- Perfect hashing: easier to find for 3 tokens than 7.

So merging propagates as input-size reduction through every strategy
downstream. Or-patterns already get this benefit because they arrive
pre-merged. Cross-case merging would extend it — but only where the
user didn't already write the or-pattern themselves.

### Refinement: Wasm-level equivalence classes

If we later add cross-case merging, the equivalence criterion should be
**raw Wasm instruction sequences**, not IR structural equality:

1. Compile each arm's body ahead of choosing the dispatch strategy
   (remember: each arm is already a `Block` internally, and composition
   via `(^^^)` is just difference-list concatenation of `G.t`).
2. Compute an equivalence class per arm by comparing the compiled Wasm
   byte sequences (modulo block-label numbering).
3. Pass the class-reduced `token_set` to `Dispatch.Query` — the handler
   sees `k ≤ n` sub-lists.

Advantages over IR-level merging:
- Blind to IR phase-ordering decisions (constant folding, inlining,
  ANF) that might make two semantically-identical arms look different
  at IR level.
- Catches arms that *compile to* identical bytes for reasons the IR
  doesn't surface (e.g. two variant-projection patterns that happen to
  emit the same offset load).
- No coupling to Motoko-specific expression-equality judgement —
  works verbatim for any future matching context.

### Incremental path

1. V2 as-is: or-pattern merging only. Ships as part of #5927.
2. Later: ahead-of-time arm compilation in the recognizer, Wasm-bytes
   comparison, cross-case merging. Handler protocol is already
   compatible (token_set is a list-of-lists); this is a pure
   recognizer-side extension.

*Not yet implemented — tracked here for future work.*

## Future Optimisation: Multi-strategy batched search

The current single Gosper stream can still be slow for unlucky hash sets
(e.g. the 12-arm NNS `Action_` type needs ~8 000 iterations).  The fix is
to run several independent strategy streams **concurrently** and take the
best early result — whichever strategy happens to work cheaply for the
given hashes terminates the search.

### Candidate type

```ocaml
type candidate =
  | MaskShift of { mask : int32; shift : int; table_size : int }
      (* runtime: (hash & mask) >> shift;  overhead: AND + opt. SHR *)
  | ModPrime  of { prime : int; table_size : int }
      (* runtime: hash % prime;            overhead: i32.rem_u       *)
  | RotLow    of { rot : int; bits : int; table_size : int }
      (* runtime: rotl32(hash, rot) & (2^bits-1); overhead: i32.rotl + AND *)
```

### Cost model

```
cost(MaskShift{shift; table_size}) = 2 + (if shift > 0 then 1 else 0)
                                       + table_size   (* br_table entries *)
cost(ModPrime{prime})              = 3 + prime        (* rem_u costs ~3 cycles *)
cost(RotLow{rot; bits})            = 2 + (if rot > 0 then 1 else 0)
                                       + 1 lsl bits
```

Rank by estimated cycle cost; lower is better.

### Strategy generators (lazy streams)

1. **MaskShift (batched Gosper)**: For each bit-position window
   `[2^n, 2^(n+1))` in increasing `n`, run Gosper within that window
   with `popcount = bits_needed(arms)`, then `popcount+1`, etc.  This
   avoids charging to high-valued masks before exhausting low-bit windows.

2. **ModPrime**: Iterate primes `p ≥ n` in increasing order; for each
   check `hᵢ mod p` pairwise distinct.  Terminates quickly when a small
   collision-free prime exists.

3. **RotLow**: For `bits = bits_needed(n)` and `rot = 0..30`, check
   `rotl32(hᵢ, rot) & (2^bits - 1)` injective.  31 candidates per
   `bits` level; try `bits+1` if none work.

### Merger

Run all generators round-robin (or priority-queue ordered by emitted cost).
Collect the first few injective candidates and emit the cheapest.  A simple
scheme: advance each generator one step per round until the first result
appears in any stream; collect a bounded window of results (e.g. 4) across
all streams; return the cheapest.

### Benefits

- No single strategy's worst case dominates compile time.
- Prime-based dispatch is tried immediately and wins when a small prime
  gives a tiny table (common for small variant types).
- Rotation-based dispatch is tried with only 31 probes and guarantees
  table size ≤ 2^k — useful when Gosper + mod-prime both fail cheaply.
- The current 2^16 cutoff per strategy stream remains as a hard cap but
  is rarely hit because one stream produces an early winner.

### Interaction with same-body merging

Apply the merged search after grouping arms by body (use equivalence-class
count as the effective `n`).

*Not yet implemented — tracked here for future work.*

### Refinement: `BitTest` strategy for n = 2

A degenerate case the candidate list above does not yet cover: when the
effective arm count is exactly 2 and the two hashes differ at any single
bit, dispatch reduces to one bit test and a `br_if` — strictly cheaper
than every `br_table`-based strategy.

```ocaml
| BitTest of { bit : int; cmp : Ctz | Clz; on_bit_set : int }
    (* runtime (LSB, bit=0):  i32.load hash;  i32.ctz;  br_if           — 3 wasm ops *)
    (* runtime (MSB, bit=31): i32.load hash;  i32.clz;  br_if           — 3 wasm ops *)
    (* runtime (mid):         i32.load hash;  i32.const bit; i32.shr_u; *)
    (*                        i32.ctz; br_if                            — 5 wasm ops *)
```

cost:
```
cost(BitTest{bit=0|31})  = 2   (* ctz/clz + br_if, after peephole *)
cost(BitTest{bit=other}) = 4   (* shr_u + const + ctz + br_if *)
```

**EOP-specific cleanness.** Variant hashes are 31-bit so the runtime
hash field is stored and loaded as `i32` even under EOP's 64-bit memory
model. The dispatch stays in i32 end-to-end: `i32.load` → `i32.ctz` /
`i32.clz` → `br_if` consumes the i32 condition directly. No
`i32.wrap_i64`, no `i64.extend_i32_u`, no per-side widening — the
cheapest path the codegen offers. Every other strategy
(`MaskShift`, `ModPrime`, `RotLow`) feeds a `br_table` whose index
lookup pulls at least one i64 path in EOP (table offset arithmetic);
`BitTest` skips `br_table` entirely.

**Generator.** Enumerate `bit ∈ [0..31]`; check whether
`(h₀ >> bit) & 1 ≠ (h₁ >> bit) & 1`. Prefer `bit = 0` (Ctz form) or
`bit = 31` (Clz form) when either works, since those forms reduce to 2
wasm ops via the existing peepholes in `src/codegen/instrList.ml`.

**Real-world hit.** `mo:core/Result` (`{ #ok; #err }`) —
`hash("ok") = 0x611C` (LSB 0), `hash("err") = 0x4D0765` (LSB 1). Every
Result switch dispatches in 3 wasm instructions:
`i32.load offset=H; i32.ctz; br_if Lerr`. Given Result is *the*
canonical 2-arm variant, this is high-frequency.

**Threshold.** n = 2 specifically — fills the gap below the existing
`max(64, 4n)` Gosper threshold. Same-body merging may bring an
effectively-2-arm case from a larger arm count (e.g. a 5-arm
Result-like grouped into ok/err equivalence classes); BitTest applies
after merging.

**Interaction with peepholes.** This strategy assumes the existing
`[i64.and 1; (wrap_i64;) if]` → `[ctz; (wrap_i64;) if]` peephole
(PR #6103) plus a proposed sibling
`[i32.and 1; i32.eqz; br_if]` → `[i32.ctz; br_if]`
(and the symmetric `i32.clz` variant for MSB). Without the `br_if`-aware
peephole, the strategy still emits correct code but pays one extra wasm
instruction per dispatch site (the `i32.eqz`) until the peephole lands.

## Future Optimisation: Pre-shortening before Gosper's iteration

*(Subsumed by Multi-strategy search above — ModPrime and RotLow are the
concrete pre-shortening strategies described there.)*

## Non-goals

- Nested/wildcard patterns in variant arms (handled by `compile_pat_local`)
- `switch` on non-variant types
- JS backend (`moc.js`)
