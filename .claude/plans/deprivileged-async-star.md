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

## Intuition: the little-disks picture

The implementation is a standard worklist + fixpoint algorithm, but
the *intuition* that makes the design choices fall out naturally
came from a topological framing — recorded here because it pre-
empts at least four of the implementer's questions before they
arise.  Treat this section as visualisation, the same way string
theorists draw worldsheets to explain a calculation whose actual
machinery is contour integrals.  No formal claim that this is an
operad in the strict sense; the geometry is the explainer, not the
implementation.

### Outer disks, inner disks

Each `async*` function is an **outer disk**.  Embedded inside it
are **inner disks** — one per call site within the body.  The
inner disks are of two kinds:

- a direct `await` of an `async` shared call (an IC message);
- a call to another `async*` function (which itself is an outer
  disk somewhere).

Static syntax is the disk arrangement: each `async*` function's
body is a region with a fixed set of inner disks at fixed
positions.

### Colour radiates outwards

Each inner disk radiates a **colour** representing the effect of
what the call resolves to:

- **green** — `Pure`, no send;
- **red** — `MaySend`, sends an IC message;
- **yellow** — `MayThrow`, raises an error (a second axis,
  orthogonal to red/green; see "Other effects" below).

The colour radiated by an inner disk is determined by the colours
of the outer disks the call resolves to — flowing inwards through
either:

- a **tube** to the outer boundary of a concrete callee (when the
  call is first-order: the function is named by identifier and the
  type system pins down a single outer disk); or
- a **wormhole** to a *set* of possible outer disks (when the call
  is higher-order: through a field, return value, function-typed
  parameter — any case where the call site doesn't statically name
  the target).

The wormhole's *candidate set* is the points-to result: which outer
disks could this wormhole connect to at runtime?

### Mixing on the surface

Inside an outer disk, between the inner disks and the outer
boundary, lies the function body's **surface**.  As the inner
disks radiate colour into the surface, the surface mixes them.
The mixing rule is **longest wavelength dominates** — i.e., the
join of the effect lattice.  Red dominates green; if any inner disk
is red, the surface is red.  (For yellow added in: red and yellow
are independent dimensions, so the surface carries both
independently.)

The mixed colour reaches the **outer boundary** of the disk and
becomes the colour that *this* outer disk radiates to its
downstream consumers.

### The boundary flip — outlets vs receivers

A subtle point worth pinning down: each source-region is both an
*outlet* (radiating outward when it appears as an outer disk being
analysed) and a *receiver* (its outer boundary becomes the source
that downstream inner disks receive from).  The picture is:

```
{   ... ] . [ ... ] . [ ... ] . [ ...  }
```

where each `[ ... ]` is a function body's region, the closing `]`
is its outer boundary radiating its mixed colour outward, and the
opening `[` is *another* call site somewhere downstream receiving
that colour as part of its own mixing.  The `.` is the connection —
a tube or wormhole.

This bidirectionality matches the standard effect-system rule:
each function's *outer effect* is the join of its body's inner
*effect demands*.  Algorithmically, points-to flows forward (where
do function-values go?), effects flow backward (what does each
region radiate?), and they fixpoint together.

### Tubes vs wormholes — knowing the candidate set

The whole game is **shrinking the wormhole's candidate set**:

- A *small* candidate set means CFA can compute the precise join
  of colours and propagate that outward.
- A *large* or *unknown* set means the wormhole is conservatively
  red, because any unknown callee could send.

Three partitioning dimensions help shrink the candidate set:

1. **Type partitioning.**  Motoko's type system already groups
   function values into compatible signature classes.  A field of
   type `(Smurf, LookupKey) → Smurf` can only ever receive
   functions of that exact type.  The CFA candidate set is
   *intersected* with type-compatible candidates — for free,
   courtesy of the type system.  This is the largest win against
   the bare-CFA worst case: the bench's Accessor.lookUp field has
   maybe 30 concrete candidates across the whole canister, not
   "every function in the program".

2. **Field-name partitioning.**  Field-sensitive points-to analysis
   (Andersen-style with field discrimination): each field has its
   own points-to set, scoped by name.  A function flowing into
   `accessor.lookUp` is tracked separately from one flowing into
   `accessor.someOtherField`.  Standard refinement; bench needs it
   to keep clientSmurf / cardSmurf / charSmurf wrap callbacks
   distinct in the analysis.

3. **Returner-of-async\* partitioning.**  When function `f` returns
   an `async*` value, the returned values from *different functions*
   can be tracked separately.  This is k-CFA's contribution; k=0
   collapses returners, k≥1 distinguishes per call site.  For our
   shape 0-CFA is probably enough; escalate to 1-CFA if a real
   precision-loss case appears.

### Non-`async*` code is in scope

The analysis cannot stop at `async*` boundaries.  A regular
non-`async*` function that captures an `async*`-returning callback
and stores it in a record propagates the callback's colour through
a wormhole that traverses a non-`async*` region.  Function values
must be tracked **whole-actor**, not bounded to `async*` types.
The CFA follows function references everywhere they go — through
non-`async*` code, through closures, through ordinary fields —
because that's where wormholes can route.

### Other effects (yellow, …)

The picture generalises: each independent effect dimension is a
separate colour axis.  Yellow for `MayThrow`.  Hypothetical future
axes could include `MayAccessCertifiedData` or
`MayConsumeCycles`.

Algorithmically: the effect lattice is a product of one chain per
axis (e.g., `Send × Throw`, each chain `Pure ≤ MayX`).  The join
is component-wise.  Capability checks consult only the relevant
axes (a `query` checks Send; an exception-free annotation would
check Throw).

For v1 we ship only the Send axis; the product shape is reserved
so the v2 throw-tracking work can be added without re-engineering.

### Why the topology is more than decoration

Two design choices the picture makes obvious that the bare CFA
pitch left implicit:

- **Bidirectional fixpoint.**  Points-to flows forward; effects
  flow backward; the geometry shows them meeting at the boundary.
  Treating the analysis as bidirectional from the start avoids the
  trap of computing PT-then-effects-as-a-separate-pass (which
  doesn't converge cleanly for mutually-recursive call graphs).
- **Type-directed candidate pruning is a first-class step.**  The
  picture makes the partition visible: wormholes can only connect
  to type-compatible disks.  In a sloppier framing, type
  compatibility might get treated as a sanity-check afterthought;
  here it's load-bearing precision.

The picture also makes `toDesc` decidable in the bench specifically:
charSmurf's `toDesc` is `await* parent.toDesc()`, and `parent` is
typed `Smurf`.  Through type-directed pruning + field-sensitive PT,
the analysis sees a finite set of `toDesc` candidates (one per
*Smurf-constructor in the bench), all of which have green inner
disks.  The recursion is structural, the fixpoint terminates,
queries can traverse Smurfs.

## Implementation sketch (path C)

### Inputs

- Motoko IR post-typecheck.  Closure-converted form simplifies the
  CFA (every function literal is named).
- Existing capability metadata from `async_cap.ml` for the *context*
  side (already enforces direct sends in query bodies).

### Effect lattice

For v1, a single chain:

```
Pure ≤ MaySend
```

For v2+, a product of independent chains, one per effect axis:

```
(Send: Pure ≤ MaySend) × (Throw: Pure ≤ MayThrow) × …
```

Join is component-wise.  Capability checks consult only the relevant
component (a `query` body checks Send; a no-throw annotation —
when/if added — would check Throw).  Ship the product structure
on day one even if only Send is populated, so v2 doesn't
re-engineer the lattice.

### Phases

1. **Points-to graph builder** (field- and type-sensitive).  Walk
   the IR; for each variable / field of function type, compute the
   set of concrete function literals that can flow in.  Two
   refinements over a naive Andersen:

   - **Field-sensitive**: per-field-name points-to sets.  A
     function flowing into `accessor.lookUp` is tracked separately
     from one flowing into `accessor.someOtherField`.
   - **Type-directed pruning**: the candidate set at any wormhole
     is *intersected* with the set of type-compatible function
     literals.  Motoko's type checker has already established
     these compatibility classes — the CFA reads them off, no
     extra type-analysis pass.  This is the largest precision win
     over bare CFA; on the bench it shrinks Accessor.lookUp
     candidates from "every function in the actor" to "the ~30
     `lookUp` literals across the Smurf protocol".

   Function values are tracked **whole-actor** — through
   non-`async*` code, closures, ordinary fields, return paths.
   Restricting tracking to `async*` boundaries misses wormholes
   that route function references through regular functions.
   Cubic worst-case; in practice linear-ish for our shape.

2. **Effect derivation pass** (backward flow).  Compute per-
   function-literal effect:
   - Base case: each axis bit is set if the body contains the
     corresponding triggering construct (`await <shared-call>` for
     Send; `throw` for MayThrow; …).  Else axis bit clear.
   - Recursive: at each call site within the body, look up the
     CFA-resolved callee set; the call's effect is the join over
     the set.  Function's effect is the join over its body's call
     sites.
   - Fixpoint over the call graph (cycles handled by starting at
     `Pure` on each axis and widening).

3. **Bidirectional fixpoint.**  Points-to (forward: where do
   function values go?) and effect derivation (backward: what does
   each region radiate?) interact at higher-order calls — the
   effect computed for a function influences future PT iterations
   only insofar as later effect refinements don't shift PT, which
   in practice they don't (PT is purely about values flowing).  In
   typical cases the two passes can be sequenced (PT first, then
   effects); if the call graph contains pathological mutual
   recursion the implementation falls back to alternating until
   both fixpoints stabilise.

4. **Capability refinement.**  At each `await` / `await*` site,
   compare the CFA-derived call effect against the current
   `async_cap`.  Replaces the type-only conservative check.

5. **Diagnostics.**  New error code (M0264 or similar): "this
   `await*` may transitively send via `<function literal at file:line>`;
   sends are disallowed in a `query` body".  The function-literal
   pointer is the CFA's bookkeeping — already computed.  When the
   send happens *through* a chain of higher-order calls, the
   message can show the wormhole-walk path: "this lookUp at L1 was
   reached via that lookUp at L2 was reached via …".  More
   helpful than "MaySend ≠ Pure", which is what a type-system
   error would say.

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
