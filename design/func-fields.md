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
and `infer_exp_field` calls `infer_exp env exp` directly. The desugaring `obj`
(`desugar.ml:1165`) emits a sequential `blockE decs obj_e` with no self-ref.
Hence `{ a = 1; b = a }` is a scope error.

**What recursive scoping would require.** Extend `infer_check_bases_fields` to
(a) gather the `func`-declared field names, (b) adjoin them to env before
checking `func`-field bodies. The desugaring already puts all fields in one
`blockE` scope, so runtime capture resolves correctly.

**Soundness pitfalls:**
- **`var`-field aliasing.** A `func`-field body capturing a sibling `var` field
  captures the block-local `VarD` cell, *not* a record-field projection. On
  record copy, the closure retains the original cell — diverges from object
  `self.f` semantics.
- Forward references to non-function fields are safe (closures read values at
  call time, after block init).
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
2. **Phase 2 (sibling capture, gated):** extend `infer_check_bases_fields`
   (`typing.ml:2654`) to gather `func`-declared field names and adjoin them for
   `func`-field bodies only. Medium risk (hybrid scoping rule + `var` semantics).

**Open questions:** add a `self`-like binding (a desugaring change analogous to
`build_obj`'s `letE self e (varE self)` at `desugar.ml:1134`)? restrict `var`-field
capture inside `func`-fields? experimental-flag phase 1 initially?

**Overall risk:** low for phase 1; medium for phase 2.
