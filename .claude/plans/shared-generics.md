# Shared generics with `T with type`

Plan document for filling Motoko's biggest public-interface hole: public
canister methods cannot today be parametrically defined over a type
variable. This document captures the design discussion that led to the
sketched proposal so the solution-space can be explored experimentally.

## The hole

Today every `public func` in a Motoko canister must be monomorphic. A
service that wants to expose "give me a `T`, I give you a `T` back" or
"serialise this `T`" has to pick a fixed `T` (often inlining a variant
that enumerates the supported cases) or fall back to `Blob`/`Text` with
a hand-rolled schema. This is exactly the abstraction failure
[#2096 — Shared generics](https://github.com/caffeinelabs/motoko/issues/2096)
diagnoses: any operation that is intensional (`==`, serialisation,
`debug_show`, dispatch on shape) has to be added as a new language
primitive rather than written as a generic library.

Related issues:
- [#2096 "Shared generics"](https://github.com/caffeinelabs/motoko/issues/2096) (open, 2020) — Rossberg's proposal: `<shared T>` type bindings with implicit RTT (runtime-type-representation) arguments passed by an evidence-translation lowering.
- [#5682 "AnyShared for Shared Generic Bounds"](https://github.com/caffeinelabs/motoko/issues/5682) (closed) — narrower predecessor proposing `<A <: AnyShared>` as a type-system primitive.
- [#3519 "Type checker doesn't reject parametrised shared functions"](https://github.com/caffeinelabs/motoko/issues/3519) (closed) — the inverse hole was eventually plugged, leaving the feature wholly unavailable.

## Surface design — `T with type` + `switch type`

The proposed surface form: a type parameter declared `T with type` brings
into scope a value-level *type descriptor* for `T`, scrutinisable by a
new `switch type` construct. Inside each branch the descriptor's match
implies a refinement of `T` and of every value whose type mentions `T`.

```motoko
public func foo<T with type>(arg : T) : async T {
  switch type T {
    case Int 42 + arg;             // arg : Int inside this branch
    case [X] switch type X {       // T is some array [X], with X bound here
      case Char ...;
      case _ ...;
    };
    case _ arg;                    // wildcard
  }
};
```

Inside `case Int`, the user writes plain `42 + arg` without any explicit
cast. The elaboration story is "shadowing" — conceptually the
type-checker inserts `let arg = cast<Int>arg` (and similar bindings for
every other in-scope value whose type mentions `T`) at the branch
boundary; the user never writes those casts. Implementation-side, the
cleanest realisation is **environment refinement**: open every type in
the current val_env, substitute `T := Int`, close. This is the standard
GADT-refinement spec, framed as a type-environment transformation.

### `switch type` over `type switch`

The chosen keyword form is `switch type T { … }` — `switch` is the verb,
`type` is the modifier, `T` is the target. Reasoning:

- **Parallel construction.** Motoko already has `switch e { case (#int n) … }`
  for values. `switch type T { case Int … }` keeps the outer shape; the
  `type` modifier signals what dimension is dispatched on. Readers familiar
  with value-switch acquire the new construct immediately.
- **Natural English.** "Switch on the type of T" reads verb-modifier-target;
  "type-switch T" treats "type switch" as a compound noun, slightly clunkier.
- **Composes through nesting.** Inner `switch type X` inside an outer
  `case [X] …` keeps visual rhythm.
- **Greppability.** `switch type` as a two-token sequence picks out every
  type-dispatch site with no false positives; `type switch` collides with
  `type` declarations and value-`switch` statements in line-scanning tools.
- **Parser.** `switch` keyword + `type` keyword + type expression is one
  rule; `type` is reserved so there's no identifier ambiguity.

### Type-pattern semantics inside `case`

Cases match against type *shapes*. Inside a pattern, the `type` keyword
is the **explicit binder marker** — the same role it plays at function
and Candid signatures (`T : type`, `(T : type)`). A name without `type`
is *always* a reference to an existing type; a name preceded by `type`
introduces a fresh type-variable scoped to the case body.

| pattern | meaning |
|---|---|
| `case Int` | match the primitive `Int` exactly |
| `case [Int]` | match arrays of `Int` (fully concrete) |
| `case [type X]` | match arrays of any element type; bind it as fresh `X` |
| `case [X]` (X already in scope) | match arrays of the existing `X` |
| `case [X]` (X not in scope) | **parse error** — `X` is neither bound here nor an existing type |
| `case ?[type X]` | option of array of any element type; binds `X` |
| `case (type X, type Y)` | tuple shape; both components fresh |
| `case { f : type X; g : opt X }` | record shape with field-binder, dependent on first |
| `case _` | wildcard |

Composes through nesting:

```motoko
switch type T {
  case [type X] switch type X {        // outer binds X; inner switches on it
    case Char "string of chars";
    case Int  "array of ints";
    case _    "array of something else";
  };
  case [Int] "specifically [Int]";     // fully concrete pattern; no binder
  case _ "not an array";
}
```

A fresh binder shadowing an in-scope name is explicit (`case [type X]`
re-binds `X`); the type-checker should warn on such shadowing, the same
way Motoko warns on value-level shadowing.

## The `Type` abstract primitive

A runtime `Type` value (the *type witness* / RTT) is exposed to user code
as an **abstract primitive type**, analogous to Motoko's existing
`Error` type. `Type` is to `switch type` as `Error` is to `catch`: same
"abstract primitive with a distinguished elimination form" pattern.

> **Internal mechanism — `prim switch (typCode stream)`.** The user-
> facing `switch type T { ... }` is a *skin* over an internal
> compiler primitive
> `prim type X<T>(stream : Candid) = prim switch (typCode stream) { case N : type T = … }` —
> a value-driven refinement form sketched and reasoned about in
> [GADTs.md → "Value-driven refinement: `prim switch`"](GADTs.md#value-driven-refinement-prim-switch-sketch-internal-only).
> The internal form reads the Candid type-table indices (LEB128-
> encoded `Int`s — `-3 = nat`, `-4 = int`, `-19 = vec`, non-negative
> = back-reference into the type table); the surface `switch type T`
> is its user-facing presentation, dispatching on a `Type` value
> rather than on a raw idx but using the same σ refinement machinery
> underneath (the existing GADT arm-refinement clauses `type T = …`).
> Closing this loop means the surface elimination form falls out as
> sugar over a primitive we'd ship anyway for runtime Candid
> decoders.
>
> **Progress (gabor/gadt branch, 2026-05-18, commit `1ce451fb4`):**
> The parser and AST for `prim type` + `prim switch` are landed,
> behind the existing `MOC_UNLOCK_PRIM=1` privilege gate.  The form
> currently parses end-to-end; typing emits M0229 ("not yet
> implemented") at elaboration.  Next slices replace M0229 with real
> semantics (arm selection, σ application, witness threading) — the
> M11/Path A refinement-clause machinery already in place is what
> feeds those.  AST shape settled:
> ```ocaml
> dec'.PrimTypD of typ_id * typ_bind list * (id * typ) list * typ
> typ'.PrimSwitchT of prim_discr * prim_switch_arm list
> prim_discr = TypCode of id        (* extensible — jump, etc. later *)
> prim_switch_arm' = { pat; refinement : typ_constraint }
> prim_idx_pat = IdxLitP of int | IdxWildP | IdxGuardP …
> ```
> The value-parameter slot (`(id * typ) list` on `PrimTypD`) is new
> — TypD has no value-parameter grammar; `prim type` introduces it
> behind the privilege gate without disturbing the user-facing
> `type` decl form.  Future slices will compose `switch type T`
> elaboration on top of this primitive when it has real semantics.

```
// Existing in Motoko:
type Error = ...      // abstract primitive; inhabitants from `throw` / arrive via `catch (e) { ... }`
                      // operations: Error.message, Error.code  (via Prim.errorMessage etc.)

// Proposed:
type Type = ...       // abstract primitive; inhabitants synthesised by the compiler
                      // distinguished elimination: `switch type T { ... }`
```

### User-visible operations

- **Construction (implicit only).** The compiler synthesises a `Type`
  value at every polymorphic call site from the static type argument,
  and at every Candid decode from the wire's type table. Users cannot
  `mk_type(...)` arbitrary RTTs.
- **`switch type T { ... }`** — the distinguished elimination form.
  Pattern-matches on the type's shape and refines the typing environment
  inside each case body (see Refinement semantics below).
- **`Prim.typeOf<T>() : Type`** — explicit value-level handle for a
  type-variable in scope, when the user wants to store / pass an RTT
  rather than just match on it.
- **`==`** — structural type equality. The runtime canonicalises (or
  hashes; see Implementation note below) so equality is cheap.
- **`debug_show : Type -> Text`** — diagnostic printer, formatted as
  the type's Candid IDL representation.

Additional operations (`Type.isSubtype`, `Type.codom`, `Type.fieldList`,
…) can be added as primitives once concrete use cases surface; the
abstract-type discipline keeps the surface evolvable without breaking
existing user code.

### Implementation note — internal representation

Internally a `Type` is `{ table : Blob; index : Int }`:
- `table` is the Candid type-table portion as a Blob, in wire format.
- `index` is the Candid type-table reference convention — negative
  values denote primitive Candid types (`-4 = int`, `-5 = nat`, `-2 =
  bool`, ...); non-negative values index the structured entries in
  `table`.

This representation is **not user-visible**. It's chosen because:

- Candid-arrived `Type`s and Motoko-synthesised `Type`s share the same
  bit layout — one runtime form, two construction paths.
- Pattern-matching by `switch type T { ... }` decomposes by reading the
  type-table entry at `index` and (for nested cases like `[type X]`)
  constructing fresh `Type` values that share the same `table` Blob and
  point at the inner indices. No copy; one Blob serves the whole
  type tree.
- Primitive `Type`s (Int, Nat, Text, Blob, Principal, ...) can be
  interned: `{ table = ⟨empty⟩; index = -<primcode> }` for each, shared
  globally; allocation drops to zero for the common case.
- μ-types fall out: the type table on the wire already encodes cycles
  via the index convention; sub-RTT navigation through a recursive
  shape just follows the indices around the loop.

Equality (`==` on `Type`) can be implemented by structural decode-and-
compare with cycle detection, by canonicalisation on construction, or
by a hash-then-structural-fallback. The PoC can start with structural
decode; production probably wants hashing.

Since the representation is hidden behind the abstract-type wall,
swapping among these strategies is non-breaking.

## Refinement semantics — open design questions

The mechanism is settled at the spec level; the open questions are about
its *scope*.

1. **Descriptor universe — closed or open?**  Is the set of type-pattern
   shapes (`Int`, `Nat`, `[X]`, `?X`, `(X, Y)`, `{ f : X; ... }`, ...)
   fixed at language design time (closed) or extensible per call site
   (open)? Closed gives exhaustiveness checks; open requires a wildcard
   and demands GADT-indexed singletons (`Type T`) under the hood.
2. **Subtyping × refinement.**  Motoko's structural subtyping makes
   `Int <: Int or Bool` (in variant types). If `T = {#int; #bool}` and
   the user writes `case {#int}`, does that refine `T` to *exactly*
   `{#int}` or to "the subset of `T` compatible with `{#int}`"? The
   latter is what one normally wants — "T happens to be some variant;
   bind its tag set" — but the rule that delivers that overlaps with
   row polymorphism.
3. **Exhaustiveness.**  Open universe forces a wildcard. A *bounded*
   variant — `<T : Int | Nat | Text>` — could enable exhaustive matching
   over the stated bound. Mirrors row polymorphism on variants.
4. **Refinement crossing closure boundaries.**  A closure created inside
   a branch and captured outwards: does its type carry the refined
   `arg : Int` (refinement leaks) or the original `arg : T` (refinement
   is local)? Probably the former for principledness.
5. **Recursive (μ) types crossing canister boundaries.**  Candid wire
   μ-types arrive as type-table cycles. The receiving canister either
   mints a fresh nominal type per call (leaks types over time) or
   unifies structurally with an existing local declaration (requires a
   structural-equivalence search at decode time). Both work; the choice
   is observable to users.

> *Resolved during design discussion.* "Binder vs reference inside case
> patterns" is settled by reusing the `type` keyword as the explicit
> binder marker — bare names are references, `type X` names are fresh
> binders. Symmetric with `T : type` at signature level.

## Lowering / implementation strategies

The four candidates considered were:

- **Implicit refinement (GADT-style)** — refine the typing environment
  per branch. Most ergonomic; biggest type-system surgery. **The
  endorsed direction.**
- **Explicit coercions** — pattern-match yields a descriptor; user
  writes `cast<Int>arg` per use. Type system stays cheap; ergonomic
  regression makes the feature unworth shipping.
- **CPS encoding** — reflect the type to `forall R. (...continuation) ->
  R`. Mechanism known; unergonomic to write.
- **Shadowing** — elaborator inserts `let arg = cast<Int>arg`. Reads as
  user-friendly spec; the *implementation* of "shadow every value whose
  type mentions T" turns out to be the same work as GADT refinement, so
  this is the same design as implicit-refinement at a different
  documentation register.

**Strategy.** Env-refinement is the spec. Open-substitute-close is the
operational form. A side-table of refinements consulted at type look-up
is the equivalent low-cost alternative for small envs.

The lowering at IR level threads an RTT argument through every
parametric call (Rossberg's #2096 mechanism). The `switch T` desugars to
a dispatch on the RTT.

**Cost.** Erasure dies for parametric code: every call carries an RTT;
structural equality on μ-typed RTT isn't O(1). Local monomorphic code
is unaffected.

## Phased bound expansion

Public/cross-actor calls force `T` to be Candid-serialisable, hence
`<T : shared>`. Intra-actor calls and library functions could relax to
`<T : stable>` (adds mutable refs but nothing big) or even `<T : Any>`
(fully erasure-dropping). Recommended staging:

- **Phase A** — `<T : shared>` only. Covers the actual public-interface
  hole. ABI-relevant. RTT-passing limited to shared types.
- **Phase B (optional)** — relax to `<T : stable>` for non-public uses.
  Adds mutable-ref support to library generics. Skip unless concretely
  requested; the marginal expressiveness over `shared` is "T may contain
  mutable refs", rarely what generic code wants.
- **Phase C (out of scope)** — full `<T : Any>` parametric refinement.
  RTT for closure types, async types, etc. Erasure across the board.

## Candid

The wire format already does the work; the change is purely at the IDL
grammar and type-checker level.

### Wire format — no change

Candid's wire format is already "type table, then value sequence". The
type table is per-call and carries the encoding of every type used in
the message. Existential and universal returns alike just use the
existing slot: T's encoding goes in the type table, T-typed values go
in the value sequence. The receiver decodes T first, then the value
typed against it. No new on-wire construct is needed.

### Surface (IDL)

Reject both the `reserved` placeholder (UX-broken — clients
reasonably read "accept anything" as "accept nothing of substance" and
ship calls with empty payloads, breaking the polymorphic contract) and
the prefix-Λ form `<T : shared> (T) -> (T)` (overcomplicated grammar).

Adopt instead **free type variables with per-method implicit
quantification, à la Haskell/Rust/Standard ML**. A type name that
isn't a primitive and isn't bound by a top-level `type T = …`
declaration is implicitly a fresh type variable, quantified at the
method scope:

```
service : {
  id     : (x : T) -> (T);                  // forall T. T -> T   (universal)
  swap   : (x : T, y : U) -> (U, T);        // forall T U. (T, U) -> (U, T)
  surprise : () -> (T);                      // exists T. () -> T   (existential)
  repeat : (x : T) -> (T, T);                // T independent from id's T
}
```

T scopes globally over the .did file as a *name*, but each method's
free vars are *its own* type variables. Want a second type variable in
the same method? Call it `U`, `T1`, etc. — uniqueness is the user's
job, as in Haskell.

### Quantifier position rule

Universal vs existential is determined by where the free variable
appears:

| variable appears | quantifier | semantics |
|---|---|---|
| in args (with or without results) | **universal** | caller picks T, callee respects |
| in results only | **existential** | callee picks T, returns witness + value |
| in args only | **forbidden** | uninhabitable (no value of caller-chosen T can be produced from nothing) |

This matches the natural reading and lets the IDL stay annotation-free.

### Bounds and forward declarations

Per-method bounds get awkward inline; the recommended shape is a
top-level forward declaration:

```
service : {
  type T : shared;                          // T is an abstract type variable; bound: shared
  id : (x : T) -> (T);
  swap : (x : T, y : U) -> (U, T);          // U also implicitly shared (default bound)
}
```

The forward declaration `type T;` (no body, just a name) doubles as a
**typo-footgun killer**: declaring `T` once at the top of the .did
makes `(x : Inventroy)` (misspelled) a parse error rather than a
silent implicit universal. The default bound for any introduced type
variable is `shared` (Candid's universe); writers can tighten with
`type T : <subtype>` if needed.

### Migration story — naturally one-sided

Because the wire format is unchanged, **existing canisters compiled
against the old IDL still decode correctly** as long as they don't try
to bind T statically. They see the type table entry for T but, having
no place to use it, just ignore it. They wouldn't be calling
polymorphic methods anyway since their tooling wouldn't have generated
the right wrappers.

So the rollout is:

1. Motoko compiler learns the new IDL semantics, emits parametric
   methods with implicit forall.
2. IC replica and core Candid type-table parsers: **no change**.
3. Agent libraries that want to call polymorphic methods generically
   add codegen for the new IDL syntax (TS/JS surface the result as a
   `{ type, value }` pair for existentials; Rust agent generates
   generic functions; etc.). Agents that don't can ignore the new
   syntax.

No coordinated cross-ecosystem flag day; the polymorphism-producing
side moves first, polymorphism-consuming side follows when ready.

### ABI evolution note

Changing an existing monomorphic `foo : (Int) -> Int` to parametric
`foo : (x : T) -> (T)` is an ABI break, regardless of IDL encoding.
Candid function subtyping is contravariant in args; the new signature
isn't a subtype of the old one. This is fundamental, not a syntax
issue — no graceful migration for existing methods.

### What this proposal explicitly does *not* do

- **Service-level abstract types** (a service exposes a type `T` that is
  the *same* across all its methods, Java-interface style). Out of
  scope; what's proposed here is per-method polymorphism. A separate
  mechanism would be needed later if service-level abstract types are
  desired.
- **Higher-rank or higher-kinded polymorphism** (`<T> ((T) -> T) -> (T)
  -> T`). Out of scope; the implicit-forall rule covers rank-1 only.

## Interaction with the worker/wrapper split (PR #6119)

Parametric public methods compose with the lattice-refined `Shared`
provenance ([PR #6119](https://github.com/caffeinelabs/motoko/pull/6119)
roadmap comments) without conflict: a `public func foo<T : shared>(...)`
is still a method on `self` with provenance `Self(srcloc, ...)`; its
worker `foo*` is parametric in the same `T`. Both wrapper and worker
take the RTT argument; the worker invocation forwards it. The
`switch T` body executes inside the worker, so RTT structural dispatch
benefits from the worker-call fast path on self-calls.

## Performance honesty

- RTT-passing is per-call overhead for parametric code only. Monomorphic
  code is unchanged.
- Structural equality on μ-typed RTT is not O(1); generic inner-loop
  code pays the cost.
- The `switch T` dispatch is a runtime tag check or equality compare on
  the RTT. For closed-universe descriptors, jump-table-fast. For
  open-universe with structural compare, slower.

Acceptable for an opt-in feature; would benchmark on the typical use
cases (serialisation library, generic container method) before
finalising.

## Sketch of a first PoC

1. Surface syntax — extend `parser.mly` to accept `<T with type>` in
   type parameter lists, and `switch type T { ... }` plus the new
   case-pattern forms (`case [type X]`, `case Int`, etc.).
2. Type-system extension — `T` declared with `with type` carries a
   "reflectable" flag in its `T.bind` record. Introduce the abstract
   primitive type `Type` (paralleling `Error`). The type checker tracks
   refinement equations per branch via a side-table (cheaper than
   env-rewriting for small envs).
3. Runtime representation of `Type` — the abstract primitive's hidden
   form is `{ table : Blob; index : Int }`, the Candid type-table
   reference convention. Motoko-synthesised and Candid-arrived RTTs use
   the same layout. Intern primitive `Type`s globally. Equality starts
   structural-decode-with-cycle-detection; can become hash-then-fallback
   later without breaking user code (representation is hidden).
4. Elaboration — `switch type T` desugars to a runtime dispatch over
   the type-table entry at the `Type`'s index, with the per-branch
   refinement equation added to the typing context for the case body.
5. Lowering — at IR level, every `<T with type>` (bound `: shared`)
   parameter becomes an ordinary parameter of the abstract `Type`,
   passed alongside the actual user-supplied value. Calls insert the
   `Type` synthesised from the static `T` at the call site
   (`Prim.typeOf<T>()` at the IR level).
6. Candid integration — emit free-type-variable IDL using the
   implicit-forall convention from §Candid. Wire format unchanged; no
   spec bump required for first roll-out. Agents on the consuming side
   pick this up when they add codegen for free type variables.
7. Tests — a `serialise<T with type>(x : T) : Blob` library function
   exercised from a driver actor; a `roundtrip<T with type>(x : T) :
   async T` public method exercised from another actor; one
   `switch type T` with `case [type X]` nesting; one Maybe-self / Phase-2
   call to validate composition with PR #6119.

## Red team — projected critiques from Motoko's type-theory voices

This is the strongest dual of "what's been agreed". Projecting what
Andreas Rossberg and Claudio Russo might emphasise on first reading,
based on prior issue threads (#2096, #5682, #707, #705, et al.), their
public design conventions, and standard type-theory expectations.

### Andreas would probably push back on three things

1. **Implicit-forall is un-Motoko-like.** The language inherited ML's
   explicit polymorphism conventions: `<T>` binders, named scope
   variables, no Haskell-style implicit quantification. Switching to
   per-method implicit quantification at the IDL level needs
   justification beyond keystroke-saving — and the typo footgun he
   himself flagged in #2096-style discussions leans him toward keeping
   binders explicit. He'd likely prefer either the original `<shared T>`
   prefix form or a top-level `type T : shared;` declaration with
   explicit references thereafter; bare-name implicit binding rejected.

2. **Position-driven universal-vs-existential is too clever.** The rule
   "freevar appears in args → universal; in results only → existential"
   reads natural but conflates two distinct quantifier shapes via a
   heuristic. His own #2096 proposal stayed *strictly* universal; adding
   existentials at all is a bigger semantic commitment than the
   four-line position rule suggests. He'd probably want explicit
   `forall T.` and `exists T.` keywords, or rule out existentials in
   this proposal entirely and revisit them separately.

3. **RTT formalisation needs a semantic story.** Allergy to
   underspecified semantics (from WebAssembly spec work). The plan's
   "structural equality on μ-typed RTT" hand-wave would, with him,
   become a hard requirement to actually define: when are two RTTs
   equal? Up to α-conversion of bound vars in μs? Up to subtyping?
   What's the decision algorithm? Concrete answers required before
   sign-off.

### Claudio would home in on different points

1. **Refinement-as-env-substitution = GADTs by another name.** F-omega
   background and OCaml-style GADT translation experience. He'd point
   out (gently) that "open-substitute-close" is the GADT refinement
   semantics with the GADT-isms file-and-replaced away. Fine, but it
   means the type-system formalisation work is full GADT — equality
   witnesses, coherence proofs, refinement-soundness under recursion.
   Not lighter just because we're not calling it GADTs.

2. **Interaction with structural subtyping is the actually-hard part.**
   Motoko's variants and records are width/depth-subtyped. Refinement
   under subtyping (when does `case Int` refine `T` to *exactly* `Int`
   vs to any subtype of `T` compatible with `Int`?) doesn't have a
   one-line answer; the existing literature on row polymorphism + GADT
   refinement is thin. A concrete semantic rule with examples is
   required before committing to the surface.

3. **`switch type` vs reflection-as-a-library.** He'd likely raise the
   option of doing this *without* new syntax: expose RTT as an ordinary
   Motoko value (`Type.rtt<T>() : RTT`) and let users write
   `switch (Type.rtt<T>()) { … }` with regular pattern matching. The
   new syntax buys a refinement-on-success rule that the library form
   doesn't have, but you trade language surface for ergonomics. He'd
   want that trade made explicit, not implicit in the surface design.

### Where both would likely nod

- Candid wire format being unchanged is the strongest single piece —
  the rollout doesn't demand a coordinated flag day.
- The bound being `shared` for public, the staging Phase A → B →
  (skipped) C, and the explicit out-of-scope list (no service-level
  abstract types, no higher-rank) all read as a properly-scoped first
  pass.
- The acknowledgement that adding parametricity to an existing
  monomorphic method is a fundamental ABI break (not fixable by syntax)
  — honesty appreciated.

### The single thing both would want before sign-off

A small implementation that demonstrates the type-checker's refinement
semantics on a non-trivial example (e.g. a generic
`serialise<T>(x : T) : Blob` with a couple of `switch type T` cases)
end-to-end through Candid. Specifications convince them; running PoCs
convince them harder.

**Net read.** The design has the right architecture but is *pre-paper*,
not *pre-spec*. Both would help shape the formal story; both would
expect a working sketch to focus the conversation. The §Sketch of a
first PoC items are exactly the slice that would warrant booking time
with either of them.

## Open follow-ups before experimentation

- Decide closed vs open RTT universe (point 1 above).
- Decide subtyping × refinement rule (point 2).
- Settle on RTT data structure (Motoko variant vs primitive RTT type
  with operations).
- Coordinate with the Candid spec discussion on Λ.
- Validate against existing parametric library APIs (Map, Buffer, etc.)
  — does the proposal allow them to be polymorphic in their element
  type *across* public method boundaries?

---

This plan is exploratory. The PoC items in §"Sketch" are the smallest
slice that lets us feel the design end-to-end; expect to revisit the
descriptor universe and refinement rules after the first round.
