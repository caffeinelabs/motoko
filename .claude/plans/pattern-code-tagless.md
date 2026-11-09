# Plan: Finally-tagless pattern code with operational primitives

*Status: draft, to be fleshed out. An alternative to "Step A" of
"Finally-tagless pattern code and switch-specialised interpreters" in
[variant-switch-br-table.md](variant-switch-br-table.md) (PR #5927).*

## Motivation

Codegen decides how to compile a `switch` by inspecting the IR on the
side, with predicates that guess what `fill_pat` will emit:

| IR predicate (`src/codegen/compile.ml`) | decides |
|---|---|
| `flatten_tag_leaves` (#5927) | whether a `br_table` dispatch applies, and on which hashes |
| `tuple_switch_arity` (#6486) | whether the scrutinee tuple is compiled unboxed |
| `simplify_cases` | dropping the second tag test of a 2-label variant |
| `single_case` | the one-arm `switch` fast path |
| `Ir_utils.is_irrefutable_nonbinding` | dropping a sub-scrutinee instead of matching it |
| `known_tag_pat` (`TagP ("", p)`) | an IR *rewrite* telling `fill_pat` to skip a decided tag test |

Each is a separate, partial model of the same thing: what the matching
code does. They are written against IR *shapes*, so every new shape
(wildcard arms, shared tags, a variant in a tuple component, a capturing
arm) needs another predicate or another fallback to the slow path.

The pattern-code EDSL (`patternCode`, `(^^^)`, `orElse`, `with_fail`)
already *is* that model, but it is opaque:
`CannotFail of G.t | CanFail of (G.t -> G.t)` hides the operations in
`G.t` blocks and the control flow in a closure over the failure
continuation.

## The idea

Make the `(^^^)` language itself finally-tagless, and give it the
**operational** primitives `fill_pat` actually emits: load the scrutinee
local, read the tag hash, compare with a constant, project a payload,
load tuple component `k`, test for `null`, project an option, branch to
the failure continuation. Then interpreters can see everything `fill_pat`
does, and the side predicates above can be derived instead of guessed.

This differs from Step A of the master plan, whose primitives are
pattern-level (`test_tag : path -> string -> t`) and which needs
`fill_pat` to compute and pass access paths. Here nobody passes paths:
they are the **abstract values** an analysing interpreter computes for
the stack slots.

`fill_pat` stays: it is the one walk that translates `Ir.pat` into a term
of this language. What goes are the predicates that inspect the IR
*besides* that walk.

## Signature sketch

Stack-indexed, so that an interpreter cannot lose track of the Wasm
stack (phantom stack shapes, as in a typed stack machine):

```ocaml
module type PAT_CODE = sig
  type ('s1, 's2) t            (* code taking stack 's1 to 's2 *)
  type 's v                    (* a value on top of 's *)

  (* the (^^^) algebra *)
  val nop       : ('s, 's) t
  val seq       : ('a, 'b) t -> ('b, 'c) t -> ('a, 'c) t      (* (^^^) *)
  val fail      : ('a, 'b) t                                 (* definiteFail *)
  val or_else   : ('a, 'b) t -> ('a, 'b) t -> ('a, 'b) t      (* orElse *)

  (* opaque code: arm bodies, anything from compile_exp *)
  val opaque    : effect:('a, 'b) stack_effect -> G.t -> ('a, 'b) t

  (* operational primitives *)
  val get_local : local -> ('s, 's * ptr) t
  val set_local : local -> ('s * 'x, 's) t
  val drop      : ('s * 'x, 's) t
  val tag       : labels:Type.lab list -> ('s * ptr, 's * hash) t
  val eq_hash   : int64 -> ('s * hash, 's * bool) t
  val payload   : ('s * ptr, 's * ptr) t
  val tuple_get : int -> ('s * ptr, 's * ptr) t       (* Tuple.load_n *)
  val is_some   : ('s * ptr, 's * bool) t
  val opt_proj  : ('s * ptr, 's * ptr) t              (* Opt.project *)
  val lit_eq    : Ir.lit -> ('s * ptr, 's * bool) t   (* RTS call for Nat/Text *)
  val guard     : ('s * bool, 's) t                   (* false -> failure continuation *)
  val fresh_local : string -> local                   (* see "Locals" *)
end
```

The exact primitive set is open (see "Open questions"). Two points are
deliberate:

- `tag` carries the variant's label set. That is *type* information, not
  IR shape, and it is what lets facts like `Not {l1}` imply `Is l2`.
- `opaque` must declare its stack effect. Arm bodies stay `G.t` blobs;
  the analysis treats their result as unknown, but still knows the stack.

## Interpreters

1. **Emit.** `('a, 'b) t = patternCode`. Each primitive emits what
   `fill_pat` emits today. The output must be byte-identical: that is the
   gate for introducing the signature at all.
2. **Analyse.** Abstract interpretation of the stack machine. Abstract
   values are *access terms*:

   ```ocaml
   type acc =
     | Root                         (* the scrutinee *)
     | Comp of acc * int            (* tuple_get *)
     | Payload of acc               (* payload, under the facts that guard it *)
     | Some_ of acc                 (* opt_proj *)
     | Tag of acc                   (* tag hash of acc *)
     | Opaque of int                (* fresh atom: anything the analysis can't see *)
   ```

   `eq_hash h` on `Tag a` followed by `guard` *is* a tag test on `a`; the
   control-flow tree from `seq`/`or_else`/`guard` gives the success and
   failure edges. Analyse records, per arm, the tests it performs and the
   uses of each access term, and performs `Match_arm` for the `Dispatch`
   handler.
3. **Specialised emit**, built by Analyse per edge. Emit plus a fact
   environment `acc ↦ Is l | Not s` (and literal facts later): a decided
   `eq_hash` + `guard` becomes `nop` or `fail`, everything else
   delegates. This replaces `known_tag_pat`.

## What derives from the interpretation

| today | derived from |
|---|---|
| `flatten_tag_leaves` | `eq_hash`/`guard` on `Tag Root`, alternatives under `or_else` |
| `tuple_switch_arity` | no arm uses `Root` as a whole value (only `tuple_get` on it): compile the scrutinee unboxed |
| boxing a tuple that *one* arm binds whole (roadmap step 2 of #6464) | that arm stores `Root`: rebuild (or reuse the pointer) *in that arm* only |
| `simplify_cases`, `single_case` | facts: `Not` all labels but one ⇒ `Is` that one, via `tag`'s label set |
| `is_irrefutable_nonbinding` | a projected value that is only `drop`ped is dead: elide it |
| `known_tag_pat` | Specialised emit |
| repeated loads (below) | the use table of access terms |

## Stateful, retroactive getter sharing

The same access term showing up twice (the tag hash of `Root` in every
arm of a linear chain, a payload projected in two arms, a component
reloaded per arm) is a common subexpression. Access terms are value
numbers, so this is the value numbering of roadmap step 3 in the #6464
review, coming out of the same domain.

The interpreter collapses repeated getters into one `local.tee` and
several `local.get`s. The decision is retroactive: when the first getter
is emitted, nobody knows yet whether a second follows.

### Two mechanisms

1. **Two passes (preferred to start).** Analyse runs first anyway. It
   builds a use table `acc ↦ (count, occurrences)`; Emit consults it:
   the first occurrence emits `getter; local.tee l`, later ones
   `local.get l`, single uses stay plain. Retroactive in effect, and the
   table is directly testable.
2. **Single pass with promises.** `G.t` is `depth -> pos -> instr list ->
   instr list`, and depth labels are already `Lib.Promise`s fulfilled at
   materialisation (`instrList.ml`: `new_depth_label`, `remember_depth`,
   `branch_to_`). A getter can likewise be emitted as a hole resolved at
   the end to `getter` or `getter; local.tee l`, allocating `l` only once
   a second use appears. One interpreter, but more weight on `G`'s
   deferred machinery.

### Cost rule

For a getter of cost `c` (instructions) used `k` times, use a local iff

    k·c  >  c + 1 + (k − 1)        (one tee, k−1 gets)

The case `c = 1` (a single `LocalGet`/`Const`) never pays, which is the
`immut_local` idea of PR #6121 (closed unmerged): don't allocate a local
when re-reading is free. Both directions are one rule here, applied by
the interpreter instead of an opt-in helper.

### Soundness

- **Immutable sources only.** Tag hashes, tuple components, variant
  payloads, option contents, immutable record fields. A `var` field or
  array element read is a fresh `Opaque` atom every time.
- **Dominance.** The `tee` must execute before every `get`. Arm 2 is
  reached via arm 1's failure edge, and arm 1 may have failed *before*
  its load. The `seq`/`or_else`/`guard` tree gives the dominators: place
  the write at the lowest common dominator of the uses.
- **Hoisting must not fault.** Moving a getter above the test that
  guarded it is safe only if it cannot trap without that test: a
  variant's payload slot exists for every variant, so `payload` can
  hoist; `opt_proj` before `is_some` cannot. Start with "first occurrence
  dominates the rest" plus hoisting non-faulting getters into the switch
  prologue. General placement is partial redundancy elimination, out of
  scope.
- **Heap pointers in locals.** No worse than today: `let`-bound locals
  already hold heap pointers across arbitrary code. Pattern code contains
  no `await`, and the shared range ends where an opaque arm body begins.
  Tag hashes are scalars.

## Locals

`fill_pat` allocates locals eagerly, outside the `CanFail` closure, in
its `OptP`, `TagP`, `TupP`, `ObjP`, `AltP` and `AndP` cases. Under
Analyse that would allocate dead Wasm locals. Hence `fresh_local` in the
signature: Analyse hands out symbolic locals, the emitters allocate (and
getter sharing may make some of them unnecessary).

## Relation to the master plan

- Same three interpreters and the same per-edge specialisation as
  "switch-specialised interpreters" in the master plan; the difference is
  the primitive level.
- The master plan's "access paths are the real refactor" pitfall goes
  away: paths are computed, not threaded.
- Additionally covers lazy boxing (step 2), getter sharing / value
  numbering (step 3) and the `immut_local` decision, which the
  pattern-level variant does not see.
- Costs more up front: the whole `patternCode` surface, not just tag
  tests, moves behind the signature.

## Open questions

- **Primitive granularity.** One `tag` primitive, or its parts (forwarding
  pointer, field load)? Coarser keeps the domain small; finer exposes
  more sharing.
- **Stack typing in OCaml.** Phantom-indexed `('s1, 's2) t` with
  products for stack shapes, or a GADT with real stack descriptors that
  Analyse can match on? The latter lets the abstract stack be checked;
  the former is cheaper.
- **Access-term identity.** Structural equality first, hash-consing once
  facts and interpreters are memoised (the DAG step).
- **Scope.** `patternCode` is also used for `let` patterns,
  `compile_unboxed_pat` and function parameters. All of them must go
  through the signature, but only `switch` needs Analyse at first.
- **Literals.** `lit_eq` on `Nat`/`Text` is an RTS call; facts about it
  are `Is`/`Not` on literals, same shape as tags.

## Migration

1. Signature + Emit; `fill_pat`, `orElse`, `with_fail` and friends
   through it. Byte-identical output; all `.ok` files unchanged.
2. Analyse; the switch recognizer uses it instead of `flatten_tag_leaves`.
   Same `Dispatch` plans.
3. Specialised emit replaces `known_tag_pat`.
4. Derive `tuple_switch_arity` and lazy boxing in capturing arms.
5. Getter sharing (two-pass), with the cost rule.
6. Facts on failure edges; memoised interpreters (decision DAGs).

Steps 1–3 change no emitted code; 4–6 are where the wins are, each
measurable on `test/bench/variant-switch.mo` and
`test/run/switch-tuple-noalloc.mo`.

## Key code locations

- `src/codegen/compile.ml`: `PatCode` (`patternCode`, `(^^^)`,
  `orElse`, `with_fail`), `fill_pat`, `SwitchE`, `Dispatch`,
  `flatten_tag_leaves`, `tuple_switch_arity`, `simplify_cases`,
  `known_tag_pat`
- `src/codegen/instrList.ml`: `G.t`, depth promises
- `src/ir_def/ir_utils.ml`: `is_irrefutable_nonbinding`
