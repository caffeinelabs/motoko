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

      **Deferred — projection on existential-bearing types**: bare
      `t.0` / `r.field` cannot soundly escape an existential. The two
      principled paths are (a) internally rewrite any such expression
      into a block-with-destructure (`{ let (_0, _1, _2) = t; … }`) or
      (b) reject bare projection and require the user to write the
      `let (...) = t` themselves. **We pick (b)** — Cardelli's `open`
      discipline. Simpler implementation, clearer error messages, no
      rewrite-engine debt.

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
        at call sites, not at the class declaration. Deferred —
        full support needs a cons-result coercion step or making
        the class function's return type the declared `Rec` rather
        than the synthesised body alias.

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

- [ ] **M11 — Remove side-tables**: the current `gadt_arm_constraints`,
      `gadt_arm_existentials`, `gadt_existential_set`, and
      `gadt_refinement_at` in `type.ml` are global mutable hash tables.
      They bridge surface-typing and IR-checking pragmatically, but:
      - Need explicit `rewrite_gadt_side_tables` hooks at every IR pass
        that clones cons (currently `async.ml`, `erase_typ_field.ml`).
        Any future pass that renames cons silently breaks GADT-typed code
        until someone notices and adds the hook.
      - Compilation units share the table — fine for a single `moc`
        invocation, dangerous if pipeline gains multi-unit reuse.
      - Surface-side registration is implicit; the AST itself doesn't
        carry the refinement, so transformations that rewrite AST
        (region-preserving but otherwise structural) cannot know about
        the σ they should drag along.

      Replacement directions:
      - Carry σ on the IR `case'` and `TagE`/`TagPrim` node directly
        (an extra `refinement : T.con_env` field). All sites that
        construct cases (4 places in `construct.ml`, 1 in `desugar.ml`)
        get a `~refinement` argument with `T.ConEnv.empty` default.
        IR passes that clone the AST already walk types — they walk
        refinements naturally.
      - Move arm-level GADT info from `type.ml` side-tables onto the
        `T.Con` definition body: extend `T.field` (variant arms) with
        the same `(var * typ) list` and `con list` per-arm. Then the
        cons-renaming passes handle them as part of `t_typ`/`t_field`.
      - Same applies to `T.is_gadt_existential` — replace the global
        `gadt_existential_set` check with "cons is reachable from a
        Variant arm's existential list," checkable at the IR site.

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

## Cross-references

- [shared-generics.md](shared-generics.md) — the dynamic-`switch type` design;
  shares the Σ-substitution machinery proposed here.
- Existing variant typing in `src/mo_types/type.ml` (`Variant` constructor) —
  the AST node that grows the `type_clause` field.
