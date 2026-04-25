# Non-Candid wire format for public actor methods

Design plan for replacing Candid (de)serialization on individual public
actor-method boundaries with user-supplied closures. Branch: `gabor/encoder`,
PR #5996.

---

## Motivation

Candid is the default wire format for the Internet Computer, but for
specialised methods (e.g. high-throughput ingestion, custom binary
protocols, formats expected by non-Motoko clients) canister authors want
to substitute their own encoder/decoder.

The strategic driver is **direct OpenAPI/Web2 interfacing**: a canister
that exposes JSON-over-HTTP endpoints to Web2 initiators (browsers,
mobile apps, server-side fetchers) cannot rely on Candid because those
callers have no Candid runtime. Custom encoder/decoder closures let a
canister speak whatever shape an OpenAPI spec dictates while leaving
the Motoko-internal call paths untouched. This document covers the
underlying mechanism; the OpenAPI integration that builds on top of it
is its own follow-on plan.

The escape hatch must be:

1. **Per-method**, not per-canister-wide — most methods stay on Candid.
2. **Declarative** — visible at the method's public signature, so
   reviewers/auditors don't have to chase an interceptor in the body.
3. **Type-checked** end-to-end — the user's closure must actually have
   the right shape.
4. **Composable with normal Motoko** — the rest of the method body and
   the actor framework must be unaffected.

The chosen surface syntax is the existing visibility-parenthetical
slot, which already carries other annotations (e.g. cycle limits):

```motoko
persistent actor {
  (with encoder = func (n : Nat) : Blob = ...;
        decode  = func (b : Blob) : Nat   = ...)
  public func get(_ : Nat) : async Nat = ...
}
```

---

## Current state (what has shipped)

### Surface syntax & parser
The visibility parenthetical is just an `ObjE`, so `decode = …` and
`encoder = …` parse without grammar changes. Field names are a closed
set, enforced by typing.

### Frontend typing — `check_vis_parenthetical` ([typing.ml:3857-…](../src/mo_frontend/typing.ml#L3857))

For a public `func name(args : A) : async R`:
- `encoder` is type-checked against `R -> Blob`.
- `decode`  is type-checked against `Blob -> A`.
- Direction is **method → codec** (the method's signature drives the
  expected closure types). An alternative — *codec types driving the
  method signature* — is recorded as a code comment for future
  exploration.
- Per-field check is bidirectional and lazy: only fields that the
  parenthetical actually contains are required to match; unknown fields
  warn (M0212); unknown effects error (M0215, naming the offending
  field). No field is mandatory — both `(with encoder = …)` alone and
  `(with decode = …)` alone are accepted.

### IR shape — `codecs` record on `FuncE` ([ir.ml:78-90](../src/ir_def/ir.ml#L78-L90))
```ocaml
| FuncE of … * exp * codecs

and codecs = { encoder : exp option; decoder : exp option }
```
A labeled record was chosen over a positional `exp option * exp option`
trailing pair so future codec-shaped additions (e.g. inbound-cycles
caps, retry policy, schema version pins) land additively without
churning every pattern match.

A `no_codecs` helper in `construct.ml` papers over the common
"no codec annotation" construction case.

### Encoder pipeline — already wired end-to-end
- `desugar.ml:build_actor` finds `encoder = …` in each public method's
  parenthetical and threads it onto the FuncE.
- All ir_passes (`rename`, `subst_var`, `freevars`, `await`, `async`,
  `const`, `tailcall`, `erase_typ_field`) carry `codecs` through.
- `async.ml`'s CPS transform reads `codecs.encoder` when synthesising
  the reply continuation, lifting it into `ICReplyPrim(ts, Some enc)`.
- `compile_classical.ml` / `compile_enhanced.ml` branch on
  `enc_opt` at `ICReplyPrim` codegen: `Some` → call the closure on the
  result, send the resulting `Blob` via `IC.reply_with_data`; `None` →
  emit `Serialization.serialize ts ; IC.reply_with_data`.
- `check_ir.ml` independently type-checks the encoder closure.

### AST interpreter — intentionally a no-op
`mo_interpreter/interpret.ml`'s `declare_dec_fields` doesn't visit the
`Public(_, Some par)` payload at all. The AST interpreter runs the
high-level `[run]` semantics where Candid serialization is not
modelled, so any wire-byte transform is moot. A reviewer-facing
comment captures this so future readers don't wonder why the
parenthetical seems to vanish at this level.

### Decoder pipeline — frontend only
- Frontend parses, type-checks, and stores `decode` on the FuncE
  through the `codecs.decoder` field.
- IR passes thread it through unchanged.
- `desugar.ml`'s `build_actor` does **not** install the decoder yet
  (`codecs.decoder` is always `None`).
- Codegen does **not** consult `codecs.decoder`.
- Result: a parenthetical `(with decode = …)` is fully type-checked
  but has zero runtime effect. The receiving Candid pipeline runs
  unchanged.

### Tests
Run-drun:
- `parenthetical-public.mo` — encoder, returns `()`, all phases.
- `parenthetical-decode.mo`  — decoder pipeline `Blob -> ?Nat` via
  `decodeUtf8 ∘ Nat.fromText`, method ingress `?Nat`, returns Candid
  `Nat 42`.

Fail (matched pairs, encoder ↔ decode):
- `parenthetical-{encoder,decode}-effect.mo`   — M0215 effect-free.
- `parenthetical-{encoder,decode}-mismatch.mo` — M0095 (finer than
  field-level M0214) on a wrong codec type.

---

## Pending work

### 1. Decoder desugaring (`desugar.ml`)
Mirror the encoder helpers:
```ocaml
and find_decode_in_par par : S.exp option = (* … *)
and build_decoders (df : S.dec_field) : S.exp option list = (* … *)
```
…and extend the `List.map2` zipper in `build_actor` (line 776) to a
`List.map3` over `(encoders, decoders, ds)`, populating
`codecs.decoder = Some (exp dec_exp)` next to the existing `encoder`
field.

### 2. Codegen hook (both backends)
Both `compile_classical.ml` and `compile_enhanced.ml` deserialize at
`compile_const_message`:
```ocaml
Serialization.deserialize env arg_tys ^^
G.concat_map (Var.set_val_vanilla_from_stack env ae1) (List.rev arg_names)
```
Branch on `codecs.decoder`. With `Some dec`:
```ocaml
let (set_dec, get_dec) = new_local env "decoder" in
compile_exp_vanilla env ae dec_exp ^^ set_dec ^^
get_dec ^^ Closure.prepare_closure_call env ^^
IC.arg_data env ^^                          (* raw blob *)
get_dec ^^
Closure.call_closure env 1 1 ^^             (* result : T.seq arg_tys *)
Tuple.unbox env (List.length arg_tys)       (* multi-arg case *)
```
Then the existing `set_val_vanilla_from_stack` walk consumes the N
values exactly as today.

The closure type is `Blob -> arg_typ` (strict, no framework-side
`?`-unwrap). User trapping is the user's responsibility — explicit in
the closure body or implicit by typing the method's argument as a
`?T` and propagating null further on. This keeps codec semantics
fully symmetric with the encoder side.

### 3. Actor-level codec defaults
Allow the parenthetical that today annotates an individual public
method to also be attached to the actor itself, supplying default
codecs for *all* public methods that don't override:

```motoko
(with encoder = defaultEnc;
      decode  = defaultDec)
persistent actor {
  public func quiet(x : T) : async R = body;        // inherits both
  (with encoder = customEnc)
  public func loud(x : T) : async R = body;         // overrides encoder, inherits decode
}
```

Implementation sketch:
- Frontend: extend the actor declaration's parenthetical handling to
  populate a `default_codecs : codecs` value computed before
  `build_actor` walks the dec_fields. Per-method codecs override
  field-by-field (`{ encoder = override.encoder ?? default.encoder;
  decoder = override.decoder ?? default.decoder }`).
- IR is untouched — the per-FuncE `codecs` record absorbs the merge
  result, so codegen reads exactly one value as today.
- Type-checking each per-method override is unchanged; the actor-level
  default needs separate typing because each public method may
  differ in argument/return shape — the actor-level closure has to
  be polymorphic enough to accept any `R` / produce any `A`. Likely
  signature: encoder is `forall R . R -> Blob`, decoder is
  `forall A . Blob -> A`. (Open question: do we want method-shape
  awareness, or treat the default as always-apply-or-skip?)
- Reviewer-facing rationale: lets canister authors declare a
  canister-wide custom protocol once instead of repeating it on every
  public method.

### 4. Type-table validation escape hatch
With a custom decoder, the framework no longer enforces Candid
type-table conformance on the wire. This is a deliberate escape
hatch — the user takes responsibility for the cross-canister contract.
Worth a Changelog/doc note when desugaring lands.

### 5. Reverse direction (alternative typing)
The current direction is method-signature → codec-type. We could
instead derive the method's *input* type from the decoder's output
type and the method's *output* type from the encoder's input type,
then check the method signature against those. Lets users pin codec
types and have the method signature fall in line. Documented in a
comment in `check_vis_parenthetical`; revisit if user feedback asks
for it.

---

## Non-goals

- Replacing Candid wholesale. The default remains Candid; `decode`/`encoder`
  is opt-in per method (or per actor, after item 3).
- Rich runtime introspection of the codec closure (e.g. composition
  primitives in the parenthetical syntax). Users compose with normal
  Motoko expressions (pipes, `do ?`, `Option.map`, etc.).
- Symmetric handling on the *caller* side. This plan addresses what
  the canister presents to the wire; calling a non-Candid method
  from Motoko code on the caller side is a separate problem.
