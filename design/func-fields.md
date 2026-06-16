# Design Analysis: `func`-Declaration Syntax for Record Fields

*Analysis by a Sonnet sub-agent against the moc source on `gabor/encoder`, 2026-06-12.*

## Proposal

Allow `func fieldName(args) : ret { body }` as an alternative to
`fieldName = func(args) : ret { body }` inside record literals (the verbose form
that pervades the Smurf protocol — `lookUp`, `toDesc`, `filter`, …). The key
twist: such `func`-field bodies should capture **sibling fields** of the same
record literal (mutual visibility, like `object`/`class` members), which record
literals do not provide today.

---

## Aspect 1 — Parsing: ambiguity and LR conflicts

**Current grammar.** `src/mo_frontend/parser.mly` disambiguates `{ … }` via a
parametric context trick: `ob` (line 617) routes to `exp_obj` (a record
literal); `bl` (line 616, matching only `DISALLOWED`) forces a `block`. There is
**no** production mixing record fields and declarations in one brace pair.

- `exp_obj` (line 628) = `seplist(exp_field, semicolon)`.
- `exp_field` (line 871) only accepts `var_opt id annot_opt [EQ exp]` — `FUNC`
  is never a valid lookahead there.
- `dec_field` (line 878) and `obj_body` (line 1063) are entirely separate
  non-terminals (the `object`/`class` member syntax).

**Conflict analysis.** Adding a `FUNC`-initiated alternative to `exp_field` is
**LR(1)-clean**: `FUNC` is currently a dead lookahead in that position, so no
shift/reduce conflict. If the parser desugars immediately to
`exp_field' { mut = Const; id = "f"; exp = FuncE("f", …) }`, the AST node
(`syntax.ml:251-252`, `{ mut : mut; id : id; exp : exp }`) needs **no change**.

---

## Aspect 2 — `with` interaction

The `with` extension syntax (`parser.mly:632`):
`LCURLY bases WITH seplist1(exp_field, …) RCURLY`. The field list is the **same**
`exp_field` non-terminal. So if `exp_field` gains a `FUNC` alternative,
`{ base with func lookUp(…) {…} }` parses cleanly with no new conflicts — `WITH`
already uniquely delineates the field list. The parenthesised `with` form
(line 620) inherits support automatically by sharing `exp_field` — and per
**Aspect 5** that inheritance is *wanted*, since `encoder`/`decoder`/`migration`
are function-valued parenthetical fields the sugar improves.

---

## Aspect 3 — Capture soundness

**Object/class sibling visibility (the precedent).** `infer_obj`
(`typing.ml:4117`) → `infer_block` (4140) runs a **gather pass**
(`gather_block_decs` → `infer_block_valdecs`, 4723-4732) that pre-populates all
names with `T.Pre` and adjoins them to `env` before checking any body — standard
mutual recursion. At desugaring, `build_obj` (`desugar.ml:1127`) wraps decls in
`blockE ds obj_e`; with a `self_id` it binds `self` via `selfRefE` (a `SelfRef`
IR primitive, `construct.ml:153`).

**Record literals have NO sibling visibility today.**
`infer_check_bases_fields` (`typing.ml:2654`) calls `infer_exp_field` (2647)
with the **caller's unmodified `env`** — no gather pass, no adjoin —
and `infer_exp_field` calls `infer_exp env exp` directly. Hence `{ a = 1; b = a }`
is a scope error **at the source type phase** — *not* because the lowering can't
express it (the `obj` desugaring at `desugar.ml:1130` already emits one
mutually-recursive `blockE`, no self-ref needed), but because the type phase
never adjoins the siblings. Removing that one omission is the whole of Phase 2.

**What recursive scoping would require — a typechecker-only change.** Extend
`infer_check_bases_fields` to (a) gather **all** field names (data *and*
function, **including fields inherited from the bases**) with their types, (b)
adjoin them to env, (c) check **function-valued** field bodies — both the
`func`-field sugar *and* plain lambda fields (`f = func(…){…}`) — against that
augmented env. **No desugaring change is needed.** The `obj` desugaring
(`desugar.ml:1130`) already lowers a record literal to a single
`blockE base_decs ++ field_decs ++ gap_decs; obj_e`, and the IR checker treats a
`BlockE` as one mutually-recursive scope — `gather_block_decs` then `adjoin env
scope` *before* checking any dec (`check_ir.ml:763-765`). So once the source
typechecker admits the names, runtime/codegen capture resolves for free, in any
order. The source typechecker simply **does not adjoin them today** — `{ a = 1;
b = a }` is a scope error not because the runtime can't, but because the type
phase never offers the siblings. Phase 2 is precisely teaching it to (this is
the outlook, not current behaviour).

**Three consequences fall out of full-surface capture** (per the review comment
on the Phase-2 verdict bullet, 2026-06-16):
- **Recursion is free.** A `func`-field name in the augmented env is visible in
  its own body and its siblings' bodies → self- and mutual recursion, exactly
  as `object`/`class` members enjoy.
- **All fields, not just functions.** The captured env should carry every
  sibling, so `func f() = dataField` works even when `dataField = expensive()`
  is an ordinary value field — the lambda reads the *already-computed* binding,
  it does not re-run the initialiser.
- **Base fields too — and bases are already evaluated once.** In
  `{ b1 and b2 with f1 = …; func f2() = b1sFieldA }`, `f2` must see fields
  contributed by the bases. The desugaring already binds each base to a fresh
  `base` var exactly once (`base_dec = letD base_var base_exp`,
  `desugar.ml:1135`) and projects inherited (“gap”) fields off it
  (`dotE (varE base_var) lab`, `desugar.ml:1154`). So a captured base field is a
  projection of the once-evaluated base — even an expensive computed `b1` is not
  re-run per capture. Again: nothing in the lowering changes; the type phase
  need only adjoin the inherited field names.

**Soundness pitfalls:**
- **`var`-field aliasing.** A `func`-field body capturing a sibling `var` field
  captures the block-local `VarD` cell, *not* a record-field projection. On
  record copy, the closure retains the original cell — diverges from object
  `self.f` semantics.
- **Circularity is confined to function bodies, so it is not a hazard.** Only
  function-valued bodies receive the augmented env; a value field `= expr` keeps
  the outer env and so cannot reference a sibling at all. An eager
  use-before-init cycle therefore cannot arise from full-surface capture. A
  *function* body may close over any sibling (incl. itself) freely, because it
  is evaluated lazily — at call time, after the whole block has initialised —
  not during construction. Hence forward and recursive references resolve
  correctly with no init-order constraint.
- Contrast with the `let self : T = { …; toDesc = func() = self.x }` workaround:
  that uses field projections through `self`; the proposed feature binds sibling
  block variables directly — equivalent for immutable fields, subtly different
  for `var`.

---

## Aspect 5 — Parentheticals

*Follow-up analysis by a second Sonnet sub-agent, 2026-06-12.* Refines Aspect 2:
the `with` form's auto-inheritance is real but **undesirable** for parentheticals.

**Parentheticals reuse `exp_field` verbatim.** The parenthetical production
(`parser.mly:619-621`) is:

```
%inline parenthetical:
  | LPAR base=exp_post(ob)? WITH fs=seplist(exp_field, semicolon) RPAR
    { Some (ObjE (Option.to_list base, fs) @? at $sloc) }
```

The field list is the **same** `seplist(exp_field, …)` as `exp_obj` (line 628).
So a `FUNC` alternative on `exp_field` lands in parentheticals automatically —
there is no mechanical way to admit it in record literals while excluding it
from `(with …)` *without* splitting `exp_field` into two non-terminals.

**Parsing — no conflict, but a structural constraint.** The two `(`-initiated
forms — `exp_plain` (`LPAR seplist(exp(ob), COMMA) RPAR`, line 638) and
`parenthetical` (line 620) — are distinct non-terminals invoked at distinct
sites (`exp_un`/`exp_nondec`/`vis`/`dec_nonvar`, never competing with a bare
parenthesised expr), so adding `FUNC` to `exp_field` introduces no shift/reduce
clash there. Note `(func foo(x){x})` *already* parses today — as a parenthesised
named-function **declaration** (`exp_nonvar → dec_nonvar → LetD(VarP foo, FuncE)`
via `func_pat`'s `id_opt`), nothing new. The one rule: `WITH` is mandatory to
separate `base?` from `fs`, so `FUNC`-fields must live only in the post-`WITH`
`fs` list, never the base — naturally satisfied since `exp_field` appears only
there. A `menhir --explain` run would confirm, but the grammar structure is clear.

**Semantics — separate the sugar from the capture.** The sub-agent's first cut
recommended *excluding* parentheticals (fork `exp_field`). That conflates two
things: it's a sound argument against **Phase 2 capture** here, but **not**
against the **Phase 1 sugar** — and the sugar is the part worth wanting.

Parentheticals are a **closed, compiler-known annotation vocabulary**, and three
of those fields are **function-valued**:
- `check_parenthetical` (`typing.ml:4213`): `cycles : Nat`, `timeout : Nat32` —
  scalar, irrelevant to `func`-fields.
- `check_vis_parenthetical` (`typing.ml:4232-4272`): `encoder : Blob→ret`,
  `decoder : Blob→arg`.
- `migration` (`check_migration_function`, `typing.ml:4363`; the `(with
  migration = fn)` form, line 4393): a non-generic local function.

For all three, the Phase-1 sugar is a genuine ergonomic win independent of
capture — `(with func migration(old) : New { … })` reads better than
`(with migration = func(old) : New { … })`, and likewise `(with func
encoder(b) { … }; func decoder(b) { … })`. *You don't have to use every facet of
a feature for it to pay off here.*

Where the agent is right: **Phase-2 capture has no use in a parenthetical.**
A parenthetical is an in-place annotation, not a heap record; its fields share
no scope with each other, so an inline `migration`/`encoder` body captures the
surrounding actor scope regardless. Capture is therefore simply *inert* here —
not harmful (it's gated to `func`-fields, so no `= e` field changes meaning;
M0215 at `typing.ml:4267` still enforces effect-freeness), just unmotivated.

**Verdict — keep `exp_field` unified; do NOT fork.** The sugar should reach
parentheticals precisely *because* of `encoder`/`decoder`/`migration`. Phase 1
desugars `func f(args):ret{body}` → `f = func(…){…}`, which is exactly what
`check_*_parenthetical` already accept — no typer change, no grammar fork.
Phase 2's gather pass need only run for record-literal / `with` field lists;
in a parenthetical it has nothing to gather and is a no-op. So the earlier
"introduce `full_exp_field`" recommendation is **withdrawn**: sharing the
non-terminal is the right call, and the only "exclusion" needed is that
Phase-2's adjoin step is naturally empty for parenthetical field lists.

---

## Aspect 4 — Desugaring & backwards compatibility

**Pure-sugar desugaring.** At parse time, `func f(args):ret{body}` in `exp_obj`
→ `exp_field' { mut = Const; id = "f"; exp = FuncE("f", …) }`. No AST change.

**Making ALL record fields mutually recursive is a SILENT breaking change.**
`{ a = 1; b = a }` errors today (safe to "fix"), but: if the outer env has
`let foo = 5` and a record writes `{ foo = "hello"; bar = foo }`, today
`bar = foo` refers to the outer `foo : Nat`; with whole-record sibling
visibility, the sibling `foo : Text` shadows it and `bar = foo` changes meaning.
Backwards-incompatible for currently-valid programs.

**Opt-in via `func`-syntax is backwards-safe.** Make sibling capture operative
**only** for `func`-declared field bodies; plain `field = expr` keeps the outer
env. No existing program changes meaning.

**Lambda-field caveat (gate choice).** Extending capture to plain lambda fields
(`f = func() = a`) as well — wanted, so the sugar and its desugaring behave
identically — *re-opens* a narrow shadowing hazard the `func`-only gate had
sidestepped: in `{ a : Text = …; f = func() = a }` with an outer `a : Nat`, the
lambda's `a` flips from the outer binding to the sibling. Today such a record is
either a scope error (no outer `a`) or silently means the outer `a`; capture
would change the latter. Two ways out: (i) accept it — the colliding-name case
is rare and the new meaning is the intuitive one; or (ii) gate Phase 2 behind an
experimental flag until the corpus is checked. Either way, **non-function**
`field = expr` bodies keep the outer env untouched.

---

## Verdict

- **Parsing:** Unambiguous. `FUNC` as a first token of `exp_field` is LR(1)-clean
  (currently a dead lookahead). `with` composes for free. No AST change if the
  parser desugars immediately.
- **`with`:** Clean and automatic.
- **Soundness:** Pure-sugar (rename only) is trivially sound. Sibling capture is
  *conditionally* sound — safe for immutable function fields (the intended use),
  but carries a `var`-field aliasing pitfall. Whole-record recursive scoping is a
  breaking change; opt-in via `func`-syntax is safe.

**Cleanest design — two phases:**
1. **Phase 1 (pure sugar):** `func f(args):ret{body}` in a record literal
   desugars to `f = func(…){…}` at parse time. No typer change, no sibling
   capture. Low risk. (Solves the user's stated dislike of the `= func` form.)
2. **Phase 2 (sibling capture, typechecker-only):** extend
   `infer_check_bases_fields` (`typing.ml:2654`) to gather **all** field names —
   data and function, **including those inherited from the bases** — and adjoin
   them when checking **function-valued** bodies (both the `func`-field sugar and
   plain lambda fields `f = func(…){…}`). **No desugaring change** (see Aspect 3):
   the `obj` lowering already emits one recursive `blockE` with bases evaluated
   once. Buys self-/mutual recursion and capture of (possibly expensive,
   once-evaluated) sibling and base fields for free. Medium risk (hybrid scoping
   rule + `var` semantics + the lambda back-compat caveat below).

**Open questions:** add a `self`-like binding (a desugaring change analogous to
`build_obj`'s `letE self e (varE self)` at `desugar.ml:1134`)? restrict `var`-field
capture inside `func`-fields? experimental-flag phase 1 initially?

**Overall risk:** low for phase 1; medium for phase 2.

---

## Running example — OSL accessor literals (`test/bench/object-spec.mo`)

The densest real-world user of the pattern this feature targets: the OSL "Smurf"
accessors. An `Accessor` (`test/bench/osl/OSL.mo`) is
`{ kind : {#property;#element}; form : {#indexed;#named;#test;#id}; fourcc : Text;
lookUp : (Smurf, LookupKey) -> Smurf }`, and the bench builds ~30 of them as
record literals — almost all sharing the same three-field structural prefix and a
function-valued `lookUp`. The root actor's `#indexed "ord "` accessor is the
worked case below.

### Baseline (today, on `gabor/encoder`)

```motoko
{
  kind   = #element;
  form   = #indexed;
  fourcc = "ord ";
  lookUp = func(parent : Smurf, key : LookupKey) : Smurf {
    let view = OSL.CollectionSmurf<Order>(orders, "ord ", parent, orderSmurf, orderLookup, func o = orderId o, null);
    switch (findAccessor(view, "ord ", #indexed)) {
      case (?acc) acc.lookUp(view, key);
      case null notFoundSmurf parent;
    };
  };
},
```

Three lines of `kind`/`form`/`fourcc` boilerplate; `lookUp = func(…) : Smurf {…}`
reads as an assignment, not a method. The 4cc `"ord "` is written **three** times
(field + twice in the body) and `#indexed` **twice** (field + body) — the same
constants restated across the literal.

### Stage 1 — Phase-1 sugar + a constructor helper + `with`

With the obvious helper
`func indexedElement(fourcc : Text) = { kind = #element; form = #indexed; fourcc }`
(siblings `namedElement`/`testElement`/`idElement`):

```motoko
{ indexedElement "ord " with
  func lookUp(parent : Smurf, key : LookupKey) : Smurf {
    let view = OSL.CollectionSmurf<Order>(orders, "ord ", parent, orderSmurf, orderLookup, func o = orderId o, null);
    switch (findAccessor(view, "ord ", #indexed)) {
      case (?acc) acc.lookUp(view, key);
      case null notFoundSmurf parent;
    };
  };
},
```

The structural prefix collapses to `indexedElement "ord "`; `func lookUp(…)` now
reads as a **method**. This is pure Phase-1 sugar over the `{ base with … }` form
— no capture — so the *body* still restates `"ord "` (×2) and `#indexed`. The
helper's narrow inferred variants (`{#element}`/`{#indexed}`) are subtypes of
`Accessor`'s wider ones, and the enclosing `accessors : [Accessor]` annotation
pins the result, so no return-type annotation is needed.

> Wins on the block-bodied element accessors (with a `switch`). It does **not**
> pay off on the compact `lookUp = func _ = ValueSmurf(…)` property one-liners —
> spelling out `func lookUp(_ : Smurf, _ : LookupKey) : Smurf = …` is *longer*
> than `func _ =`, which infers its parameter types from the expected `Accessor`.

### Stage 2 — Phase-2 capture (corrected)

The base fields `fourcc` and `form` are now in scope inside `lookUp`'s body:

```motoko
{ indexedElement "ord " with
  func lookUp(parent : Smurf, key : LookupKey) : Smurf {
    let view = OSL.CollectionSmurf<Order>(orders, fourcc, parent, orderSmurf, orderLookup, func o = orderId o, null);
    switch (findAccessor(view, fourcc, form)) {
      case (?acc) acc.lookUp(view, key);
      case null notFoundSmurf parent;
    };
  };
},
```

`indexedElement "ord "` becomes the **single source of truth**: `"ord "` and
`#indexed` each appear exactly once, and the body reads its own `fourcc`/`form`.
(`findAccessor`'s third argument is `form` — the `{#indexed;#named;#test;#id}`
discriminant — *not* `kind`, which is the `{#property;#element}` axis.)

This is exactly the **base-field capture** case the Phase-2 verdict bullet calls
for: `fourcc`/`form` are fields of the **base** (`indexedElement "ord "`, a
computed call evaluated once via the lowering's `base_dec`), reached through
`with` — not siblings written in the same literal. It is also the *safe* corner
of Phase 2: both captured fields are immutable, so the `var`-field aliasing
pitfall does not apply. A clean, non-synthetic motivator for landing Phase 2.
