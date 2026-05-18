# GADTs for Motoko Variants

## Goal

Allow each arm of a parametric variant to **refine** the type parameter(s), so
that pattern-matching can recover the refined types in the case body. Target
use-case: a type-safe tagless evaluator over an `Expr<A>` AST, where each
constructor's arm constrains `A` to the type its value carries.

Multi-parameter generics like `Pair<A, B>` are supported — the syntax is
comma-separated `type` clauses (e.g. `type A = Nat, type B = Bool`), and the
implementation handles arbitrary arity uniformly via list-based data
structures throughout the typechecker and side-tables.

```motoko
type Expr<A> = {
  #int  : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #add  : type A = Nat in (Expr<A>, Expr<A>);
  #if_                      : (Expr<Bool>, Expr<A>, Expr<A>);
  #eq   : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>);
};

func eval<A>(e : Expr<A>) : A = switch e {
  case (#int n)            n;
  case (#bool b)           b;
  case (#add (x, y))       eval(x) + eval(y);
  case (#if_ (c, t, e))    if (eval c) eval t else eval e;
  case (#eq (cmp, x, y))   cmp(eval x, eval y);
};
```

No `assert false`, no coercion, no boxing — `eval` returns the right type per arm.

## Syntax

### Variant arm grammar extension (Flavor B4)

A variant arm gains an optional **type clause** after the `:`, separated
from the payload by `in`. Same shape as a TypD top-level `= constraints
in body` — single mental model across both sites.

```
arm  ::=  '#' Ident (':' Payload-or-Constrained)?
Payload-or-Constrained ::= Type
                        | type-clause 'in' Type
type-clause ::= type-decl (',' type-decl)*
type-decl   ::= 'type' Ident '=' Type   // refinement: outer X is set to Type
             | 'type' Ident             // existential: introduce fresh X
```

Examples:

```motoko
#int  : type A = Nat in Nat                    // refinement: A ≡ Nat
#bool : type A = Bool in A                     // refinement; payload uses A
#if_                                            // no payload (unit)
#plain : Text                                   // no clause = parametric
#eq   : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>)
```

### Semantics

`type X = T` introduces a **scoped type alias** `X` bound to `T` for the lifetime
of (a) the rest of the arm declaration and (b) the body of every `case (#tag …)`
that matches this arm.

`type X` (no RHS) introduces a fresh type variable that may appear in the payload
but does **not** appear in the outer-parameter slot — the existential.

No clause at all = the arm is parametric in the outer type parameter, identical
to today's variant typing.

### What carries across to the case body

```motoko
case (#eq (cmp, x, y)) <body>
```

In `<body>`:
- `A` is locally bound to `Bool` (the refinement).
- `B` is locally bound to a fresh abstract type (the existential).
- `cmp : (B, B) -> Bool`, `x : Expr<B>`, `y : Expr<B>` (under the refinement).
- Code must be parametric in `B` — i.e., uses of `B`-typed values cannot peek at
  `B`'s identity, only pass them around or feed them to `B`-consuming helpers.

## Type system: refinement as substitution

The mechanism is **first-order substitution** (Robinson's algorithm), no fancy
unification needed. Mental model: each typing context carries a substitution
`Σ : TypeVar → Type`.

| Concept | Linear-algebra analog |
|---|---|
| Free type variable | Column |
| `type X = T` clause | Pivot row: `X − T = 0` |
| `type X` (no RHS) | Free column, no row |
| Parametric arm (no clause) | Empty row addition |
| Σ at a typing point | Reduced row-echelon state |
| Inconsistent refinement | Contradictory row → arm unreachable |
| Type-variable ambiguous after elimination | Free column → existential survives |

### Per case-arm

```
Σ_inner = mgu(Σ_outer, arm.refinement)
```

If `mgu` succeeds, enter body with `Σ_inner`. If it fails (contradictory equation
between the scrutinee's known refinement and the arm's), the arm is **statically
unreachable** and can be omitted from the switch without breaking exhaustiveness.

### Coverage / exhaustiveness

For a switch on `e : Expr<T>`:
- For each declared arm, attempt `mgu(Σ_current, arm.refinement)`.
- If unifiable → arm is reachable; must be matched.
- If contradictory → arm is unreachable; may be omitted.

```motoko
func dual(e : Expr<Bool>) : Expr<Bool> = switch e {
  case (#bool b)         #bool (not b);
  case (#if_ (c, t, e))  #if_ (c, dual e, dual t);
  case (#eq (cmp, x, y)) #eq (cmp, y, x);
  // #int and #add unreachable: their type A = Nat is incompatible with Bool
};
```

The typechecker accepts this without `#int`/`#add` because their refinements
contradict the scrutinee's `Expr<Bool>` type.

### Occurs check

`type A = Expr<A>` would put `A` on both sides — reject as infinite type.
Motoko isn't equirecursive; the natural rule applies.

## Interactions with existing Motoko

### Construction-side inference

Resolved by existing **variant subtyping**, no new bidirectional rule needed.
`#int 5 : {#int : Nat}` (minimal), and `{#int : Nat} ⊑ Expr<Nat>` by structural
variant subtyping (since `Expr<Nat>` is the structural variant whose `#int` arm
contains a `Nat`).

This means: when the typechecker looks for a supertype, the refinement clauses
are consulted to compute what `Expr<Nat>` structurally is — the union of all
arms whose refinement is compatible with `A = Nat`. The arm refinements act as
filters during expansion of `Expr<T>` to its structural form.

### Subtyping vs equality at refinements

For v1, `type A = T` means *equality* `A ≡ T`. No upper/lower bounds. Keeps Σ a
single substitution. Bounded refinement (`A ⊑ T`) would need lattice tracking
and is out of scope.

### Bounded polymorphism interaction

Motoko's `<A <: Bound>` resolves most subtyping-from-caller use-cases without
needing variance machinery on the GADT itself. E.g., `func f<A <: Bool>(e : Expr<A>)` 
works without asking whether `Expr<Bool> ⊑ Expr<Any>`.

### Variance

Deferred — sidestepped in practice by bounded polymorphism. When the question
forces itself (e.g. `List<Expr<A>>` nested inside another variant generic that
needs variance), revisit. Most likely answer: `Expr<A>` is invariant in `A`
because arms like `#if_ : (..., Expr<A>, Expr<A>)` use `A` covariantly *and*
arms like `#eq` payload uses an existential `B → Bool` (contra-covariant). But
this only matters when nested in another generic.

### Erasure

Motoko is type-erased. Existentials cost nothing at runtime; the `B` of `#eq`
is gone after typechecking. The packed `cmp` closure has been monomorphized for
the actual `B` chosen at construction. No witnesses, no Refl, no extra fields.

### Candid

GADTs slot into Motoko's existing Candid machinery — **no protocol-level
change, the Candid spec is untouched**. All GADT-aware logic lives on the
Motoko side: deciding *which* type to hand to the Candid encoder/decoder,
not changing what the encoder/decoder does with it. Defaulting, subtyping,
optional / record / variant wire rules — all unchanged. The pruned forms
we produce must be expressible in the existing Candid type grammar (they
are: recursive variants, no new constructors needed).

**Outgoing: existentials are unshareable, whole-type.** Any type whose
declaration mentions an existential ("black-hole type") fails the existing
`is_shared` check at message boundaries. This is *whole-type, not per-arm*:
if `Expr<A>` has one arm with an existential clause, `Expr<X>` is unshareable
for every `X`, even when in practice you only construct refinement-only arms.
This matches how Motoko already treats records with one non-shareable field —
the type is the unit of shareability, no per-instance cleverness.

Refinement-only declarations stay fully shareable.

**Outgoing wire format: pruned per instantiation.** For a shareable GADT
`Expr<Bool>`, mo_to_idl emits the *pruned* variant — arms whose refinement
clauses conflict with the receiver's instantiation are dropped. Same mechanism
as M5 coverage pruning, just applied at IDL generation. So `Expr<Bool>`
exports as `{#bool : Bool; #if_ : ...}` rather than the full declaration.

**Incoming: receiver's pruned form drives Candid's existing decoder.** No
new runtime type-table check is needed; the existing Candid wire-vs-expected
subtype check does the work, provided the *expected* type fed to the decoder
is the receiver's refinement-pruned form. A wire arm that doesn't appear in
the pruned form fails Candid's variant-tag check and traps — exactly the
right behaviour.

**Receiver parameters are fixed.** `from_candid : Expr<Bool>` locks `A = Bool`;
there is no inference. Pruning is a syntactic walk over the declared variant
against a known instantiation.

**Recursion and the `#if_`-style arm.** An arm without its own refinement
clause but whose *payload* mentions the GADT recursively forces per-
instantiation pruning to be honest:

```motoko
type Expr<A> = {
  #int  : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #if_  : (Expr<Bool>, Expr<A>, Expr<A>);   // no clause, payload refers back
};
```

`Pruned[Expr<Bool>]` is recursive: `μX. {#bool : Bool; #if_ : (X, X, X)}` —
`#if_`'s payload mentions `Expr<Bool>` again, which is the recursive knot.

Three things this forces:

1. **Per-instantiation, not in-place.** Mutating `Expr`'s `T.Def` body to its
   pruned form would corrupt `Expr<Nat>`'s pruning (which keeps `#int`, drops
   `#bool`). `gadt_prune_for_coverage` already does per-call pruning; the
   IDL/Candid layer must commit to a fresh pruned form per `Con(Expr, ts)`.

2. **Mixed instantiations in one payload.** `Expr<Nat>`'s `#if_` arm is
   `(Expr<Bool>, Expr<Nat>, Expr<Nat>)`. The Candid type table carries *both*
   `Pruned[Expr<Bool>]` and `Pruned[Expr<Nat>]` as distinct recursive defs.
   mo_to_idl must not dedupe on source cons alone.

3. **Termination.** Walking `Pruned[Expr<Bool>]` recursively needs memoisation
   keyed by `(c, ts)` so the inner recursive reference reuses the outer one,
   otherwise the pruner loops.

**.did file consequence.** External (non-Motoko) clients see the *pruned*
declaration. A non-Motoko sender that conforms to the published .did
literally cannot encode an unreachable arm — there's no `#int` tag in
the published wire type for `Expr<Bool>`. Adversarial / malformed wire
data is what would trip the Candid decoder, and trap is the right
response. Document this explicitly: the .did is refinement-aware.

**Test matrix** (M12 captures):

- Outgoing positive: refinement-only `Expr<Bool>` crosses an actor
  boundary cleanly, with `#bool` and recursive `#if_` payloads.
- Outgoing negative: any `Expr<X>` (for any `X`) carrying an `#eq`-with-
  existential arm rejected by `is_shared`, error message contains
  "black-hole".
- Outgoing negative: M10 `type Tup = type X in ...` as message arg,
  same rejection. Existential nested inside array / tuple / opt /
  record field / function return — each path through `is_shared`
  exercised.
- Incoming positive: `from_candid : Expr<Bool>` on a wire `#bool true`
  succeeds; recursive `#if_(b1, b2, b3)` succeeds.
- Incoming negative: `from_candid : Expr<Bool>` on wire data carrying
  an `#int 5` tag — traps. Symmetric for `Expr<Nat>` with a `#bool`
  arm on the wire.
- Mixed-instantiation positive: a value of `Expr<Nat>` whose `#if_`
  payload contains `Expr<Bool>` nested inside, all pruning consistent
  end-to-end.

### `switch type T` unification

The static GADT refinement (`type A = T` on variant arms, case-discriminating at
compile time) and the planned `switch type T { case Int … }` (from
`shared-generics.md`, runtime type-case) are **the same mechanism at different
phases**. Both refine a type variable's identity via tag-driven dispatch:

- **Static**: refinement happens at typechecking; the tag is the variant tag;
  the body type-checks under the refined Σ.
- **Dynamic**: refinement happens at runtime; the value carries an abstract
  type T; the case body sees T concretised.

Same Σ substitution machinery is reused; build it once, deploy in both
contexts. Specific integration point: when `switch type` arrives, its
case-emission can lower to the same internal "extend Σ, type-check body under
Σ" routine.

## Implementation sketch

### AST / IR

Variant arm declarations grow a `type_clause : (Ident * Type option) list`
field, defaulting to `[]` for current code.

```ocaml
type variant_arm = {
  tag : Ident.t;
  type_clause : (Ident.t * Type.t option) list;  (* (X, Some T) = refinement; (X, None) = existential *)
  payload : Type.t;
}
```

### Typechecker

Thread an explicit substitution `Σ` through the type-checking environment, or
add it as a derived field on `Env.t`. Two options:
- **Additive**: new `subst : Type.subst` field on `env`. Cheap to add, minimal
  churn elsewhere.
- **Folded**: extend the existing type-binding map to carry equations. Unifies
  code paths but touches more files.

The additive route is the safer v1.

At each case arm:

```
1. expected = scrutinee.type     // e.g. Expr<Bool>
2. arm_refinement = match expected with
   | Expr<U> -> Σ_local = compose Σ ((A, U) :: arm.type_clause)
3. if not (unifiable Σ_local) then mark arm unreachable
4. else recurse into body with env extended by Σ_local
```

### Code generation

Nothing new. Erasure means the codegen sees only the structural variant
representation; refinements are typechecker-only. The variant-dispatch
machinery (br_table or linear) operates on tags as before.

## Milestones

- [x] **M1 — Parser & AST**: variant arms accept `type X = T` / `type X` clauses,
      comma-separated. AST gains `typ_tag'.constraints` and `typ_constraint'`.
      Branch: `gabor/gadt`, commit `4085892ec`.

- [x] **M2 — Construction-side eager check**: side-table `gadt_arm_constraints`
      in `Type`, populated by `TypD` handler. `check_exp` consults at variant
      construction against an expected `Con(c, ts)` type and rejects with M9001
      if an arm's refinement is incompatible with the slot.
      Commit `fa9abc5f3`.

- [x] **M3 — Pattern-match side env-transformer**: `gadt_sigma_for_case`
      distills refinements into `Σ : T.ConEnv.t`; `check_case` applies Σ to
      `env.vals`, the pattern's `ve`, and the expected return type. A complete
      `func eval<A>(e : Expr<A>) : A` over a GADT typechecks.
      Strictly additive — empty Σ short-circuits to the pre-existing behavior.
      Commit `80006ae92`.

- [x] **M4 — Existentials**: `type X` (no RHS) brings a fresh skolem (abstract
      con) into scope for the payload at declaration time and into the case
      body at pattern match. Construction-side does ad-hoc inference of the
      existential's witness from the inferred payload type.
      - **M4-decl** (commit `69ff492b4`): skolem cached on `typ_constraint`
        AST node (`(typ_constraint', Type.con option) annotated_phrase`)
        for cross-pass `eq_kind` stability. Declaration with `type B`
        elaborates without M0029.
      - **M4-use**: `T.unify_existentials` structurally walks expected vs
        actual to extract witnesses for each existential con, substitutes,
        then sub-checks. `T.gadt_arm_existentials` side-table mirrors the
        refinement table. New error code M9002 for unification failure.
        `#eq (natEq, #int 5, #int 5) : Expr<Bool>` now typechecks; pattern
        match in eval returns the right type per arm.
      - **Known gap**: when `#eq (...)` is the bare argument to a generic
        function (no explicit type ascription), Motoko's inference doesn't
        yet propagate the GADT refinement upward to instantiate the type
        parameter. Workaround: ascribe `: Expr<Bool>` at construction.
        Tracked as a sub-item for a future milestone.

- [x] **M5 — Refinement-aware exhaustiveness**: implemented as **type-level
      pruning** before the coverage analyzer sees the variant.
      `T.prune_gadt_variant` filters out arms whose refinement is
      incompatible with the scrutinee's type-args; `gadt_prune_for_coverage`
      wires it into the SwitchE handler in typing.ml. Pleasant side effect:
      coverage error messages now show the *reachable* sub-variant instead
      of the full one, so the user sees only the cases that matter.
      Existing non-exhaustiveness warnings still fire for genuinely missing
      reachable arms. `desugar.ml` needs no change — pruning happens at the
      typechecker layer; lowering sees only the source-level cases.

- [x] **M6 — Multi-parameter GADTs**: turned out to need **no extra work** —
      the AST stores `constraints : typ_constraint list`, both side-tables
      key by `(con, lab)` returning lists, and the typechecker helpers
      (`gadt_check_refinements`, `gadt_sigma_for_case`,
      `T.prune_gadt_variant`) all iterate over the constraint list and look
      up slots in `tbs` by name. So `Pair<A, B>` with comma-separated
      `type A = Nat, type B = Bool` clauses works on both construction and
      pattern-match sides, and exhaustiveness pruning fires per-parameter.
      Verified with `/tmp/gadt-multi.mo`.

- [x] **M7 — Coverage analyzer fixup**: subsumed by M5. Type-level pruning
      hands `coverage_cases` a variant already restricted to the reachable
      arms, so the analyzer's existing logic transparently handles every
      coverage scenario:

      - Partial match over reachable arms → exhaustive (no false missing).
      - Explicit `case (#int _)` for `Expr<Bool>` (an arm with `type A = Nat`)
        → M0146 "this pattern is never matched", because the pruned variant
        doesn't contain `#int`.
      - Wildcard `case _` after a partial GADT match → reachable, no false
        unreached warning.
      - Or-patterns `case (#bool _ or #int _)` for `Expr<Bool>` → the `#int`
        leg gets M0146; the `#bool` leg is fine.

      No code change needed beyond M5.

- [ ] **M8 — Diagnostics: errors & warnings on `type` clauses** (in progress):
      - [x] **Duplicate clauses** (M9003 error): `type A, type A`,
            `type A = Nat, type A = Bool`, `type B, type B` — any same-name
            collision on a single arm. Subsumes "conflicting refinement"
            and "existential collision" since they're both duplicate-name
            symptoms.
      - [x] **Circular refinement** (M9006 error): `type A = Foo<A>` —
            AST-level walker scans the RHS for the name being defined.
      - [x] **Unused existential** (M9004 warn): `type B` whose name
            doesn't appear in the payload — genuinely dead code.
            (Refinements are *not* warned on, since they constrain the
            outer instantiation independently of the payload.)
      - [x] **Invalid refinement** (M9005 error): `type U = T` where `U`
            isn't an outer type-parameter of the enclosing GADT — the
            refinement has nothing to refine. Suggests fixing as an
            existential or removing the clause. When the payload *also*
            mentions `U`, the existing M0029 "unbound type" fires first
            during elaboration and masks M9005; both are informative.
      - [x] **Skolem escape — direct return** (M9007 error): after the
            case body is checked, walk `exp.note.note_typ` and reject if
            any of the arm's existential skolem cons appears in it.
            Catches `case (#eq (_, x, _)) x` where the body is just a
            variable whose type still mentions the skolem.
            **Known gap**: when the body wraps the skolem-typed value in
            a structure that the checker widens at sub-position (e.g.
            returning `(true, x)` against `(Bool, Any)`), the TupE's
            note_typ is set to the expected widened type, so the skolem
            disappears from the note before the check sees it.
            Operationally inert (Motoko erases types; the Any-typed value
            is opaque at runtime), but type-theoretically still a leak.
            Closing this gap would need a sub-expression walker.
      - [ ] **Shadowing warning**: `type A = T` where `A` shadows an outer
            name in a misleading way → optional warning, low priority.

- [x] **M10 — Existentials in tuples and records** (record-literal,
  tuple, and `object {...}` construction/destructuring done; `class`
  return-type form and projection-error guard remain deferred — see
  end of this bullet): lift `type X` clauses
      from variant arms to the top of any type definition. **Syntax: a
      comma-separated list of `type X` bindings, then `in`, then the
      body type.** `in` (already a keyword from `for (x in xs)`) reads
      naturally as "hide X *in* this type"; comma between bindings
      mirrors the variant-arm syntax:

      ```motoko
      type Tup = type X in (X, X -> Text);
      type Rec = type X, type Y in { value : X; size : Y };
      ```

      **Refinement (`type X = T in body`) is rejected at top level**:
      there is no outer type variable for the refinement to bind — at
      most it's a let-style substitution sugar (`X` everywhere becomes
      `T`), which doesn't need a new mechanism. The constraint can
      always be substituted in. Only existential `type X` clauses
      carry semantic content here.

      Semantics: `Tup` is an existential pack `∃B. (B, B → Text)`. Any
      concrete `B` can construct a value (`(5, Nat.toText)` → witness
      `B = Nat`); destructuring (`let (x, f) = tup`) introduces a fresh
      skolem `B` into the body's scope, with `x : B` and `f : B → Text`
      parametric.

      Work splits along the same M1–M9 axes:
      - **Parser/AST**: allow `type X (= T)?` clauses on `TypD`'s RHS, not
        just on variant arms. Probably a small extension to the grammar.
      - **Side-table**: extend or generalise — currently keyed by
        `(con × tag-label)`, would need a tag-less form keyed by `con`
        alone for non-variant types.
      - **Construction-side**: same `T.unify_existentials` machinery; just
        feed it from a non-variant payload.
      - **Destructure-side**: `let`-pattern over a tuple/record type with
        existentials brings a fresh skolem into scope (mirror M3's
        env-transformer).
      - **Exhaustiveness/coverage**: not applicable — these types have
        only one constructor.
      - **Diagnostics M8**: same suite applies.

      **Projection on existential-bearing types — shipped
      2026-05-16** (`test/fail/gadt-m10-projection.mo`). Cardelli's
      `open` discipline: bare `t.0` / `r.field` on a `Con(c, ts)`
      with non-empty `lookup_typd_existentials c` is rejected with
      M9010, pointing at the destructuring fix. Two guards: one in
      `ProjE` (typing.ml ~2335) on the pre-promote type, one in
      `try_infer_dot_exp` (~2755) for `DotE`. The two rejected
      alternatives — (a) internally rewrite into a block-with-
      destructure (`{ let (_0, _1, _2) = t; … }`), (b) reject and
      require the explicit destructure — we picked (b). Simpler
      implementation, clearer error messages, no rewrite-engine
      debt. The error suggests the form (`let (...) = t` for
      tuples, `let { f; ... } = r` for records) using the variable
      name when the receiver is a `VarE`.

      **Cross-skolem-mixing soundness (deferred to M11)**: today the
      schema-side existential `B` is cached on the AST and re-used by
      every destructure-site, so two destructures of the same type
      bind at the *same* skolem — type-checks but unsound under
      cross-mixing (`let (x1, f1) = t1; let (x2, f2) = t2; f1 x2` is
      accepted, crashes at runtime). The fix is fresh-per-site skolems
      on every destructure, which falls out naturally once M11 moves
      σ onto AST nodes. The same fix tightens variant-arm code: today
      two sibling `case (#eq ...)` arms also share the schema's `B`,
      reachable via the same cross-mixing path. Current test corpus
      doesn't exercise it.

      **Surprises landed during implementation:**

      - **`as_seq` normalises through Cons** — the FromCandidE desugar
        looked innocent (`T.as_seq t`) but internally called
        `normalize` and opened `Con(Expr, [Bool])` to its Variant
        body, losing the cons identity that the codegen's pruning
        needs. Fix was local (peek through normalize once, unpack
        only on `Tup`), but the failure mode was silent: pruning
        looked active everywhere downstream and just didn't fire.

      - **Multiple normalize sites in `Serialization`** — pruning at
        `type_desc` is necessary but not sufficient; the wire-format
        emitter has separate normalize calls in `buffer_size`,
        `serialize_go`, `deserialize_go`, plus an `add_idx` /`idx`
        pair, each independently keying the function-name hash off a
        type. All must agree on the pruned form or encoder and
        decoder synthesise inconsistent function names → bad wire
        format. `T.normalize_pruned` exists to make swap-in
        uniform.

      - **`tagE` and `tupE` synthesise their own IR notes** — those
        smart constructors compute the IR node's note from the
        children rather than copying the surface note. The σ
        registration only "fires" because the *outer* `typed_phrase'`
        in desugar preserves the surface region; the IR check then
        looks up σ at that outer region. If a future refactor were
        to push σ application down into the smart constructors,
        the lookup would suddenly miss.

      - **Variant-arm subtype check needs σ on the *promoted* form,
        not the surface note** — `T.subst σ t` on a `T.Con (Expr,
        [Bool])` doesn't recurse into the Cons's Def body, so the
        σ ends up applied to nothing observable. `T.subst σ
        (T.promote t)` is the working incantation; the first attempt
        silently passed σ through and produced no refinement.

      - **`unify_existentials` originally walked only `Tup`/`Func`/
        etc.** — extending to records meant adding `Obj` and
        `Variant` to the walker. Without that, the witness inference
        for records found no mappings and silently returned the
        empty σ, after which the sub-check rejected the concrete
        record as not-a-subtype of the existential-laden schema.

      - **The IR `Variant` deser path is the one place where Candid's
        existing trapping does the right thing for us** — once the
        sender's wire type-table is pruned (just `#int` / `#if_`
        for `Expr<Nat>`) and the receiver's expected variant is the
        pruned-for-`Bool` form (`#bool` / `#if_`), the existing
        "wire tag-hash not in expected arms → coercion failure"
        path traps cleanly. No new runtime check needed; we just
        had to feed both ends the pruned form. The original "negative
        test returns Some not null" mystery resolved to "the
        FromCandidE desugar normalised early and pruning never
        applied" — once pruning fired, Candid did the rest.

      - **The IR `BlockE` hook is the broadest tripwire** — applying
        σ at every BlockE works because of region-uniqueness, but
        any future IR pass that synthesises a BlockE and inherits the
        parent's region could trip it. The hook is necessary because
        record literals desugar to a BlockE-wrapped newObjE, and the
        σ has to refine the block's expected type for the result
        sub-check. M11 eliminates this whole pattern (σ as a field
        on the relevant IR nodes).

      - **`class MkRec(args) : Rec = {...}`** doesn't fall out for
        free even though it shares the ObjBlockE desugar path —
        the class declaration's body-vs-declared-return-type check
        at M0134 is a separate site that needs its own σ
        registration, and the synthesised class type is a fresh
        `Con(MkRec, [])` whose subtype check against `Rec` happens
        at call sites, not at the class declaration. **Shipped
        2026-05-16** (`test/run/gadt-class-existential.mo` passes
        all phases). The fix turned out to be smaller than the
        deferral note assumed — two narrow patches:

        1. **typing.ml M0134 site:** before the raw `sub t' t''`,
           if `t''` is `Con(c, ts)` with non-empty
           `lookup_typd_existentials c`, run
           `T.unify_existentials body' t' es` and check `t'` against
           the σ-refined body.
        2. **check_ir.ml CallPrim arm:** apply σ to the expected
           ambient type before the `t_ret <: t` sub-check (mirror
           of TupPrim/TagPrim). The call-site σ is registered by
           `gadt_check_typd_existentials` when typing
           `(MkRec 7 : Rec)` — the IR check just has to consult it.

        No cons-result coercion needed; the function's range stays
        `Con(MkRec, [])` and the σ-at-call-site makes the result
        sub-check go through.

        **Two non-obvious findings worth recording for next time:**

        - **`let p : T = e` is parser-rewritten** to `let p = (e : T)`
          via `normalize_let` in `parser.mly`. The annotation moves
          off the pattern and onto an `AnnotE` on the expression. So
          `let r : Rec = MkRec 7` never hits `AnnotP`'s sub-check;
          it hits `check_exp Rec (MkRec 7)`, which dispatches to
          `gadt_check_typd_existentials`. This is why typing
          accepted the cross-type sub without anything in the
          pattern path being aware of GADTs — and why a debug
          print on `T.sub` showed *no* `Con(MkRec)` calls at all.
          The σ that makes it work is registered at the call's
          `exp.at`, not the pattern's region.
        - **CallPrim is the third IR-check site repeating the
          σ-lookup pattern** (after TupPrim and TagPrim). Each
          construct that can carry a typd-existential refinement
          needs another copy of
          `match T.lookup_refinement_at exp.at with Some σ → subst
          σ (promote t) | None → t`. This is the cleanest concrete
          argument for **M11a** (σ on AST nodes): when σ lives on
          the IR node itself, every sub-check inherits it without a
          site-specific arm. Next σ-bearing construct added without
          M11a will be the fourth.

        **Gotchas mapped during a 2026-05-16 investigation pass (not
        yet shipped — recorded so a future attempt doesn't relearn
        them):**

        - **ClassD bypasses `check_exp`.** The typd-existentials
          check is wired into `check_exp` (typing.ml ~2864): when
          the expected type is `Con(c, ts)` with non-empty
          `lookup_typd_existentials c`, it dispatches to
          `gadt_check_typd_existentials`. `ClassD` in `infer_dec`
          (typing.ml ~5073) instead **infers** the body type
          `t'` via `infer_obj`, then runs a raw `sub t' t''` at
          the M0134 site. No witness inference fires. The fix
          is local: before the raw `sub`, if `t''` is a
          `Con(c, ts)` with typd existentials, run
          `T.unify_existentials body' t' es`, check
          `sub t' (subst σ body')`, and `register_refinement_at`
          σ at a region the desugar can find.
        - **The σ-key region question is real.** Record literals
          key σ off the ObjBlockE's surface region; lowering
          looks it up at the same region. The class desugar
          (desugar.ml ~1297) builds an internal `obj_block at s
          eo (Some self_id) dfs rng_typ` where `at` is the class
          declaration's region — the same as `dec.at` in typing.
          That's the natural key. But the enclosing FuncE
          synthesised at the same region might also try to read
          σ, which would be wrong. Mitigation: only ObjBlockE/ObjE
          lowering reads σ today, so collision is unlikely in
          practice — verify before declaring done.
        - **`rng_typ` is already promoted at desugar.** Line
          1314 computes `rng_typ = T.promote (T.open_ inst rng)`.
          For `class MkRec() : Rec = {...}` the promoted form is
          already the `Obj` body — the `Con(Rec, [])` wrapper is
          gone by the time the obj_block sees it. So σ must be
          applied to a *promoted* form. Mirror of the variant-arm
          gotcha ("σ on the promoted form, not the surface
          note").
        - **Class function signature stays at `Rec`.** The
          function's range type at typing is the user-declared
          `rng = Con(Rec, [])` (existential pack form), not the
          synthesised body. So callers receive `Rec` as expected.
          The coercion is needed only at the body-return site,
          not at the class identifier's type — there's nothing
          to do at the FuncE wrapper.
        - **No fresh `Con(MkRec, [])` for plain `class`.** The
          original deferral note above mentions "the synthesised
          class type is a fresh `Con(MkRec, [])` whose subtype
          check against `Rec` happens at call sites". For
          *actor* classes there's a `Cons.fresh id (T.Def([],
          class_typ))` (desugar.ml line 1544), but ordinary
          `class MkRec(...) : Rec = {...}` doesn't synthesise
          one — the function's range is already `Rec`. So
          "cons-result coercion" is a red herring for the
          non-actor case; the actual missing piece is just the
          M0134 witness-inference branch + the desugar σ-lookup.
        - **Minimal repro:** `type Rec = type X in { value : X;
          toText : X -> Text }; class MkRec(v : Nat) : Rec = {
          public let value = v; public let toText = func (n :
          Nat) : Text = debug_show n; };` currently fails with
          M0134 (`class body of type {toText : Nat -> Text;
          value : Nat} does not match expected type Rec =
          {toText : X -> Text; value : X}`).

- [x] **M9 — Soundness axioms**: state the GADT round-trip invariants
      explicitly, and verify by property-based tests. Three hand-written
      tests cover construct⇒dissect, dissect⇒construct, and round-trip
      through an existential arm:

      - `test/run/gadt-axiom-construct-dissect.mo` (axiom 1)
      - `test/run/gadt-axiom-dissect-construct.mo` (axiom 2)
      - `test/run/gadt-axiom-roundtrip.mo` (axiom 3 — `#eq` with `B`)

      Skolem-escape check (M9007) was dropped: structural subtyping
      already catches the leak for any non-`Any` return type, and an
      `Any`-typed leg is an explicit opt-out (the user erased the
      witness on purpose).

      **Implementation cost paid for axiom 3:** the IR type-checker was
      naïve about GADT refinement and existential packing. Three changes
      to keep IR happy across the post-typing transforms:
      - **`T.is_gadt_existential`**: relax `check_ir`'s `T.Abs` "free
        type constructor" check to admit registered existential cons.
      - **`gadt_refinement_at` side-table** keyed by source region:
        surface `check_case` registers σ for the arm; surface
        `gadt_check_existentials` registers σ at `TagE.at`. IR's
        `check_case` substitutes σ into both `t` (switch return) and
        `ve` (pattern bindings); IR's `TagPrim` branch substitutes σ
        into `T.promote t` before sub-check.
      - **`T.rewrite_gadt_side_tables`**: invoked at the end of
        `async.ml` and `erase_typ_field.ml` (both pasess clone cons)
        to migrate side-table keys/values onto the renamed cons.

      Side-tables are global mutable state — pragmatic short-term;
      removed in M11.

      **Axioms:**

      1. **Construct ⇒ Dissect.** If `v = #tag(args) : Expr<T>` typechecks,
         then `switch v { case (#tag(p)) ... }` binds `p` to the same
         values, with `p`'s type refined appropriately. The destructor
         recovers what the constructor packed.

      2. **Dissect ⇒ Construct (same arm).** Inside `case (#tag(p)) body`,
         the expression `#tag(p)` reconstructs a value of type `Expr<T>`
         that is observationally equal to the original scrutinee — same
         tag, same payload values, same refined type-parameter slot.

      3. **Round-trippable.** The composition
         `construct ∘ dissect ∘ construct = construct` on types — i.e.
         the type-system view of a value after one construct-dissect-
         construct cycle equals the type after the first construct.
         Existentials add a subtle wrinkle: the witness type at
         re-construct is the skolem from pattern-match, not the
         original concrete type, but both inhabit the same outer GADT
         instance (`Expr<T>` is the same), so externally indistinguishable.

      **Failure modes to detect via these axioms:**
      - Refinement applied at construction but not at pattern match (or
        vice-versa) → axiom 1 or 2 breaks.
      - Type erasure of existentials losing parametricity → axiom 3 leaks
        a witness identity.
      - Coverage pruning that admits values it shouldn't → axiom 1 or 2.

      **Verification strategy:**
      - Hand-written test cases per axiom × per arm shape (refinement,
        existential, parametric, mixed) live under `test/run/gadt-*.mo`.
      - Optional: a fuzzer that generates random GADT-shaped types and
        randomly constructs / destructures values, asserting round-trip
        equality.

- [~] **M11 — Remove side-tables + un-entangle the black holes**:
      two distinct sub-goals, related by both replacing the
      "schema-cons-as-shared-skolem" status quo. Soundness goal
      (un-entangling) shipped 2026-05-17 via path B; refactor
      goal (side-table elimination) partially shipped — four
      slices done, destructure-pat σ residual deferred.

      **M11a — refactor only (low risk).** Move σ from
      region-keyed side-tables onto IR AST nodes. Removes the
      tripwire class (the BlockE / TupPrim / TagPrim hooks become
      `match node with | { refinement; ... } -> ...`, no region
      matchmaking). Doesn't change soundness yet.

      **M11b — "Just Let Them Be Their Own Black Hole" (Overly
      Entangled Black Holes fix).** Each destructure site mints a
      *fresh* skolem cons per schema-existential, so two sibling
      destructures of the same type live in *different* black
      holes. Cross-feeding values across the holes
      (`let (x1, f1) = t1; let (x2, f2) = t2; f1 x2`) becomes a
      type error, where today it silently accepts and the runtime
      crashes when the witnesses turn out different.

      First cut of M11b shipped for M10 destructures only —
      see `test/fail/gadt-m11-cross-entangle.mo` for the now-rejected
      cross-feed and a region-keyed `gadt_fresh_skolems` cache plus
      σ application at the IR LetD check.

      **Two abandoned σ-extension attempts.** Both tried minting
      fresh skolems per arm-existential by extending
      `gadt_sigma_for_case`, mirror of the M10 destructure path.
      Both broke roundtrip tests via IR's `check_pat` /
      `check_case` impedance mismatches between σ-refined inner
      pat.notes and un-refined arm-payload extractions (see git
      log between 2026-05-16 and 2026-05-17 for the full pitfall
      traces). Path B (below) sidesteps by minting at TagP only.

      Status: M11a — partial migration shipped 2026-05-16, four
      slices (`gabor/gadt` `4787726f9` → `50e965ec9`):

      1. **Slice 1: σ field on note.** Added `note_sigma : T.typ
         T.ConEnv.t option` to surface `typ_note` and IR `Note.t`.
         Construction-side writers (gadt_check_existentials,
         gadt_check_typd_existentials) write to both side-table and
         note. Consumers (desugar ObjE/ObjBlockE, check_ir
         CallPrim/TupPrim/TagPrim) prefer note over side-table.
      2. **Slice 2: σ field on case node.** Added `mutable gadt_sigma`
         to both surface `case'` and IR `case'`. ~22 case'
         construction sites updated. Cons-rewriting passes
         (erase_typ_field, async) rewrite σ keys/values during
         migration to keep them coherent.
      3. **Slice 3: drop side-table writes for migrated paths.**
         Construction-side and case-arm σ writes removed from typing.
         Check_ir's BlockE arm migrated to the refine_target helper.
         Cons-rewriting passes preserve exp.note.gadt_sigma through
         t_exp clones.
      4. **Slice 4: drop side-table read fallbacks.** check_ir's
         refine_target, check_case, and desugar's ObjE/ObjBlockE all
         read note σ only — no side-table fallback.

      **Residual on side-table: destructure-pat σ only.**
      typing.ml's M11b LetD writer (~5605) and check_ir.ml's LetD
      reader (~1204) are the last two callsites. Migrating them
      needs either:
      - changing `pat = (pat', Type.typ) annotated_phrase` to carry
        a record note (~86 pat.note touchpoints across the tree), or
      - converting IR `LetD of pat * exp` to an inline-record
        variant or 3-tuple with σ (~35 LetD construction sites).
      Both are wider than a single slice; deferred.

      **What M11a fixed:** the "third σ-lookup site" concern
      (CallPrim, after TupPrim and TagPrim) is now a single
      `refine_target` helper at the top of `check_exp`. Cons-
      renaming passes (`async.ml`, `erase_typ_field.ml`) still
      need `rewrite_gadt_side_tables` for the destructure-σ
      residual *and* must rewrite σ on exp/case notes through
      `t_exp` / `t_case` — pattern for future cons-renamers.

      Still-global tables in `type.ml`: `gadt_arm_constraints`,
      `gadt_arm_existentials`, `gadt_existential_set`,
      `gadt_typd_existentials`, `gadt_fresh_skolems`,
      `gadt_refinement_at`. Only the last is σ-storage; the
      others are name-resolution tables read at typing time and
      not currently a soundness liability the way σ-on-region was.

      **Variant-arm un-entangling — SHIPPED 2026-05-17 via path B**
      (`4a24c1426`). Sidesteps the σ-extension impedance mismatch
      from the two reverted attempts by minting fresh skolems at
      `TagP` *only*, leaving the σ machinery untouched.

      Three patches, all consumer-side:

      1. `check_pat`'s `TagP` case in typing.ml: walk the
         extracted arm payload's shallow cons through
         `is_gadt_existential` to collect this arm's
         schema-existential cons; mint a fresh skolem per
         `(pat.at, schema_cons)` via `fresh_destructure_skolem`;
         substitute schema → fresh on the arm payload and recurse
         with the substituted form. Outer `pat.note` stays at the
         un-touched `t`, so IR's `check_case` `check_sub t_pat
         pat.note` keeps passing. Inner `pat.note`s and `ve`
         bindings carry fresh cons.
      2. `check_exp'`'s `TagE+Variant` fall-through: replicates
         `gadt_check_existentials`' unify bridge for the path
         where the body's expected `t` arrived as a Variant body
         (e.g., switch case body, after the dispatcher's
         normalize). Stashes σ on the TagE's note so IR's
         `refine_target` at TagPrim bridges fresh ← schema.
      3. IR's `check_pat_tag`: mirror of (2). When the scrutinee
         uses schema cons but pat.note carries fresh cons,
         `unify_existentials` derives σ and the refined arm
         payload is sub-checked against pat.note.

      Minimum churn: zero new typ rep, no `T.normalize` σ-awareness.
      σ-substitute lives only on the EXTRACTED arm payload, not on
      the schema-bound type, so the outer pat.note stays at schema
      and the IR view stays consistent — the impedance mismatch
      that killed the two σ-extension attempts doesn't arise.

      **Regression tests:**
      - `test/fail/gadt-cross-arm-mixing.mo` — original
        cross-feed (variant + arrow): `f1 x2` where x2 has
        different stamp than f1's param type. Rejected.
      - `test/fail/gadt-existential-leak-pair.mo` — single-arm
        reassembly into a concrete tuple: `(different, payload) :
        Pair<Nat>` rejected because fresh-X </: Nat.
      - `test/fail/gadt-existential-leak-pair-bare.mo` — mirror of
        the above but with a bare top-level existential alias
        (`type Pack = type X in (X, X)`). Different mechanism —
        alias-instantiation already gives fresh cons per use, no
        TagP mint needed — same outcome.
      - `test/fail/gadt-existential-leak-poly.mo` — known leak,
        currently passes (see "Skolem→Any subsumption" below).

      **Future refactor — path A (deferred).** First-class
      `(bind list, typ_with_Vars)` on variant arm fields. Same
      semantics as path B, cleaner internals (mirror of `Func`'s
      binders).

      **Scope audit (2026-05-17).** Wider than a single session;
      needs a dedicated branch with explicit checkpoints.

      **Rep choice (revised after slice-1 attempt 2026-05-17, rolled
      back at `WIP-gadt-pre-Path-A`):** add `binds : bind list` to
      `gen_field` itself, so there is one record type used uniformly
      by Obj fields, Variant arms, and (anticipating) record fields
      that may themselves grow existentials. Affects all 93
      field-record-literal sites tree-wide (each one trivial:
      `{lab; typ; src}` → `{lab; binds=[]; typ; src}`). No
      disambiguation gymnastics — single record type, single rule.

      **Why not the per-rep-shape variants tried first.** Slice-1
      attempted a fresh `variant_field` record alongside `field`
      with shared field names (`lab`, `typ`, `src`). OCaml's record
      field disambiguation requires explicit type annotations at
      every ambiguous site (warning 41 treated as error). The
      annotations cascade across `compare_*`, `align_*`, `inhabited_*`,
      `singleton_*`, `combine_*`, `rel_*`, plus codegen / IDL / docs
      / value consumers — a thousands-site noise floor *before*
      touching producer/consumer logic. Renaming the fields to
      `arm_lab`/`arm_binds`/`arm_typ`/`arm_src` works but pushes the
      same churn into every walker that reads them.

      **Anticipated future use — record-field existentials.** Once
      `gen_field` has `binds`, syntactic extensions like
      `type Counter = { state : type S in S; tick : S -> S; show : S -> Text }`
      (Mitchell-Plotkin object encoding) or phantom-tagged record
      fields fall out as natural uses of the slot — no further rep
      change. Variant arms and record fields share one mechanism.
      Doing the rep change once, with this end-state in mind,
      avoids a second rewrite later.

      **Alternative rep (deferred):** keeping the rep as
      `Variant of (gen_typ gen_field) list` (closure-in-typ
      thunks) was tried in the abandoned stash@{0} stage-2; rejected
      because OCaml's structural compare crashes on values
      containing closures.

      Plus ~10 type.ml walker sites need binder discipline
      mirrored from Func (compare/shift/subst/open/sub/
      free_cons/prune_gadt_variant). Templates already exist at
      type.ml:255, 459, 516, 570 (Func's bind handling).

      **The win**: eliminates `gadt_arm_existentials`,
      `gadt_arm_constraints`, the global `gadt_existential_set`,
      and the `is_gadt_existential` predicate. The escape check
      at check_ir.ml:210 and the shallow-cons filters at
      check_ir.ml:1161 / typing.ml:3220, 4198 currently
      exception-list registered existentials; with binds
      properly scoped by their arm, normal scope rules handle it
      and these special cases go away.

      **Out of scope for Path A**: `gadt_typd_existentials`
      (keyed by con, not arm — needs binds-on-Def-kind, separate
      refactor), `gadt_refinement_at` (destructure-pat σ,
      orthogonal), `gadt_fresh_skolems` (per-region memo, needs
      structural pat-node carrier).

      **Slice plan (revised for gen_field+binds rep).** No sub-branch
      needed — work proceeds on `gabor/gadt` with `WIP-gadt-pre-Path-A`
      as the rollback anchor. Each slice individually
      regression-green via `test-runner -bf gadt`.

      - **Slice 1 — Rep change.** `gen_field` grows `binds : bind list`.
        Walker helpers (`shift_field`, `subst_field`, `open_field`,
        `compare_field`) become binder-aware, mirroring Func's
        discipline. All 93 field-record-literal sites updated to
        include `binds = []` — find/replace, no semantic change.
        Build green, tests green, `binds` is `[]` everywhere.
      - **Slice 2 — Populate arm.binds at variant arm registration.**
        At `register_gadt_arm_existentials`'s call site in typing.ml,
        also write the existentials into the constructed
        `Variant fs`'s arm `binds` field. Side table
        `gadt_arm_existentials` still written in parallel. Consumers
        still read from side table. No behaviour change.
      - **Slice 3 — Migrate consumers to structural reads.**
        Replace `lookup_gadt_arm_existentials c lab` with structural
        reads of the matching arm's `binds`. Sites: typing.ml's
        Path B TagP fresh-mint (≈4197-4213), `gadt_check_existentials`
        (2910, 2939), check_ir.ml's escape check (210) and shallow-
        cons filters (1161). Side table still written but no longer
        read — confirms the structural data is correct.
      - **Slice 4 — Drop side-table writes + global predicate.**
        Remove `register_gadt_arm_existentials` and
        `register_gadt_arm` writes. Delete the two tables, the
        global `gadt_existential_set`, `is_gadt_existential`, and
        `migrate_existentials` in `rewrite_gadt_side_tables`.
        Escape check at check_ir.ml:210 no longer exception-lists
        — arm-bound cons appear inside their arm's scope via
        normal lookup. Refinement constraints
        (`gadt_arm_constraints`) follow the same path once a
        future slice migrates them onto the same `binds` slot or a
        sibling `constraints` field.
      - **Slice 5 — Refinement binders + 3-phase elaboration.**
        Extend `bind_sort` with `Refinement of typ` so refinement
        constraints (`type A = T`) ride alongside `Existential of con`
        on the arm's bind list.  Migrate the three refinement
        consumers (`gadt_check_refinements`, `gadt_sigma_for_case`,
        `prune_gadt_variant`) to read structurally via
        `arm_refinements` / `refinements_of_binds`.

        **3-phase kind elaboration protocol.**  Adding refinement
        binds at `check_typ_tag` time doesn't work: `rhs.note` is
        only reliably elaborated *after* check_typ runs, and the
        existing 2-pass scheme (Pre → Final, enforced by the
        `eq_kind` assert in `infer_id_typdecs`) breaks if pre- and
        final-pass produce different binds.  Resolution: extend the
        protocol to a third phase.

            Pre  →  Final  →  Refinement-Augment

        - Pre / Final: unchanged.  The placeholder→final transition
          via `set_kind`; subsequent re-elaborations confluence-
          checked via `eq_kind`.
        - Refinement-Augment: `Type.augment_arm_binds c lab binds`,
          called at the variant-decl registration site *after*
          `infer_id_typdecs` has returned.  Uses `Cons.unsafe_set_kind`
          to mutate the variant Def's matching arm in place.

        The third phase is justified by three invariants:

        1. **Gated**: only fires for variant Def-kinds.
        2. **Monotonic**: appends-only, prior existential binds
           survive at the head.
        3. **Post-confluence**: typing's 2-pass scheme has run to
           completion by the time augment runs; no later `set_kind`
           or `eq_kind` check observes the mutated cons-kind.

        Plus a deBruijn-scope correction: appending arm-local
        binders shifts the local scope index, so the payload's
        outer-Var references must be `shift 0 |new_binds| f.typ` to
        keep pointing at the right outer type-parameter.  (Without
        this shift, opening a `Con(Variant, [Bool])` would fail to
        substitute `Var(A, 0)` because the deBruijn index now sits
        below the inner arm-scope; the type leaks `Var` into walks
        like `serializable`, tripping the `Var | Pre -> assert false`
        invariant.)

        Side-tables `gadt_arm_existentials` and `gadt_arm_constraints`
        are now redundant (no readers); deletion deferred to a
        cleanup slice along with `is_gadt_existential` /
        `gadt_existential_set` once the escape-check exception at
        `check_ir.ml:210` is replaced by proper arm-binder scoping
        (requires `close_`-ing the arm payload — separate refactor).

        σ-derivability and `patch -R M11a` step deferred — still
        needs case'.gadt_sigma migration to derive on-demand from
        the now-fully-populated arm.binds.

      **Status update (2026-05-18) — Path A slices 1-5 shipped,
      slice 6 partial, cleanup of dead tables done.**

      - Slices 1-5 fully shipped on `gabor/gadt`.
      - `gadt_arm_existentials` and `gadt_arm_constraints` side
        tables retired (`bea205c19`); writers and readers are gone.
        Surviving registration is a single
        `List.iter T.register_existential` for the global
        `gadt_existential_set` (still needed for
        `is_gadt_existential` at check_ir.ml:210).
      - Slice 6 partial (`bc5e6a540`): variant-arm σ derives
        structurally via `derive_case_sigma` from
        `(t_pat_raw, arm label)`.  Path-compression
        (`normalize_stop_at is_gadt_con`) preserves outer Con on
        `exp.note.note_typ` and `pat.note` for GADT-bearing cons,
        so derivation sees the slot context.
      - **Top-level alias σ still uses `note_sigma` cache**.
        Blocker is `typing.ml:3026` —
        `gadt_check_typd_existentials` writes `T.normalize t` to
        `note_typ` precisely because desugar / codegen walk the
        note's structural body (Tup, Obj, Variant) directly, not
        through Con.  Preserving Con there needs either a wider
        desugar audit *or* the HKT-style Def-binder extension
        below.

      **HKT-style Def-binder extension (shipped 2026-05-18 at
      `3b66ef817`).** Top-level existential aliases now wear their
      binders structurally on the Def kind, mirroring slice 1's
      `gen_field+binds` one layer up:

          and kind =
            | Def of bind list * typ   (* binds can now contain
                                          [Existential c] entries *)
            | Abs of bind list * typ

      Bind list already supports `Existential of con` (slice 5).
      Def becomes
      `Def([{var="X"; sort=Existential X_cons; bound=Any}], body)`
      for `type Pack = type X in body`. Populated by
      `T.augment_def_binds c typd_binds` during the augment phase
      of TypD elaboration (mirror of `augment_arm_binds`).

      Consequences (now realized):

      - `is_gadt_con` is a pure structural test on the Def's
        binders (still falls back to the side-table set for
        Abs-kinded skolem cons).
      - `derive_typd_sigma` reads existentials from
        `Cons.kind c`'s binders directly — fully structural.
      - `reduce` / `check_typ_bounds` (both frontend and IR
        `check_ir.ml`) filter Existential binds before the arity
        comparison; user-visible callers still pass only the
        type-param actuals.
      - `gadt_typd_existentials` side table **retired
        (`b90cac552`).** Replaced by `T.typd_existentials c` which
        reads `Def.binds` via `existentials_of_binds`. The TypD
        register site now does `List.iter T.register_existential
        typd_es` to keep feeding `gadt_existential_set` in both
        passes (binds-on-Def population still lives in the augment
        block guarded by `not env.pre`). Six call sites in
        `typing.ml` migrated.

      With this slice landed, full σ-derivability for both variant
      arms and top-level aliases is achieved. `note_sigma` field
      becomes dead weight (next: drop fallback in `refine_target`).

      Sweep on commit (HKT slice): 23 `gadt-*` tests green;
      `test/run` quick (310 .mo) and `test/fail` quick (382 .mo)
      both clean.

      Sweep on commit (table retirement, `b90cac552`):
      `test-runner -b` 24/24 `gadt-*`; full `test/run` +
      `test/fail` 692/692.

      Rollback anchors:
      - `WIP-gadt-pre-hkt-defbinds` at `a3c34746d` — before HKT
        Def-binder slice
      - `WIP-gadt-path-a-variant-derived` at `bc5e6a540` — before
        either of HKT or table retirement

      **Slice 6+ (out of scope of initial Path A).** Anticipating
      record-field existentials, the same `binds` slot serves Obj
      fields too. No further rep change needed — only syntactic
      surface work and typing-side propagation to flow binders into
      field-level scopes the way they currently flow into arm
      scopes. Worth keeping in mind during Slice 1's walker design
      so the binder discipline is uniform (don't special-case the
      Variant arm; treat any field with non-empty binds the same).

      **M11a slice 2 reverted (`3d3b9137e`, 2026-05-18).** The
      `case'.gadt_sigma` field on both surface AST and IR became
      dead state after slice 6 retired the only consumer
      (variant-arm σ now derived on-demand via
      `derive_case_sigma` from the scrutinee type + arm label).
      `git revert -n a36f15507` cleanly removed the field + all
      ~22 threading sites; two conflicts (check_ir.ml,
      typing.ml) were resolved by keeping slice 6's derivation
      path and dropping both stale writers (no `register_refinement_at
      case.at` write — the case-path reader was gone since slice 4).

      **`gadt_refinement_at` retired (`06c33e29f`, 2026-05-18).**
      The M11b destructure-pat σ — previously stashed at pat.at in
      a region-keyed hashtable for check_ir to replay — is now
      recovered structurally on demand via
      `T.derive_destructure_sigma (typ exp) pat.note`, which
      unifies the schema's normalised body against the already-
      substituted pat.note.  `register_refinement_at` /
      `lookup_refinement_at` / `migrate_refinement_at` all gone.

      Collateral fix in `erase_typ_field.ml` and `async.ml`: the
      top-level `t_bind` didn't rename `Existential` / `Refinement`
      sorts when cloning Def kinds, so after Erase the Def's binds
      carried stale schema cons while the body had renamed cons.
      `T.typd_existentials` then returned the wrong list and σ
      derivation found nothing to unify.  Promoted the local
      sort-renaming logic from `t_field`'s shadowing `t_bind` to
      the top-level definition (deleted the shadow).

      **`Note.gadt_sigma` retired (`7b32b4455`, 2026-05-18).** The
      per-expression construction-side σ cache is gone — typing
      inlines σ into `note_typ` at the construction site
      (`gadt_check_typd_existentials` writes `note_typ = refined`
      where `refined = subst σ (open_ ts body)`), so desugar and
      check_ir see the already-substituted structural form.  Side
      effects of the retirement:
      - `Note.t` and surface `typ_note` lose their `gadt_sigma`
        field
      - `desugar.ml`'s ObjBlockE/ObjE σ-branches collapse to
        `obj_block / obj` on `note.Note.typ` directly
      - `check_ir.ml`'s `refine_target` simplifies — the non-TagPrim
        branch is just `t` (no σ recovery needed; substitution is
        already in `note_typ`); TagPrim still uses
        `derive_tag_sigma`
      - `async.ml` + `erase_typ_field.ml` lose the per-exp σ-rewrite
        blocks (no field to maintain)
      - All `note_sigma = None` initialisations in typing.ml,
        surface AST builders, etc. dropped

      Cost: typing's σ-inlining makes the LetD subtype check
      asymmetric — `pat.note` for a construction may still be the
      alias `Con` form (single VarP), while `typ exp` is the
      substituted structural form; for a destructure it's the other
      way around.  `check_ir.ml`'s LetD now derives σ in either
      direction via `derive_destructure_sigma`, unfolding whichever
      side carries the Con.

      **`gadt_existential_set` retired (`7939af651`, 2026-05-18).**
      Skolem cons identity now lives in their stamp value via a
      partition introduced in `mo_types/cons.ml`:
      - `>= 1`: counter-driven, freshly minted (regular cons)
      - `<= 0`: skolem reserve, today populated by
        `Cons.fresh_skolem`'s decrementing counter (`-1, -2, …`)

      `is_gadt_existential c := Cons.is_skolem c = fst c.stamp <= 0`
      — pure function of cons identity, no set lookup.  All call
      sites unchanged.  `register_existential`,
      `rewrite_gadt_side_tables`, and their callers in `async.ml` /
      `erase_typ_field.ml` deleted.  Cons clone preserves
      skolem-ness by routing to the appropriate counter based on
      the original's stamp sign.

      Side cosmetic: `M0137`'s "references type parameters …" cons
      list was implicitly hash-ordered (so its order shifted with
      the stamp re-numbering); now sorted alphabetically — also
      matches source-declaration order.  `test/fail/issue-2391.ok`
      updated.

      **The `<= 0` reserve future-proofs deterministic stamping.**
      Today's decrementing counter gives skolems unique stamps in
      the reserve; future deterministic encoding (e.g.
      `-Hashtbl.hash (region, schema_stamp)`) will live in the
      same range without touching `is_skolem`'s predicate.  Only
      the *minter* changes when that slice ships.  The
      `Fresh_skolem` effect handler still memoises per-site
      identity across typing passes; it goes when stamping becomes
      deterministic (see `type.ml`'s comment on
      `with_skolem_pool`).

      **Remaining GADT state: none in side tables.** All Path A
      side tables retired.  Surviving infrastructure:
      `Fresh_skolem` effect handler (per-typing-pass memo for
      skolem identity; deferred per above) and `is_gadt_existential`
      itself, now a one-liner over the stamp.

- [x] **`gadt_fresh_skolems` → OCaml-5 effect handler (2026-05-17).**
      First side-table retired via a scoped handler rather than a
      structural representation change. `fresh_destructure_skolem`
      now performs a `Fresh_skolem` effect; `with_skolem_pool`
      installs a per-handler memo table that dies with the handler
      frame. `Typing.infer_prog` wraps its body so each typing pass
      has its own pool — VSCode-extension / playground recompile
      flows can't pick up a previous pass's stamps.

      Knock-on: `migrate_existentials` in `rewrite_gadt_side_tables`
      now seeds the new `gadt_existential_set` by renaming every
      entry of the old set (rather than rebuilding only from
      `gadt_arm_existentials`), so fresh skolems minted by the pool
      survive cons-renaming passes (`async.ml`, `erase_typ_field.ml`)
      even though their source table no longer exists.

      **Future cleanup: deterministic stamping makes the handler
      redundant.** If skolem cons identity is encoded as a structural
      function of `(pat.at, schema_cons.stamp)` — e.g. the scope
      string `$skolem:<region>:<schema-stamp>` plus a tag — then
      `Cons.compare` will treat re-minted cons as equal *by
      construction*, no memo needed. Pros: deletes the handler + ~30
      lines net, no effect-tracking discipline at callsites. Cons:
      partitions the stamp space (counter-driven stamps vs
      deterministic skolem stamps) and overloads `Cons.t.stamp.scope`
      with a second meaning (currently filename / module-name only,
      would need either a sum-typed scope or a documented prefix
      convention). Path A's binders-on-arms refactor doesn't subsume
      this — fresh-cons stable-identity is orthogonal to where arm
      binders live.

      First-time use of OCaml-5 effects on the `gabor/gadt` branch
      (effects are used on `gabor/variant-switch` for the dispatch
      protocol). Sets a precedent other side-table refactors can
      follow.

- [ ] **Skolem→Any subsumption — open soundness gap (M13 candidate).**
      `test/fail/gadt-existential-leak-poly.mo` currently
      type-checks cleanly with no diagnostic. Two `#pack` arms in
      one switch produce distinct skolems `X1`, `X2`; a parametric
      `consume<T>(_ : Pair<(T, T)>) : ()` needs `(X1, X1) <:
      (T, T)` and `(X2, X2) <: (T, T)` for one `T`. Inference
      picks `T = Any` (universal upper bound of two abstract-bound
      cons) and accepts.

      **Why this is conceptually wrong.** Same family as the `.0`
      M9010 rule: `.0` rejection forbids naming a position inside
      a still-packed existential because that lets the witness
      escape; here, inference quietly coalesces two distinct
      *opened* witnesses through the universal upper bound. The
      witnesses leave their `case` scopes via a lossy subsumption
      that erases their identities. Strict Cardelli would say:
      a skolem may flow into a generic parameter, but inference
      must not promote it to `Any` to satisfy the bound when
      multiple distinct skolems would coalesce.

      **Strictly safe today** (parametricity — `consume<T>`
      cannot observe T). But invisible: no warning, no
      breadcrumb, exit 0.

      **Two staged mitigations:**
      1. **Warning.** Fire when inference's chosen solution for a
         type variable is `Any` *and* the only reason was that
         multiple distinct abstract-bound cons had to coalesce.
         New W code (e.g. M0197 "type variable inferred to Any to
         satisfy multiple distinct skolems"). Surfaces the leak
         in builds with warnings-as-errors; doesn't break
         existing code.
      2. **Hard error.** Mark inference variables that arose from
         generic-param solving; refuse the `Any` solution when
         multiple distinct skolems would merge. Caller is forced
         to either pass one skolem or explicitly annotate.

      (1) is the cheap first step. (2) is the principled fix and
      requires inference-side surgery.

- [ ] **VSCode plugin / playground hygiene — GADT side tables
      survive `Cons.session`.** `Cons.session` in
      `src/mo_types/cons.ml:32` saves/restores the cons-stamp
      counter via `Fun.protect`, giving the JS bundles
      (`js_check`, `moc_interpreter`, etc.) a clean slate per
      compilation. But it resets *only* the stamps — the six
      GADT side tables in `type.ml`
      (`gadt_arm_constraints`, `gadt_arm_existentials`,
      `gadt_typd_existentials`, `gadt_existential_set`,
      `gadt_fresh_skolems`, `gadt_refinement_at`) are global
      mutable state and persist across session boundaries.

      **Footgun shape.** After a session ends and stamps roll
      back, the next session's fresh cons can re-use a stamp
      from the prior session. `Cons.compare` is
      `(hash, stamp, scope, name)` with hash =
      `Hashtbl.hash (name, stamp)`; a new cons with the same
      name + stamp compares equal to a dead one. So the new
      session would silently retrieve the prior session's
      side-table entry under a `ConHash`/`ConLabHash`/`ConSet`
      lookup. Subtle in repeated-recompile flows
      (VSCode-extension save-on-edit, playground iterate).

      **Fix shape.** ~10 lines: extend `Cons.session` (or add a
      sibling `Type.session` that wraps it) so its `finally`
      block also clears the six GADT tables. Defensive,
      independent of representation choice.

      **Sequencing.** Deferred until after the Monday
      management buy-in. Path A (when shipped) eliminates three
      of the six tables structurally
      (`gadt_arm_constraints`, `gadt_arm_existentials`,
      `gadt_existential_set` — and the `is_gadt_existential`
      predicate falls out too), shrinking the session-reset
      surface. The other three
      (`gadt_typd_existentials`, `gadt_refinement_at`,
      `gadt_fresh_skolems`) survive Path A and still need
      session handling. Either order works; doing this *after*
      Path A means enumerating fewer tables.

- [x] **M12 — Candid / share-typing**: implement the Candid rules
      spelled out in [Interactions with existing Motoko → Candid](#candid).
      Three touch-points, one new function, no Candid spec changes:

      1. **`is_shared` extension**: existing share-typing walk gains a
         "registered existential cons appears free" check. Whole-type,
         not per-arm. Error message: **"black-hole type not shareable"**.
         Orthogonal to the wire format — runs *before* anything reaches
         Candid.
      2. **`T.monomorphise : typ -> typ`**: new function. Walks the
         type, replaces each `Con(c, ts)` whose body carries GADT
         refinements with its per-instantiation pruned form. Identity
         on non-GADT types. Memoised by `(c, ts)` so recursive variants
         terminate.
      3. **Call sites**: `monomorphise` is applied as a pre-massage at
         both wire boundaries:
         - Outgoing: `mo_to_idl` (and `to_candid`) before handing types
           to the IDL emitter.
         - Incoming: `from_candid`'s expected-type before Candid's
           wire-vs-expected subtype check.
         Symmetric application keeps the two ends' .did declarations
         in sync.

      Test matrix per the Candid section.

## Open knobs (deferred)

1. **Variance** of the type parameter when GADTs are present. Sidestepped by
   bounded polymorphism in most uses; revisit when a concrete codebase forces it.
2. **`switch type` integration timing** — implement static GADT first, then
   reuse Σ machinery for dynamic case.
3. **Uninhabited-type collapsing** (`Expr<Text>` → `None`) — skip; the user
   can't construct, so the distinction is moot.
4. **Variant-arm existential-pack as a first-class internal type
   (path A)** — record retroactively from the 2026-05-17 design pass.
   The principled internal representation of a variant arm with
   existentials is the existential pack `∃binds. payload(binds)`:
   variant arm fields carry `bind list × typ_with_Vars`, mirroring
   how `Func` carries `bind list × arg-typs × ret-typs`. Construction
   packs (witnesses inferred and *hidden* in the resulting `Variant`
   type — user always sees `#eq val : Expr<A>`, never an HKT-typed
   arm). Destruction opens with fresh skolems per match site, giving
   cross-arm soundness "for free" via stamp inequality.

   Why we *didn't* take path A in this session: the audit would touch
   ~20 walker sites in `type.ml` plus downstream consumers (similar
   in scope to the abandoned closure-in-typ attempt before it).
   Refactoring debt worth taking later, but not required for the
   semantics — see path B for the minimum-churn version we shipped.

   **Path B (shipped 2026-05-17, commit `4a24c1426`):** keep today's representation
   (`Variant of field list` with schema cons referenced in arm
   payload). Two targeted patches in `typing.ml`:
   - `check_pat`'s `TagP` case: before recursing on the inner
     pattern, mint a fresh skolem per schema cons present in the
     extracted arm payload (via `fresh_destructure_skolem pat.at`
     so the cons is stable per region), substitute schema → fresh
     in the arm payload, recurse with the substituted form. Outer
     `pat.note` stays at schema (so IR `check_case`'s
     `check_sub t_pat pat.note` still passes); inner pat.notes
     and `ve` bindings have fresh cons (so cross-arm `f1 x2` is
     rejected by stamp inequality).
   - `check_exp'`'s `TagE+Variant` fall-through (the dispatcher's
     other-than-TagE+Con path): walk `f.typ`'s cons, filter
     through `is_gadt_existential` to collect the arm's
     wildcards, then route through `unify_existentials` to bridge
     fresh-cons-typed actual against schema-cons arm payload.
     Mirror of `gadt_check_existentials` but reachable without
     the Con identity.

   Path B reuses existing machinery (`gadt_existential_set`,
   `fresh_destructure_skolem`, `unify_existentials`) and adds zero
   new typ constructors / field shapes. Future refactor to path A
   is pure form-over-function: same semantics, cleaner internals.

## Rejected designs (blind alleys, with reasons)

### Flavor A — Haskell arrow syntax

```motoko
#int : Nat -> Expr<Nat>
```

Collides with existing Motoko: `Nat -> Expr<Nat>` is already a valid function
type and would parse as "variant carrying a function payload." No way to
disambiguate without breaking change. **Rejected.**

### Flavor B (early) — `with` clause

```motoko
#int : Nat with A = Nat
```

Workable but the `with` keyword has other Motoko uses (`actor with`). Putting
the refinement *after* the payload is also harder to read: by the time you parse
`A`, you've already committed to the payload type. **Superseded by B3, then B4.**

### Flavor B3 — constraints before colon

```motoko
#int  type A = Nat       : Nat
#eq   type A = Bool, type B : ((B, B) -> Bool, Expr<B>, Expr<B>)
```

The form shipped in M1–M9. Constraints sit between the tag and `:`, payload
after. Works, but asymmetric with TypD's `= constraints in body` form
introduced for M10. **Superseded by B4** — pure surface change, no
typechecker impact, parser-only refactor.

### Flavor B4 (current) — introducer-first

```motoko
#int  : type A = Nat in Nat
#eq   : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>)
```

Unified with TypD: `: constraints in payload` (arm) and `= constraints
in body` (TypD top). One mental model. `in` is unambiguous separator
between bindings and payload — no COMMA / COLON dance. **Chosen.**

### Flavor C — refinement in switch only

```motoko
case (#int n) where A = Nat -> n
```

Requires the typechecker to *globally* know which variant tags inhabit which
specialised types, but the type declaration doesn't say. Would require
abandoning structural variant typing or doing whole-program analysis.
**Rejected; not viable without per-arm declaration.**

### Existential shorthand: RHS-unbound = fresh

```motoko
#if_ : type A = B in (Expr<Bool>, Expr<A>, Expr<A>)   // B unbound, treated as fresh
```

Implicit; if `B` happens to be in scope from an outer declaration, the meaning
silently flips. **Rejected** in favour of explicit `type B` declaration.

### Refinement clauses on plain `type` declarations

```motoko
type Foo<A> = { value : A } where A = Bool
```

Refinement is meaningful only at *constructors* (variant arms), where the type
discriminates inputs into different "tracks." Records have a single constructor
shape; refining `A` there is just defining a non-generic type — no GADT-ness.
**Rejected.**

### Type-safe pattern syntax `case (#int<A=Nat> n)`

Explicit witness in patterns. Verbose; refinement should be inferred from the
arm declaration. **Rejected** in favour of implicit refinement scoping.

## Review feedback (2026-05-16)

External review of the experiment PR raised four threads. Recording
both the questions and our current responses so the design intent is
captured in-tree (and so we don't relearn the answers on the next
pass).

### 1. Equality bounds vs upper bounds

**Reviewer:** "I sort of assumed GADTs require the type constructor
(`Exp<_>`) to be injective hence nominal but maybe that's not the
case. How do you encode the equality bounds — a new sort of bound,
or upper and (new) lower bound? I wonder if you actually need
equality in the constraint and can just use an upper bound. Worth
trying to minimize the changes."

**Response:** Equality is load-bearing for *substitution*, not just
pruning. The refinement `type N = Succ<M>` in a `#cons` arm is used
to replace `N` with `Succ<M>` inside the arm's payload — that's how
`head : List<A, Succ<M>> -> A` type-checks: the body sees `N`
unified to `Succ<M>` and the payload's `List<A, M>` becomes
well-typed in the surrounding context. An upper-bound `N <:
Succ<M>` is strictly weaker; `Non` satisfies it but doesn't give
us the substitution. Pruning (M5 arm-unreachability) could in
principle be done with sub alone, but the "type equality witness"
semantics that GADT pattern matching depends on requires the
equation.

Injectivity question: yes, `Con` identities are nominal in Motoko
(via `Cons.eq`), so `Exp<Nat>` and `Exp<Bool>` are distinct cons
instantiations and pattern-matching can discriminate. The schema
itself doesn't need a separate injectivity check — it falls out of
the existing Cons machinery.

### 2. Reuse type-binder syntax in patterns and variant declarations

**Reviewer:** "Why not just reuse the syntax for type binders in
patterns and variant declarations?"

**Response:** Already in place on the variant-declaration side —
`type M, type N = Succ<M>` is the type-binder grammar (a comma-
separated list of `type X (= T)?` clauses, then `in`, then the
body type), the same shape used in `type Tup = type X in ...`.
On the *pattern* side it's currently implicit: the case body's σ
comes from the arm's declared existentials, not from a binder in
the pattern. Explicit pattern binders (`case (#cons<M> (x, xs)) →
...`) would make skolem scope more legible and let users name the
existential. Tracked as an M11a companion question — implementation
cost is small, but worth pinning down whether the implicit form
should remain the default.

### 3. Coverage of impossible cases

**Reviewer:** "I guess the hard part will be coverage checking if
you want to allow omission of impossible cases. But I don't think
that's strictly necessary."

**Response:** Agreed — not part of the first cut. M5 already prunes
unreachable arms from *coverage* checking (a `#nil` arm with `N =
Zero` is dropped from coverage when the scrutinee type forces
`N = Succ<M>`), but the syntactic requirement that every arm be
listed is unchanged. Allowing omission would need the coverage
checker to consult arm-refinement against the scrutinee's
instantiation and certify exhaustiveness for the surviving arms.
Real "GADT exhaustiveness" feature, deferrable to a later pass.

### 4. Subtyping termination

**Reviewer:** "And ensuring subtyping terminates …"

**Response:** Known soft-spot — `rel_typ` in `type.ml` already
carries `(* TBR this may fail to terminate *)` markers on the
Def-recursion arms. The new refinement machinery substitutes σ
into the body before sub-check, but σ's image is closed under
existing type constructors, so we haven't expanded the termination
surface. Parameterised refinement RHS (`type N = Succ<M>` referring
to another existential) is the kind of construct that could trip
the existing depth limit on adversarial inputs; worth a separate
"termination audit" issue if anyone trips it in practice. Not
adding new fixed-point combinators or recursive unification rules
that change the asymptotic shape.

## Cross-references

- [shared-generics.md](shared-generics.md) — the dynamic-`switch type` design;
  shares the Σ-substitution machinery proposed here.
- Existing variant typing in `src/mo_types/type.ml` (`Variant` constructor) —
  the AST node that grows the `type_clause` field.
