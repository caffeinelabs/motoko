# Plan: make `Principal` a supertype of every `actor { … }`

**Status:** design / cost estimate (not started)
**Author:** drafted 2026-07-01

## Goal

Add the subtyping rule `actor { … } <: Principal` so that any actor reference can be
used where a `Principal` is expected (upcast to its canister-id principal).

- **One-directional.** `Principal ⊀ actor { … }` — you cannot call methods on an
  arbitrary principal, so the downcast stays explicit and unsafe
  (`actor "…"` / `Prim.actorOfPrincipal`).
- Equality remains representation-based (already blob-compares actors and principals).

## Why this is cheaper than a typical subtyping feature

Actors and principals are **both `Vanilla` tagged blobs** holding the same bytes
(the canister id); they differ only by heap **tag** (`A` vs `P`).
`Prim.principalOfActor` is literally `Blob.copy env A P` — a re-tag
(`compile_enhanced.ml` ~12741; classical ~12410).

Most `Principal` consumers already treat the two alike:

- **Equality**: `Prim (Blob|Principal) | Obj (Actor,_,_) -> Blob.compare` (`compile_enhanced.ml:11523`).
- **Serialize / deserialize / `potential_pointer` / size / write** dispatch on the
  **static Motoko type** and already group `Obj (Actor,_,_) | Prim Principal`
  (`compile_enhanced.ml:7781, 7963, 7965`).
- Some tag dispatch already handles `Tagged.P | Tagged.A` together (`compile_enhanced.ml:3856`).

**Consequence:** the upcast can very likely be **coercion-free (identity)** — no re-tag
at the upcast site. This is essential: Motoko subtyping is **coercion-free by design**;
there is no infrastructure to insert coercions at upcast sites, and building it would be
the expensive path. If some consumer cannot tolerate an `A`-tagged value where a
`Principal` is statically expected, the re-tag fallback already exists
(`principalOfActor`), but needing it would be a red flag.

## Implementation

1. **Subtyping rule** — `src/mo_types/type.ml`, `rel_typ` (~L1282).
   Add `| Obj (Actor, _, _), Prim Principal when rel != eq -> true`.
   ~2 lines. Guard to the sub-direction only (not `eq`, not the reverse).

2. **`lub` / `glb`** — `src/mo_types/type.ml`.
   - `lub (actor, Principal) = Principal`
   - `lub (actorA, actorB)` for **distinct** service types now `= Principal` (today → `Any`)
   - `glb (Principal, actorX) = actorX`
   ~10–30 lines; care around the existing compatible-actor `lub`.

3. **Runtime coercion-freeness audit — the main effort.**
   Both backends (`compile_enhanced.ml` *and* `compile_classical.ml`) and the
   interpreters. Verify every `Principal`-typed operation tolerates an `A`-tagged value:
   equality ✓, ser/deser (type-driven) ✓, hashing, stable serialization,
   `debug_show` (type-driven → renders as `principal "…"`; confirm that is the desired
   rendering for an upcast actor). Most sites already handle both tags, so this is
   **verification-heavy, edit-light**.

4. **Interpreter parity** — `src/mo_interpreter/interpret.ml`,
   `src/ir_interpreter/interpret_ir.ml`, `src/mo_values/value.ml`.
   Make the subtype an identity on interpreter values. (These are exactly the
   currently-dark interpreter files — see the `SKIP_RUNNING` coverage caveat below;
   they must actually be exercised to trust the change.)

5. **Candid soundness check** — `src/mo_idl/mo_to_idl.ml`, `idl_to_mo.ml`.
   Candid does **not** make `service <: principal`. Ensure Motoko never uses the new
   rule to *justify a Candid interface subtyping* (service-import typing, actor-class
   upgrade compatibility). Investigation-heavy; may need an explicit guard so the rule
   stays **Motoko-internal only**. This is the biggest unknown.

6. **Docs + changelog** — subtyping section of the language manual; note the `lub`
   widening (distinct actors now unify to `Principal`).

## Testing

- **tc / fail** (`test/run`, `test/fail`): upcast accepted in let / arg / array / return;
  **reverse rejected** (`Principal ⊀ actor`, soundness); non-actor `object`/`module`
  still `⊀ Principal`; `lub`/`glb` cases (`if c then actorA else actorB → Principal`,
  `[actorA, principalX]`).
- **run-drun, both persistence modes**: pass an actor where a `Principal` param is
  expected across a canister call; `actor == principal`; `debug_show` of an upcast;
  serialize → deserialize as principal; a **stable `Principal` var holding an upcast
  actor across upgrade**.
- **mo-idl** (`test/mo-idl`): generated interface for a service with `Principal` params
  fed by actors is correct.
- **Interpreter parity**: run / run-ir / run-low phases exercise the identity subtype
  under interpretation (see coverage caveat).
- **Full-suite regression**: the `lub` widening will shift some inferred types /
  error-message goldens in existing tests — expect and reconcile golden churn. This is
  the least predictable cost.

## Risks / decisions

1. **Coercion-free vs coercive** — everything hinges on the tag audit. Coercive fallback
   exists (`principalOfActor`) but needs upcast-site insertion infra we do not want to build.
2. **`lub` widening** — distinct actors → `Principal` silently loses a common service
   interface and changes inference; deliberate design choice with source-compat churn.
3. **Candid mismatch** — keep the rule Motoko-internal; do not leak it into Candid
   interface subtyping.
4. **`debug_show` rendering** of an upcast actor (`principal "…"` vs `actor …`) — pick and document.

## Cost estimate

- **Implementation:** type rule trivial; `lub`/`glb` small; the runtime tag audit across
  two backends + interpreters is the bulk (moderate, ~a few focused days); the Candid
  soundness investigation (item 5) is the biggest unknown and could dominate. Total code
  delta is modest — the effort is auditing and proving soundness, not writing lines.
- **Testing:** ~6–8 new test files across run / fail / run-drun / mo-idl (× persistence
  modes) plus a full-suite golden reconciliation for the `lub` change.

## Coverage caveat (relevant to trusting the interpreter changes)

The `coverage` build runs with `SKIP_RUNNING=yes`, which disables the `moc -r` interpret
phases (`run` / `run-ir` / `run-low`), so `interpret.ml`, `interpret_ir.ml`,
`mo_values/prim.ml`, `operator.ml` are largely dark in the coverage report. The
interpreter-parity work here (item 4) will not show up as covered unless the interpret
phases are enabled — worth splitting `SKIP_RUNNING` so the cheap OCaml `-r` phases run.

## Grounding references (as of 2026-07-01, `gabor/coverage-tests`)

- `src/mo_types/type.ml`: `rel_typ` ~L1225/1282 (subtyping); `principal = Prim Principal` L354;
  `is_actor` L625.
- `src/codegen/compile_enhanced.ml`: `principalOfActor`/`actorOfPrincipal` re-tag ~L12741;
  equality grouping L11523; ser/deser + `potential_pointer` grouping L7781/7963/7965;
  `P | A` tag handling L3856.
- `src/mo_values/prim.ml`: `principalOfActor` L352.

## Progress & open follow-ups (2026-07-02)

The Candid side that *sanctions* this rule is now a draft PR: **dfinity/candid#748**
(`service <actortype> <: principal`) — spec + reference subtype checker (`subtype.rs`) +
decoder (`de.rs`) + `.test.did` tests + a machine-checked metatheory in `coq/MiniCandid.v`
(`ServiceT <: PrincipalT` modelled on `NatT <: IntT`; `soundness`/`transitive_coherence`
re-verify). Marshalling is unchanged — service and principal references are byte-identical
on the wire — so it is a conservative, decode-only widening.

- **Blocked on #748 landing** before moc adopts the rule broadly (so moc isn't ahead of Candid).
- **EOP runtime gap**: `rts/motoko-rts/src/idl.rs` `memory_compatible` needs a
  `(IDL_CON_service, IDL_REF_principal) => variance != Invariance` case, else `actor → Principal`
  stable-var upgrades trap on enhanced persistence. Verified locally; land in lockstep with #748.
  Then flip `test/run-drun/actor-sub-principal-stable.drun` off `# CLASSICAL-PERSISTENCE-ONLY`.
- Still pending here (see above): interpreter parity, `lub`-widening golden reconciliation, docs/changelog.

### Open issue idea (parked — do NOT file upstream yet; one PR is enough noise while #748 is in review)

Port dfinity/candid's `coq/` metatheory to **Rocq 9.1.1**. It currently builds only on `coq_8_18`
(the re-pin in #748). The port needs: `Require Import Coq.*` → `Stdlib.*` (or `From Stdlib Require`),
fix the `FunInd` relocation, bump the nix pin to a modern nixpkgs + Rocq, and re-verify the proofs
across the 8.18 → 9.1 gap. Orthogonal to `service <: principal` (toolchain, not content). Raise as its
own issue/PR later, or fold into #748 only if a maintainer asks.
