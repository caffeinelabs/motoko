# Deprivileged `async*` — threading a "send-or-shutup" capability through nested calls

Survey + design recommendation for tracking whether a transitive
`async*` call chain ever performs an IC send, so that calling such a
chain from a context that disallows sends (`query`, certain composite
query positions) fails at compile time rather than trapping at
runtime.

Captures the day's investigation; intended as load-bearing input for
the post-buy-in cleanup arc, where this lands as the refinement of
the "`await*` learns queries" item in
[`.claude/plans/ic-mator.md`](./ic-mator.md) and
[`.claude/plans/query.md`](./query.md).

## Problem

The bench's `Smurf` protocol is built around higher-order dispatch:
each `Accessor` carries a `lookUp : (Smurf, LookupKey) -> Smurf`
field, and `findAccessor` returns an `Accessor` whose `lookUp` is
selected at runtime.  Today nothing prevents an `Accessor.lookUp`
implementation from internally `await`ing a shared call (cross-
canister send).  Today nothing prevents a `query` method from
calling, via `await*`, an `async*` chain that resolves to such an
`Accessor.lookUp`.  The IC will reject the outbound message at
runtime and the canister traps.

The desired property:

> A `query` (or composite-query restricted) function body must not be
> able, even transitively through `async*` and stored function
> values, to perform an IC send.  Violations must surface at
> compile time, pointing at the function literal that introduced the
> send.

## What's already in moc

[src/mo_types/async_cap.ml](../../src/mo_types/async_cap.ml) defines
a **context capability** — what the current scope is allowed to do:

```ocaml
type async_cap =
  | QueryCap of T.con           (* inside a `query` body — no sends *)
  | ErrorCap                    (* inside async body of `query`/catch *)
  | AsyncCap of T.con           (* async expr, sends allowed *)
  | AwaitCap of T.con           (* async + send + try/catch *)
  | SystemCap of T.con          (* protected system funcs *)
  | CompositeCap of T.con       (* composite query — restricted sends *)
  | CompositeAwaitCap of T.con
  | NullCap
```

[src/mo_frontend/typing.ml](../../src/mo_frontend/typing.ml) enforces
this at every `await` / send site — M0037, M0039, and the "send
capability required" diagnostic family.  **Direct** sends inside a
`query` body are already caught.

What's missing: function **types** carry no "may-send" effect.  When
a `query` body calls `await* helper()` for some
`helper : () -> async* T`, the type checker has only the type to go
on.  If `helper` is defined in another module or even just lifted
out of view, the body's send status is opaque.  Today's capability
system tracks the *context* in which code lives, not the *effect*
of values it calls.

## Survey of approaches

Three shapes were considered.  Only one survives contact with the
bench.

### Approach A — monomorphic effect bit on function types

```ocaml
type send_effect = Pure | MaySend
| Async of async_sort * scope * typ * send_effect
```

Inference: function body containing any direct `await shared_call`
or any call to a `MaySend` function is itself `MaySend`.  Subtyping:
`Pure ≤ MaySend`.  Capability check at `await*` of `async* T !ε`:
require send-cap iff ε = `MaySend`.

**Verdict: insufficient for the bench.**  The moment a function
value lives in a record field (an `Accessor.lookUp`), an array, or
any container, the type checker only sees `Accessor` at the use
site.  The effect it can assign is the join across all `Accessor`s
in the program — i.e., `MaySend` if any accessor anywhere sends.
Smurf traversal from a `query` would be rejected regardless of which
specific accessors actually flow through the navigation.

For monomorphic code (no function-valued fields / arrays) this would
work.  The bench is not such code.

### Approach B — effect-polymorphic types

Add effect *variables* (ε, …) alongside type variables.  `Accessor`
becomes `Accessor<ε>`:

```motoko
type Accessor<ε> = {
  form   : { #indexed; #named; #test };
  fourcc : Text;
  lookUp : (Smurf<ε>, LookupKey) -> Smurf<ε>;
};

type Smurf<ε> = {
  accessors : [Accessor<ε>];
  toDesc    : () -> async* ObjectSpec !ε;
  ...
};
```

A `query` body operates on `Smurf<Pure>`; an update body on
`Smurf<MaySend>` (subtyping ε across uses).

**Verdict: works, but is a language extension.**  Motoko gains effect
quantification — kinds for effects, syntax for effect variables in
function signatures and generic parameters, subtyping rules across
effect arguments, inference at instantiation sites.  Annotations
spread to every protocol surface (Accessor, Smurf, AccCallback, …).
Errors are harder to localize: "`Smurf<MaySend>` not assignable to
`Smurf<Pure>`" doesn't tell the user *which* accessor inside the
record forced the upcast.

Cost: 3–4 weeks of focused compiler work, with real language-design
churn — quantifier syntax, kinds, subtyping fixed-points.

### Approach C — points-to / CFA + effect derivation (recommended)

Pivot from type-system tracking to **whole-actor flow analysis**.
Per-call-site, compute the set of concrete function-value targets;
join their effects.  Errors point at the function literal that
introduced the send.

Concretely:

1. After typechecking, run a control-flow analysis on the IR.  0-CFA
   (Shivers '88, context-insensitive) or Andersen-style inclusion
   is enough; both are polynomial in program size.
2. The CFA result is a map *call-site → set of possible function
   literals*.
3. A separate fixpoint computes effects: each function literal's
   effect is `MaySend` if its body contains a direct
   `await <shared-call>`, else the join over its call sites of the
   join over CFA-resolved callees.
4. Capability check at `await` / `await*` sites uses the CFA-
   derived effect of the call's CFA-resolved callee set — not the
   nominal type's pessimistic join.

For the bench: every Accessor literal in the source is a pure body
(array indexing, hashmap probes, predicate evaluation — no
`await shared_call` anywhere in any `lookUp`).  CFA computes:
`acc.lookUp` resolves to N concrete literals, all `Pure` → the call
through `acc.lookUp` is `Pure` → a query traversing the Smurf chain
type-checks.

**Verdict: this is the path.**  No language-surface change.
Diagnostics localize to the offending function literal.  Whole-actor
analysis is well-suited to Motoko's compilation model (each actor
compiled as one closed unit).

## Comparison

| Property                            | A: mono-bit | B: effect-polymorphic | C: CFA |
|---|---|---|---|
| Handles Smurfs                      | ✗           | ✓                      | ✓     |
| Language-surface change             | None        | Effect variables, syntax, kinds | **None** |
| Precision                           | Coarse      | Precise                | **Precise** |
| Error localizes to offending callee | n/a         | Hard                   | **Easy** |
| Inference burden on developer       | None        | Annotations everywhere | **None** |
| Compiler effort                     | 5–7 days    | 3–4 weeks              | 4–6 weeks |
| Modularity (cross-canister)        | Conservative | Conservative           | Conservative at the leaves |

## Implementation sketch (path C)

### Inputs

- Motoko IR post-typecheck.  Closure-converted form simplifies the
  CFA (every function literal is named).
- Existing capability metadata from `async_cap.ml` for the *context*
  side (already enforces direct sends in query bodies).

### Phases

1. **Points-to graph builder.**  Walk the IR; for each variable / field
   of function type, compute the set of concrete function literals
   that can flow in.  Standard subset-based or inclusion-based
   algorithm.  Cubic worst-case; in practice linear-ish for our
   shape.

2. **Effect derivation pass.**  Compute per-function-literal effect:
   - Base case: `MaySend` if the body contains a direct
     `await <shared-call>` *or* any system call known to send;
     else `Pure`.
   - Recursive: at each call site within the body, look up the
     CFA-resolved callee set; the call's effect is the join over the
     set.  Function's effect is the join over its body's call sites.
   - Fixpoint over the call graph (cycles handled by starting at
     `Pure` and widening).

3. **Capability refinement.**  At each `await` / `await*` site,
   compare the CFA-derived call effect against the current
   `async_cap`.  Replaces the type-only conservative check.

4. **Diagnostics.**  New error code (M0264 or similar): "this
   `await*` may transitively send via `<function literal at file:line>`;
   sends are disallowed in a `query` body".  The function-literal
   pointer is the CFA's bookkeeping — already computed.

### Modular boundary

The whole-actor analysis is precise only for code visible to the
compiler.  Sources of opaque function values:

- **`.did` candid imports** — cross-canister.  Sends by definition.
  Treat as a single opaque target with effect `MaySend`.
- **`.mo` imports without source** — rare, but possible.  Function
  signatures coming through an interface file would need an
  explicit effect annotation; default-conservative `MaySend` if
  absent.
- **Function values from `Any` / dynamic upcasts** — opaque.
  `MaySend`.
- **Function-typed parameters** (callbacks the canister exposes) —
  effect is `MaySend` unless the parameter type explicitly says
  otherwise.  Rare in canister code.

The "unknown means MaySend" rule applies only at the *leaves* of the
CFA (where the function value originates outside analysis).  In-
actor code is computed precisely; no developer annotation needed.

## Open questions

- **Recursion in the points-to graph** is standard (fixpoint, terminates
  polynomially); recursion in the effect lattice is also standard
  (Pure ≤ MaySend, finite height, terminates).  Higher-order
  recursion — function that returns function that returns function
  — degrades 0-CFA precision; 1-CFA or k-CFA recovers it at
  exponential cost.  Empirical question whether 0-CFA is precise
  enough for the bench shape; expect yes.
- **Composite queries** — the existing `CompositeCap` /
  `CompositeAwaitCap` machinery already permits limited sends
  (calls to `query` or `composite query` functions).  The CFA-
  derived effect needs to distinguish "may send to any shared
  func" from "may send to a query/composite-query func" — finer
  lattice than `Pure`/`MaySend`.  Probably:
  ```
  effect = { Pure | QuerySend | ShareSend }
  ```
  with `Pure ≤ QuerySend ≤ ShareSend` and the capability check
  picking the appropriate ceiling.
- **Incremental compilation** — re-running CFA on every edit is fine
  at actor scale (~10k lines, sub-second), but if Motoko grows
  cross-actor analysis, this becomes worth thinking about.  Not a
  blocker.
- **Error UX** — does the message say "via this lookUp at line N"
  or "via the chain a→b→c→shared_call"?  The full chain is more
  helpful but harder to render; the immediate culprit might
  suffice for the v1.
- **Interaction with `system` capability** — the existing
  `SystemCap` orthogonally tracks "can call protected system
  functions".  The send-effect lattice is orthogonal to the system
  capability and they compose multiplicatively.

## Why this matters for the bridge

Per the post-buy-in roadmap in `ic-mator.md`:

> 4. **`await*` learns queries.**  PR
>    [#6119](https://github.com/caffeinelabs/motoko/pull/6119)
>    lets `await*` elide IC dispatch on calls to `public` self-actor
>    methods returning `async T`.  Combined with deprivileged-async-
>    star (this doc), a Smurf-traversing query becomes possible: the
>    canister can advertise an `async`-returning public method, the
>    bridge calls it as a query, the body traverses Smurfs via
>    `await*`, and CFA proves no send happens transitively.

That's the use case that motivates getting this right.  Without it,
all Smurf queries have to be `update` methods — full IC update
overhead (consensus, replication, ingress messages) for what is
fundamentally a read.  With it, the same Smurf machinery serves
both modes.

## Recommendation

1. **Defer until query-side Smurf traversal becomes a real use case.**
   Today's canister exposes `lingo` as pure Candid (no Smurf
   touched) and `go` as `update` (sends allowed by context).  The
   architectural gap exists but doesn't bite.
2. **When it bites: take path C (CFA).**  Effort comparable to path
   B but zero language-surface cost.  Diagnostics localize.  Bench's
   protocol shape is exactly the case CFA handles well.
3. **In the meantime, document the policy by comment** in
   `object-spec.mo`: "if you add a `lookUp` that does an `await
   shared_call`, the query-side traversal will trap at runtime;
   when CFA lands this becomes a compile-time error."

When this work starts, the natural sequencing is:

1. Add the effect lattice (`Pure ≤ QuerySend ≤ ShareSend`) to
   `async_cap.ml` or a sibling module.
2. Run CFA on the IR, expose call-site → callee-set map.
3. Derive effects from CFA + function bodies.
4. Wire the capability check at await sites to consult CFA-
   derived effects instead of the type-only pessimistic join.
5. Tests: bench `tinyN` smoke; deliberate query-side leak; the
   "all-pure lookUps" property.
