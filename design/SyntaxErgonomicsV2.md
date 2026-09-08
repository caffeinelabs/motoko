# RFC: Syntax ergonomics for moc v2

Status: **Draft** — for discussion in #6231.

## Motivation

Motoko's syntax carries two recurring irritants:

1. **Token overhead** in common constructs: `switch (x) { case (?v) { ... }; case (null) { ... }; }`
   requires parentheses on the scrutinee, parentheses on patterns, and semicolons
   between cases — none of which disambiguate anything for the reader.
2. **Block-vs-record ambiguity**: `{` can open a block or a record, resolved today by
   the `(ob)`/`(bl)` grammar parameterization (each position admits only one meaning).
   The corners are user-visible: returning a record from a `case` needs `{ { ... } }`
   or `({ ... })`, and `opt ?? { x = 1 }` doesn't parse.

The dominant consumer of this syntax is an LLM. That cuts both ways: extra mandatory
tokens are extra places for generated code to go wrong, but *removed* syntax
invalidates model priors and turns into compile-retry cost in production. Hence the
overriding rollout principle: **relaxations are additive, removals are deprecations
first**, and errors must name their fix so a retry loop self-corrects.

The design approach follows Rust's handling of the identical struct-literal/block
ambiguity: keep parsing local, restrict the rarer meaning per position, provide an
explicit escape hatch (`(...)` for records, `do { }` for blocks), and invest in
diagnostics rather than grammar surgery.

## Proposals

### P1. Full expressions in scrutinee/condition position

`switch`, `if`, and `while` currently take `exp_nullary(ob)` — only atomic
expressions, so `switch (f(x))` and `if (a and b)` need parentheses. Change these
positions to `exp(bl)`:

```motoko
switch f(x) { ... }        // new
if a and b { ... }         // new
while i < n { ... }        // new
switch ({ x = 1 }) { ... } // record scrutinee now needs parens (Rust rule)
```

Under `bl`-mode a record literal directly in these positions no longer parses — the
`{` belongs to the construct's body. This is the one (tiny) breakage: a bare object
literal as scrutinee/condition, which is vanishingly rare and mechanically fixed by
parentheses (see P5 diagnostics).

Open question: extend to `for` (`for p in e block`, dropping the mandatory
`for (p in e)` parentheses)? Same mechanism; needs a conflict check around `in`.

Estimate: S–M (grammar + goldens; the `(bl)` machinery already exists).

### P2. Optional `;` between switch cases

`switch e { cs }` parses cases with `seplist(case, semicolon)`. Since every case
starts with the `case` keyword, the separator is redundant for the parser. Make it
optional (purely additive):

```motoko
switch n {
  case 0 { ... }
  case _ { ... }
}
```

Estimate: XS–S.

### P3. Un-parenthesized case patterns

`case p` takes `pat_nullary`, so `case (?v)` and `case (#tag x)` need parentheses.
Relax to `pat_un` where the pattern's extent is deterministic:

```motoko
case null { ... }
case ?v { ... }       // `?` takes pat_nullary, extent is deterministic
```

Open question: variant patterns with payloads are genuinely ambiguous without
parentheses — in `case #a x`, is `x` the payload or the case body? Options:
(a) payload binds greedily (then `case #a x` errors on missing body — confusing), or
(b) keep parentheses required exactly when a payload is present. Leaning (b).

Estimate: S, plus the ambiguity decision.

### P4. `??` cleanup (two parts)

**(a) Lexing/whitespace disambiguation.** `??` is unconditionally lexed as
`NULLCOALESCE`, and the parser carries backward-compat productions treating prefix
`?? e` / `?? T` as `?(?e)` / `?(?T)`. Replace with a lexer rule: `??` followed by
whitespace is the coalescing operator; `??x` (no space) is two option intros. Removes
the parser hack; breaking only for unspaced binary usage `a ??b`.

**(b) RHS mode flip.** The RHS is currently `exp_nest` (block-mode), so
`opt ?? { x = 1 }` fails while `opt ?? { ...decs }` is a block. Flip the RHS to
`exp(ob)`: bare record literals work, blocks use the existing escape hatch
`opt ?? do { ... }`. Breaking for bare-block RHS (operator shipped in 1.x recently,
usage is low; the P5 diagnostic points to `do { }`). Accepting both is not an option —
that would reintroduce content-based `{` disambiguation.

Estimate: S–M combined (lexer + grammar + goldens).

### P5. Diagnostics: make the ambiguity self-explaining

Three recoverable parse errors with concrete fix-its (each gets an `M0NNN` code,
appended to `error_codes.ml`, and should be `--ai-errors`-friendly):

1. *"a record literal is not allowed here; wrap it in `( ... )`"* — record in a
   `bl` position (case body, scrutinee, `??` RHS pre-P4b, ...). Kills the
   `{ { ... } }` confusion even where we change nothing else.
2. *"a block is not allowed here; use `do { ... }`"* — block in an `ob` position.
3. *"`query` is a reserved keyword and cannot be used as an identifier"* — replacing
   the generic `M0001 unexpected token` for all reserved words (from #6231's
   consider-list).

This is the highest-leverage item per unit effort: it dissolves most day-to-day pain
with zero breakage, and a precise fix-it means LLM retry loops converge in one step.

Estimate: M (menhir error reporting is fiddly; infrastructure exists in
`menhir_error_reporting`).

### P6. Reserve `implicit`

The lexer already emits an `IMPLICIT` token; the parser converts it back to an
identifier for compatibility. Remove that fallback so `implicit` is reserved (with
the P5.3 diagnostic), keeping the design space open for `implicit module` (#6084)
without a second breaking change later. Breaking for code using `implicit` as an
identifier.

Estimate: XS.

## Non-goals (explicitly out of scope for v2)

- Renaming `switch` → `match`, arrow-style case syntax, or any restructuring of what
  `switch` *is* — full-redesign cost (parser, formatter, lintoko, vscode, all
  existing code and model priors) for ergonomic gains P1–P3 mostly capture.
- Eliminating the block/record ambiguity structurally (a record head marker, or
  blocks-as-expressions requiring `do` everywhere). The only true fix, but it is a
  flag-day for every program; Rust demonstrates positional restriction + good errors
  is livable indefinitely. Revisit for v3 with a codemod.
- Semicolon/termination rules outside switch cases.

## Rollout

1. Land P5 diagnostics first (non-breaking, immediately useful, and P1/P4 reuse them).
2. P1–P3 are additive; ship with formatter canonicalization to the new forms so
   codebases and future training data converge.
3. P4 and P6 are breaking: v2 release notes + migration entries; where feasible emit
   a targeted error rather than a generic parse failure.
4. Coordinate grammar updates in the same release across formatter, lintoko, and the
   vscode extension.
