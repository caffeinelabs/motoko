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

### Decoder pipeline — wired end-to-end
- Frontend parses, type-checks, and stores `decode` on the FuncE
  through the `codecs.decoder` field.
- All ir_passes thread `codecs` through unchanged.
- `desugar.ml`'s `build_actor` populates `codecs.decoder = Some (exp
  dec_exp)` from the parenthetical via `find_decode_in_par` /
  `build_codecs`. (The previous `build_encoders` was generalised to
  return `(enc_opt, dec_opt) list`, factored on a shared
  `find_codec_in_par lab`.)
- Both backends (`compile_classical.ml`, `compile_enhanced.ml`)
  thread a `?(decoder=None)` thunk parameter — `(E.t -> VarEnv.t ->
  G.t) option` — through `FuncDec.{lit, closed,
  compile_const_message}`. The thunk is constructed at the FuncE
  call site (where `compile_exp_vanilla` is in scope) and wraps the
  decoder expression. Inside `compile_const_message`, branch at the
  argument-decoding step: `None` → `Serialization.deserialize`;
  `Some compile_dec` → `compile_dec env ae0 ; closure-call` on raw
  `IC.arg_data` instead.
- Result: a public actor method with `(with decode = …)` bypasses
  Candid on ingress; the user's closure receives the raw
  `msg_arg_data` bytes and produces a value of the method's
  argument type directly. The reply path is independent (Candid
  unless `encoder` is also set).

### Tests
Run-drun:
- `parenthetical-public.mo` — encoder, returns `()`, all phases.
- `parenthetical-decode.mo`  — full end-to-end: method ingress is
  `?Nat`, decoder is the flow `Blob -> ?Text -> ?Nat` composed via
  `decodeUtf8` and `Nat.fromText`. The `//CALL` payload is the raw
  three ASCII bytes `"123"` (`0x313233`) — *not* a Candid envelope.
  With the decoder active that blob deserialises as `?123` and the
  reply is Candid `Nat 123` (`0x4449444c00017d7b`). Without the
  decoder Candid would reject `0x313233` as malformed input — so the
  green test is end-to-end proof that ingress Candid is bypassed.

Fail (matched pairs, encoder ↔ decode):
- `parenthetical-{encoder,decode}-effect.mo`   — M0215 effect-free.
- `parenthetical-{encoder,decode}-mismatch.mo` — M0095 (finer than
  field-level M0214) on a wrong codec type.

---

## Pending work

### 1. Actor-level codec defaults
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

### 2. Type-table validation escape hatch
With a custom decoder, the framework no longer enforces Candid
type-table conformance on the wire. This is a deliberate escape
hatch — the user takes responsibility for the cross-canister contract.
Worth a Changelog/doc note when desugaring lands.

### 3. Exempt opted-out methods from the Candid interface

Methods that carry an `encoder` and/or `decode` no longer speak
Candid on the relevant edge, so advertising them in the Candid
surface is misleading — Candid-only clients (other canisters,
`dfx call`, `didc`) will Candid-encode arguments / Candid-decode
replies that the canister never produces. Two surfaces are
affected:

- **`__get_candid_interface_tmpl_v1`** — the embedded canister-section
  metadata that the IC fetches for Candid-aware tooling. The current
  generation walks the actor's public-method type list. We need to
  filter (or strip) the entries whose FuncE has a non-`None`
  `codecs.{encoder,decoder}` *for the relevant direction*: a
  decoder-only method still has a Candid-shaped reply, so its return
  type is fine in the dictionary — but its argument list isn't.
  Cleanest implementation is probably to suppress the whole method
  if either codec is set, with a follow-up if we want partial entries.
  Generation site is in `compile_classical.ml` /
  `compile_enhanced.ml` (search for `idl` / `__get_candid_interface`).

- **`moc --idl`** — the `.did` file generator (`mo_idl/mo_to_idl.ml`).
  Same logic: walk the type's public methods, drop or annotate the
  ones whose source FuncE has a codec set. The Candid AST has no
  way to express "non-Candid endpoint" today, so suppression is the
  right answer; clients see only the methods they can actually call.

For both, the codec presence is on `FuncE` in IR, but `mo_to_idl`
operates on the *type*, not the IR. We'd need to either thread the
codec presence into the public type (e.g. as a synthetic field on
`Type.func`, or a side-table mapping public-method-name → codec
presence held by the actor type) or filter at the IR level before
type extraction. The side-table approach keeps `Type.t` clean.

### 4. Reverse direction (alternative typing)
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
