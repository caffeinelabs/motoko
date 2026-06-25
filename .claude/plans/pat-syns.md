# Pattern synonyms via back-ticks

Design sketch for a small Motoko language extension: any in-scope `let`-bound
constant becomes available in pattern position by quoting its name in
back-ticks, with the desugarer expanding the back-tick site to the equivalent
literal pattern.

```motoko
let EQ_OP : Nat32 = 0x3d202020;        // '=   '
let GERMANY = #text "Germany";

switch opCode {
  case `EQ_OP` #eq;                    // ≡ case 0x3d202020 #eq
};

switch v {
  case `GERMANY` true;                 // ≡ case (#text "Germany") true
  case _         false;
};
```

---

## Motivation

The AE-decoder bench (`test/bench/object-spec.mo`) has many sites where one
*wants* a switch dispatch over named 4cc / opcode constants but is forced
into an `if-else if-else` chain because Motoko `switch` patterns don't
match against in-scope identifiers (they bind them as new variables instead).

Concrete examples from the file:

```motoko
// AE relo enum dispatch (in parseCmpdBody):
let op : Comparison =
  if (opCode == EQ_OP) #eq
  else if (opCode == LT_OP) #lt
  else if (opCode == GT_OP) #gt
  else if (opCode == LE_OP) #le
  else if (opCode == GE_OP) #ge
  else trap "AE: unsupported relo opcode";
```

```motoko
// Descriptor type-code dispatch (in parseDescBody):
if (typeCode == NULL or typeCode == EXMN) {
  if (length != 0) trap "AE: null/exmn desc with non-zero length";
  #root
} else if (typeCode == OBJ) {
  parseObjBody r
} else {
  trap "AE: unsupported ObjectSpec descriptor type"
}
```

With back-tick pattern synonyms these collapse into idiomatic switches:

```motoko
let op : Comparison = switch opCode {
  case `EQ_OP` #eq;
  case `LT_OP` #lt;
  case `GT_OP` #gt;
  case `LE_OP` #le;
  case `GE_OP` #ge;
  case _ trap "AE: unsupported relo opcode";
};
```

```motoko
switch typeCode {
  case (`NULL` or `EXMN`) {
    if (length != 0) trap "AE: null/exmn desc with non-zero length";
    #root
  };
  case `OBJ` parseObjBody r;
  case _ trap "AE: unsupported ObjectSpec descriptor type";
};
```

Workaround using literal hex (e.g. `case 0x3d202020`) is technically
available today but loses the symbolic name — and renaming/renumbering a
constant silently desyncs every match site.

---

## Syntax

- A new lexical token: a back-tick-delimited identifier ``` `name` ```.
- New pattern variant: `BackTickP` carrying the back-ticked identifier.
- Allowed wherever a literal pattern is allowed:
  - bare `case ` `X` `→ …`
  - inside variant patterns: `case (#compare {prop = `KNOWN_PROP`; …})`
  - inside or-patterns: `case (`A` or `B`)`

Back-tick is currently **completely unused** in Motoko (it isn't a token
in the lexer, doesn't conflict with any existing syntactic form, and is
reserved-comment-only outside the language proper). Free real estate.

---

## Semantics

`` `X` `` desugars to the **literal-pattern equivalent of X's definition**,
provided X is bound by a `let X = E` whose initializer `E` is a *constant
expression*.

A *constant expression* is recursively:
- A primitive literal: `Int`, `Nat`, `Nat32`, …, `Bool`, `Text`, `Char`, `Blob`.
- A variant constructor `#tag` with optional constant-expression payload.
- A record literal `{ f1 = e1; … }` where each `ei` is a constant expression.
- A tuple literal `(e1, …)` where each `ei` is a constant expression.
- Nested back-tick references to other constant-expression bindings.

Forbidden in constant expressions:
- function values (closures, `func …`)
- mutable variables (`var x`)
- side-effecting expressions (calls, awaits, traps in initialisers)
- arrays of mutable elements

The desugarer recursively expands `` `X` `` to a `LitP` / `VariantP` /
`RecordP` / `TupleP` AST node mirroring `E`. The IR pass and codegen never
see a `BackTickP` — it's entirely a front-end macro.

---

## Stacking with existing patterns

- **Variant constructor patterns** — `case (#and_(`COUNTRY_CMP`, _))` matches
  an `and_` whose first arg is exactly the value of `COUNTRY_CMP`.
- **Record patterns** — `case ({ class_ = `CLNT`; … })` matches the obj-spec
  whose class is exactly `CLNT`.
- **Or patterns** — `case (`A` or `B` or `C`)` matches any of three named
  constants (the desugaring expands to the alternation of three literal
  patterns).
- **Wildcards** — work unchanged inside record/tuple/variant literals
  expanded from the constant.

---

## Implementation sketch

### Parser

- Lex: add back-tick token (` ` `).
- Grammar: in pattern non-terminals, accept `` ` ID ` `` as an alternative
  to `litP`.
- AST: add `BackTickP of id` to the pattern variant in `mo_def/syntax.ml`.

### Desugarer (`src/lowering/desugar.ml`)

1. When traversing patterns, encounter a `BackTickP id`.
2. Resolve `id` to its binding (the typer has already done name resolution).
3. Walk the binding's initializer; check it's a constant expression
   (primitive lit | variant ctor of constant | record/tuple of constants).
4. Recursively map the initializer's AST onto the corresponding pattern
   AST (`LitP` for literals, `OptP`/`TagP` for variants, `ObjP` for records,
   `TupP` for tuples).
5. Splice into the pattern position.
6. If the binding isn't a constant expression, raise a typed diagnostic
   (`M02xx — back-ticked identifier is not a literal constant`).

The constness walk is similar to what already exists for stable-var
initializers, and to what some passes do to detect "compile-time
constants" — repurposable.

### Type checker

Probably no work: by the time the desugarer runs the AST has already been
type-checked; the back-tick site is type-checked under whatever scrutinee
type the surrounding `case` expects, and the resolved literal pattern
matches that type by construction (since X was already type-checked at
its binding site).

### IR / codegen / interpreter

No change. They never see `BackTickP`.

### `mo-doc` / pretty-printer

Add a single case for the new pattern variant.

---

## Comparison to other languages

| language | mechanism | declaration cost |
|--|--|--|
| Haskell (GHC) | `pattern P = …` synonyms | per-synonym declaration |
| Elixir | `^var` pin operator | none — any in-scope identifier |
| OCaml | `[%pat? …]` extension nodes / `match … with X when X = K` guards | varies |
| Erlang | uppercase-means-literal (no extension; constants must inline) | none, but limits naming |
| **proposed Motoko** | `` `X` `` back-tick pattern | none — any constant `let` |

Closest in spirit to Elixir's pin operator: zero declaration overhead,
sigil-based, transparent to readers. Chooses back-tick over caret (`^`)
because back-tick is unused in Motoko while `^` already pairs with `*`/
`+` in regex-flavoured contexts and reads less Markdown-friendly.

---

## Out of scope

- Synonyms for *guards* (Haskell-style `pattern Pos n <- n | n > 0`).
  Could be a v2 — would need new declaration syntax.
- Synonyms binding sub-values. The proposed `` `X` `` is purely a literal
  match; no fresh binding is introduced.
- Synonyms over types. This proposal is values-only.

---

## Open questions

1. Should `` `X` `` be permitted at non-toplevel binding sites (let-in,
   nested closures)? Default: yes, follow normal scoping.
2. What happens if `X` is shadowed at the back-tick site? Default: use the
   inner binding (lexical scope), constness of the chosen binding determines
   acceptance.
3. Recursive constants: `let A = 1; let B = A;` — is `` `B` `` legal?
   Default: yes, the desugarer follows the chain (terminates by typer's
   acyclicity guarantee).
4. Should the diagnostic for non-constant `` `X` `` suggest a workaround
   ("write the literal directly as `0x3d202020`")? Probably yes.
