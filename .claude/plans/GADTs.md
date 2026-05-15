# GADTs for Motoko Variants (1-parameter PoC)

## Goal

Allow each arm of a parametric variant to **refine** the type parameter, so that
pattern-matching can recover the refined type in the case body. Target use-case:
a type-safe tagless evaluator over an `Expr<A>` AST, where each constructor's arm
constrains `A` to the type its value carries.

```motoko
type Expr<A> = {
  #int  type A = Nat        : Nat;
  #bool type A = Bool       : A;
  #add  type A = Nat        : (Expr<A>, Expr<A>);
  #if_                      : (Expr<Bool>, Expr<A>, Expr<A>);
  #eq   type A = Bool, B    : ((B, B) -> Bool, Expr<B>, Expr<B>);
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

### Variant arm grammar extension

A variant arm gains an optional **type clause** between the tag and the `:`:

```
arm  ::=  '#' Ident type-clause? ':' Payload
type-clause ::= 'type' type-decl (',' type-decl)*
type-decl   ::= Ident '=' Type      // refinement: outer X is set to Type
             | Ident                // existential: introduce fresh X
```

Examples:

```motoko
#int  type A = Nat       : Nat         // refinement: A ≡ Nat for this arm
#bool type A = Bool      : A           // refinement; payload uses the refined name
#if_                     : ...         // no clause = parametric
#eq   type A = Bool, B   : ...         // refinement + fresh existential, mixed
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

Same wire format as today's variants. Serialisation emits the variant tag +
payload; deserialisation does *extra* type-checking against the receiving type's
refinements and can trap if a tag's payload doesn't fit the expected refinement.
No protocol-level change.

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

- [ ] **M5 — Refinement-aware exhaustiveness**: `switch (e : Expr<Bool>)` can
      omit `#int`/`#add` cases because their `type A = Nat` is incompatible.
      Coverage analyzer needs to consult the GADT side-table and prune
      statically unreachable arms from "missing" lists.

- [ ] **M6 — Multi-parameter GADTs**: comma-separated `type A = T, type B = U`
      across multiple outer type-parameters. Parser already accepts it;
      typechecker side may need a few generalisations of the single-param
      assumptions in `gadt_sigma_for_case`.

- [ ] **M7 — Coverage analyzer fixup**: revisit `coverage_cases` to handle
      GADT-pruned-arm scenarios (a partial match is still exhaustive when
      missing arms are statically unreachable).

- [ ] **M8 — Diagnostics: errors & warnings on `type` clauses**:
      - **Unused constraint**: `type A = T` on an arm where `A` is the outer
        type-parameter but `A` doesn't appear in the payload (and no other
        constraint uses it) — refinement adds nothing → warn.
      - **Unused existential**: `type B` introduces B but B doesn't appear in
        the payload → either dead-code or a typo → warn.
      - **Circular constraint**: `type A = Foo<A>` or `type A = T<A>` where
        the RHS references A itself, producing an infinite type → error.
        Same family as the regular occurs check, but specific to GADT arms.
      - **Conflicting refinement**: `type A = Nat, type A = Bool` (same name,
        two RHS) — internally inconsistent → error.
      - **Shadowing warning**: `type A = T` where `A` shadows an outer name
        in a misleading way → optional warning, low priority.
      - **Existential collision**: `type B, type B` (two existentials with
        the same name) → either error or treat as one var with two binders
        (probably error, like duplicate VarP).

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
`A`, you've already committed to the payload type. **Superseded by B3.**

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
#if_ type A = B : (Expr<Bool>, Expr<A>, Expr<A>)   // B unbound, treated as fresh
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
