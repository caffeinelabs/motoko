# Canister Object Query — AEOM-inspired Heap Querying

Design plan for a composable, auditable object query interface for Motoko
canisters, inspired by the Apple Event Object Model (AEOM) and its Object
Support Library (OSL).

---

## Motivation

A canister's query API can say anything — queries are not certified and users
cannot verify the response independently. What users need:

1. The canister's *code* is auditable (verifiable Wasm hash on-chain)
2. The *data* is reachable through that auditable code by a declared path
3. Optionally: the result is **certified** (anchored to consensus via
   `setCertifiedData`)

An AEOM-style object query interface addresses (2) directly: the traversal
logic is part of the open-source canister Wasm, so the path from root to data
item is itself auditable. Object specifiers are *data* — composable, storable,
comparable across independent auditors.

---

## Core Types (pure Motoko)

```motoko
type KeyForm = {
  #absolutePosition : Int;   // 1-based; negative counts from end
  #name : Text;
  #uniqueID : Nat;
  #property : Text;
  #range : (ObjectSpec, ObjectSpec);
  #test : BoolExpr;
};

type ObjectSpec = {
  #root;
  #object : { class_ : Text; container : ObjectSpec; key : KeyForm };
};

type Comparison = { #eq; #ne; #lt; #gt; #le; #ge };

type BoolExpr = {
  #compare : { prop : Text; op : Comparison; value : CandidValue };
  #and_ : (BoolExpr, BoolExpr);
  #or_ : (BoolExpr, BoolExpr);
  #not_ : BoolExpr;
};

type Token = ...; // app-defined; opaque Blob or typed variant

type Certificate = Blob; // IC certificate blob
```

---

## Canister Interface

```motoko
// Standard query endpoint — uncertified but auditable via code hash
public query func objectAt(spec : ObjectSpec) : async ?CandidValue;

// Certified variant — returns value + IC certificate for offline verification
public query func certifiedObjectAt(spec : ObjectSpec)
    : async ?(CandidValue, Certificate);
```

Canisters implement the `Queryable` interface by providing a `resolve`
function. The resolver is a simple recursive pattern match over `ObjectSpec`;
no external library or registry needed.

---

## Certified Data Integration

On every write the canister commits a sparse Merkle tree of
`hash(specifier) → hash(value)` via `ic0.data_certificate_present`:

```motoko
stable var certTree : CertifiedMap = CertifiedMap.empty();

func onWrite(spec : ObjectSpec, value : CandidValue) {
  store(spec, value);
  certTree := CertifiedMap.put(certTree, encodeSpec(spec), hash(value));
  IC.setCertifiedData(CertifiedMap.root(certTree));
};
```

This gives users Merkle-proof-style verification without trusting the query
node — equivalent to Ethereum's `eth_getProof` but at the data-model level.

---

## Two Levels of Exposure

| Layer | What you see | Trust basis |
|---|---|---|
| **App-level** (data model) | Accounts, records, tokens | Code audit + IC certs |
| **RTS/heap-level** | Raw GC objects, fields by offset | Code audit + RTS code |

For user-facing reassurance, **app-level is correct**. RTS-level is for
developer tooling and profiling. The two can coexist: a special object class
`"heapObject"` could expose GC-managed objects by stable address, accessible
only to controllers.

---

## Use Cases

### 1. Token / Ledger Proof of Reserves
A DeFi protocol holds user balances in a canister. Any user can construct:

```
account(uniqueID: 0xDEAD) → property("balance")
```

and get a certified answer without trusting the frontend. An auditing service
can sweep all accounts and verify the sum matches the protocol's claimed TVL.
No custom audit endpoint needed — the standard object query interface suffices.

### 2. NFT Ownership Verification
A marketplace queries a canister:

```
nft(uniqueID: 42) → property("owner")
```

The result plus IC certificate proves ownership at a specific IC block height,
usable as a receipt. No bespoke "ownerOf" method needed — ownership is just a
property in the object model.

### 3. DAO Governance State Inspection
Governance proposals, votes, and member weights are objects. An external
monitor can traverse:

```
proposal(absolutePosition: -1)  // latest
  → property("voteTally")
```

Independent observers can verify vote counts match without accessing a
privileged endpoint.

### 4. GDPR-style "Show Me My Data"
A user knows their principal. They query:

```
user(uniqueID: <principal>) → range(property("first"), property("last"))
```

and receive all their stored fields with a certificate. This gives users a
verifiable, portable record of what the canister holds about them — no custom
export function per field required.

### 5. Monitoring and Alerting Dashboards
A monitoring service periodically evaluates:

```
account(test: balance < 100) → property("id")
```

— "all accounts with balance below threshold" — via `formTest`. The query
grammar is expressive enough to drive dashboards without application-specific
API surface. Specifiers are stored in the monitoring config as plain data.

### 6. Cross-Canister Auditing
Because `ObjectSpec` is a plain Candid type, one canister can issue an object
query *to another canister* (as an inter-canister call). An auditor canister
can systematically crawl a suite of related canisters using a shared specifier
grammar — think of it as a structured spider.

### 7. macOS Automator / AppleScript Integration

The dream syntax:

```applescript
tell application "ICmator"
    get every client whose country = "Germany"
end tell
```

macOS Automator and AppleScript speak Apple Events whose object references
map directly onto `ObjectSpec`.  Because the canister's `(with encoder;
decoder)`-annotated methods already speak the AE binary wire format
natively, an intermediary scriptable macOS app/daemon ("ICmator") can be
a **pure byte pass-through** — no JSON, no Candid-on-macOS, no schema
duplication.

See [`ic-mator.md`](./ic-mator.md) for the full design (SDEF shape, AE
handler dispatch, IC transport via a small Rust subprocess wrapping
`ic-agent`, bundle layout, demo scope, App Intents alternative).

### 9. LLM-Driven Canister Conversation
Candid's rigid type system is a poor match for the open-ended, exploratory
nature of LLM interactions. An LLM agent needs to discover what data a canister
holds, navigate its structure, and answer user questions — but Candid forces it
to know the exact method name, argument types, and return schema upfront.

An object query interface changes this fundamentally. The LLM can:

1. **Discover** — query `#root → property("objectClasses")` to learn what
   kinds of objects the canister exposes, without consulting an IDL file
2. **Navigate** — follow specifier chains interactively, exploring containers
   and properties as they are revealed, just as a user browses a file system
3. **Filter** — use `formTest` to answer natural-language questions like
   "which proposals have more than 1000 votes?" without a bespoke method
4. **Verify** — retrieve certified results to ground its answers in
   on-chain state rather than hallucinated structure

The LLM generates `ObjectSpec` values from natural language; the canister
resolves them; the LLM interprets the result and either answers or refines the
query. This is a natural fit because object specifiers are *composable data*,
not function calls — the LLM can build them incrementally and correct mistakes
without re-reading an IDL.

Compared to the Candid approach (where the LLM must guess or be given the full
interface schema), the object query model is self-describing: the data model
is discoverable at runtime, class names and property names are strings, and
errors can be recovered from by trying adjacent paths. This mirrors how LLMs
already interact effectively with REST/JSON APIs versus strongly-typed RPC.

### 8. Developer Heap Debugger
A controller-gated `#heapObject` class exposes GC-managed objects by stable
pointer address. A debugging tool can:

```
heapObject(uniqueID: 0x1F4A00) → property("tag")
heapObject(uniqueID: 0x1F4A00) → property("fields") → absolutePosition(2)
```

Combined with the object class registry (tag → type name mapping), this gives
a structured heap walk — far more usable than raw `memory.inspect` dumps.

---

## Implementation Roadmap

1. **`ObjectSpec` / `BoolExpr` types** — library module, no RTS changes
2. **Resolver skeleton** — recursive `resolve : ObjectSpec -> AppState -> ?Token`
3. **`CertifiedMap` integration** — sparse Merkle tree over specifier hashes
4. **`formTest` evaluator** — typed predicate interpreter
5. **HTTP JSON endpoint** — `http_request` handler encoding/decoding specifiers
   as JSON (prerequisite for Automator / non-Candid clients)
6. **RTS introspection hooks** (optional, controller-gated) — safe GC-managed
   object iterator for the heap debugger use case

Steps 1–2 are pure Motoko, implementable today. Step 5 depends on the
non-Candid endpoint work already planned. Step 6 requires RTS changes.

---

## Progress (as of 2026-04-27)

Lives in [test/bench/object-spec.mo](../../test/bench/object-spec.mo) on
branch `gabor/encoder` (PR #5996).

### Shipped

- **AE wire codec** — full round-trip decoder + encoder for the running
  `'dle2'` envelope: `'obj '`/`'null'`/`'utxt'`/`'long'`/`'enum'`/`'logi'`/
  `'cmpd'` plus the `'name'` keyform. 638-byte fixture decodes/encodes
  byte-identical (modulo `'exmn' → 'null'` collapse for the iterand and
  the `textToUtf16` ASCII-only mangling of non-BMP UTF-8).
- **Smurf protocol skeleton** — existential via Candid blobs:
  - `type Smurf` (lazy `blob : () -> Blob`, `classFourcc`, `accessors`,
    `enumerate`, `readField`, `toDesc`, `isNotFound`).
  - `type Accessor` (`form`, `fourcc`, `lookUp(parent : Smurf, key : LookupKey) : Smurf`).
  - `type LookupKey` (`#indexed Int | #named Text | #test BoolExpr`).
  - Mutual recursion `Smurf ↔ Accessor`; protocol surface is monomorphic
    and shareable; `T` is closed inside method bodies via
    `from_candid<T>` / `to_candid` (the `_clientSmurf` constructor
    closes over `Client` directly, demonstrating the fast typed path).
- **Concrete constructors**:
  - `_notFoundSmurf` — AE-404 sentinel (`isNotFound = true`,
    `toDesc = #root` placeholder; eventual `errAENoSuchObject` envelope).
  - `_ValueSmurf` — terminal leaf class; reads parent's field at construction
    and discards the parent.
  - `_VarAccessor<T>` — typed escape hatch over a captured `[T]`. Indexing
    is 1-based with AppleScript negative-from-end (`-1` = last);
    out-of-range → `_notFoundSmurf`. Form-guarded: matches on
    `(form_, key)` so a `#named`-declared accessor doesn't fire on a
    `#indexed` key.
  - `_clientSmurf` — `(Client, Smurf) → Smurf`. `toDesc` closes over
    `parent.toDesc()` (the zipper edge) and uses `c.name` as primary
    key, producing AppleScript-equivalent `client "<name>" of <root>`.
  - `_actorSmurf` — canister root; `accessors[0]` is the `clnt`
    `_VarAccessor<Client>` over the mock DB.
- **Mock DB** — 100 Clients, deterministic, ~30% match the running
  predicate (`country=="Germany" AND 45<=age<=55`). 10×10 French/German
  name pools yield unique primary keys; an O(n²) init-time assertion
  enforces it.
- **`tiny1(i : Int) → ObjectSpec` public method** — drives the protocol
  end-to-end: `_actorSmurf.accessors[0].lookUp(_actorSmurf, #indexed i)`,
  surfaces `s.toDesc()` over `(with encoder)` to the wire. Demonstrated
  with `tiny1(1)` ("Hans Müller"), `tiny1(-1)` and `tiny1(100)` (both
  "Camille Girard" — addressing equivalence).
- **Original benchmark `go(spec) → spec`** still runs the typed-fast-path
  `runQuery` (returns `[CandidValue]` of matching incomes via the
  monomorphic `PropReader`), separate from the existential protocol.
  Cycle/heap numbers visible in the .ok file.

### Distance from the vision

| Roadmap step | Status |
|--|--|
| 1. `ObjectSpec` / `BoolExpr` types | ✓ done |
| 2. Resolver skeleton | partial — `_VarAccessor` + `_clientSmurf` lookup works for `#indexed`; full `interpret(spec) : Smurf` walking the variant tree across both layers (clnt → prop) is **not yet wired** through the protocol. The typed-fast-path `runQuery` gives the answer for the running query but bypasses the existential boundary. |
| 3. `CertifiedMap` integration | not started |
| 4. `formTest` evaluator | partial — `evalBoolExpr` works in the typed-fast-path; still TODO inside the `_VarAccessor` (no `#test` form), and `cmpd`'s obj1 doesn't yet drive the resolver to read leaves through the existential boundary |
| 5. HTTP JSON endpoint | not started |
| 6. RTS introspection hooks | not started |

### Protocol-level gaps

- **`#named` lookup** — `_VarAccessor` only handles `#indexed`. Filter
  by name (the inverse of `toDesc`'s primary-key emission) is the next
  natural extension.
- **`#test` filter** — needs to live somewhere; current sketch is to
  add `filter : BoolExpr → Smurf` on `Smurf` itself rather than as a
  third `Accessor.form` (since cardinality differs from point lookups).
- **Property accessors on `_clientSmurf`** — its `accessors : []` is
  empty; `_clientSmurf.accessors` should host four leaf accessors
  (name/cntr/age/inco) so navigation can drill into a found Client.
- **`'exmn'` ↔ `#it`** — currently collapsed to `#root` (lossy); an
  `#it : ()` variant on `ObjectSpec` would round-trip py-appscript
  output exactly. The user noted this is "really an algebraic effect"
  — Motoko has no algebraic effects, and the current bench predicate
  doesn't exercise `it as value` (only `it.<property>`), so the
  collapse is fine until extended.
- **Real UTF-16 BE in `textToUtf16`** — current ASCII assumption mangles
  non-BMP / multi-byte UTF-8. Documented in the function header.
- **`#ne` encoder** — traps; should desugar to `NOT(=)`.

### Verdict (post-`synthetic-properties` tag, 2026-05-20 evening)

Smurf protocol verdict, focused on the canister/library side
(bridge-level verdict in `ic-mator.md`):

**Composes in the small.**  `CollectionSmurf` filters via `#and_`;
`FlattenedSmurf` is a CollectionSmurf over a pre-flattened source;
the protocol surface stayed stable through (a) the multi-canister
addressing work, (b) the lingo discovery work, (c) tonight's
char-positional rewrite (only needed widening `wrap : (T, Smurf)
→ Smurf` to `(T, Nat, Smurf) → Smurf` — minor surgery).

**Three concerns growing inside the bench:**

1. **The bench is doing too much.**  `test/bench/object-spec.mo`
   is ~2200 lines covering protocol, codec, semantics, 20+ `tinyN`
   demos, and `lingo()`.  The natural seams are sharp: codec
   (writeDesc/parseTopLevel/4cc tables) and Smurf protocol can
   each become a mops package.  Bench keeps `Client`/`CreditCard`
   and demos only.
2. **Hand-rolled redundancy across multiple sites.**  Lingo
   description lives in three places (Rust `bench_lingo()`,
   Motoko `lingo()`, `ICmator.sdef`) plus AS demo file.  Tonight
   alone we hit drift twice (the first/last name rename + the
   PropReader gap).  `mo_to_lingo` codegen would close this loop;
   until then the protocol's "self-describing" property is only
   honoured by developer vigilance.
3. **Protocol accreted, needs a v1 freeze.**  Today's signature
   works but grew organically: `wrap` recently widened (chars),
   accessors have four forms
   (`#indexed`/`#named`/`#test`/property-as-class),
   `notFoundSmurf` does double duty as both error sentinel and
   navigation default, and `getName` is dead in some
   instantiations (the char collection passes `charToText` but
   the char class doesn't expose `#named`).  Pin a v1 surface
   and freeze as a versioned mops package.

### Post-buy-in cleanup arc

Order in which to act once stakeholder buy-in lands on the demo:

1. **Tease out reusable code into libraries.**  Split
   `object-spec.mo` along the natural seams:
   - `motoko-ae-codec` — wire-format codec (writeDesc,
     parseTopLevel, 4cc constants).  Reusable by any
     `(with encoder; decoder)`-aware canister.
   - `Accessors.mo` — Smurf protocol surface (per the long-term
     shape sketched below).  Bench retains only its
     entity-specific `clientSmurf` / `cardSmurf` and the 20+
     `tinyN` demos.
2. **Finalise the Smurf interface.**  Freeze:
   - `Smurf` record shape (`class4cc`, `accessors`, `toDesc`,
     `filter`, … decide on `isNotFound` vs sentinel-pattern).
   - `Accessor` shape and the four forms
     (`#indexed`/`#named`/`#test`/property-as-class).
   - `wrap : (T, Nat, Smurf) → Smurf` final signature (commit to
     position-always-passed, even when ignored, per tonight's
     `_` decision).
   - `notFoundSmurf` semantics — error sentinel or navigation
     default? today it's both; pick one or split.
3. **Define `midl` and `any`.**  Codified mapping from Motoko
   types to the OSL surface — the typed-IDL story.  Plus the
   universal-type fallback (`****` / `typeWildCard` on the AE
   side, `Any` Motoko-side) so the future Candid↔AE bridge in
   the Rust agent has a typed top to land in when the schema
   doesn't cover a value.  This is the contract `mo_to_lingo`
   codegen produces against.
4. **`await*` learns queries.**  Today the Smurf `toDesc` is
   `async*` and the canister's public `go` is `async` — composing
   them works because `go` `await*`s into the smurf chain
   internally.  But once the bridge does optimisation passes that
   want to share `await*` across query/update boundaries, the
   current rule "`await*` only on local `async*` functions" bites.
   PR [#6119](https://github.com/caffeinelabs/motoko/pull/6119)
   ("experiment: `await*` on `public` self-actor methods") is the
   in-flight machinery — it lets self-calls into `public` methods
   returning `async T` elide IC message dispatch.  Need to fold
   this into the Smurf execution model so a query-classified
   canister method can be composed via `await*` from another
   query.

(Steps 1–2 unblock 3; 4 lands independently once 6119 is in.)

### Query optimisation: positional pickaxe vs. eager `map`

A class of queries shipped today builds a full `[T]` and then picks
one element.  AppleScript renders this naturally as
`item 6 of (every client whose ...)`, which the bench currently
resolves by materialising the whole filtered list, mapping `clientSmurf`
over it, then indexing.  When the filter is **trivial** (no `whose`),
the materialised list is identical to the underlying stable array —
N × `clientSmurf` allocations + the resulting `[Smurf]`, just to throw
away `N-1` of them.

The rewrite (no semantic change, no side effects):

```
root.<plural> |> map f |> .[6]      ==      root.<singular>.[6] |> f
```

i.e. `every client` then `item 6` is equivalent to `client 6` —
direct indexed-form lookup followed by the per-element `f`.  Same
result, O(1) instead of O(N).

Generalisation: the same rewrite applies whenever the **terminal
operation** is a stable index pick (1-based, negative-from-end,
range slice) and there's an unfiltered prefix.  The OSL can spot
this in `resolve`:

- Spec shape: `#obj { class_ = X; key = #absolutePosition n;
                     container = #obj { class_ = X; key = #every;
                                        container = ... } }`
- Rewrite to: `#obj { class_ = X; key = #absolutePosition n;
                     container = ... }`  (the inner `#every` layer
                     collapses).

For the **filtered** case — `#test` predicate in the middle — we
can't pre-skip, but we shouldn't be eager either.  Today `CollectionSmurf`
walks the source and materialises every match's `toDesc()` up
front.  A lazy version keeps `pred` + `source` in the smurf and
only forces matches when something downstream actually demands them
(e.g. the `#absolutePosition` reducer, or `count`).  `count` over a
filter then becomes a single linear scan with no element-Smurf
allocations.

Order of work:

1. Spec-level rewrite for the unfiltered-then-pick pattern in
   `resolve` (small, no protocol change).
2. Lazy `CollectionSmurf` — keep `pred` + `source`, defer the
   per-element wrap until `toDesc` is forced; `filter` composes via
   `#and_` without producing intermediate lists; `count` does the
   scan directly.
3. Lazy `FlattenedSmurf` — same treatment for `every card` (across
   clients): scan parents on demand, yield per-child smurfs lazily.
4. Once (2) and (3) land, the spec rewriter from (1) becomes a
   pure optimisation rather than a correctness lever — the
   non-rewritten path is asymptotically the same.

Expected wins on the bench's `tinyN`s: `tiny8`'s position-pick over
a filtered list goes from `O(filter-scan + materialise + pick)` to
`O(filter-scan-until-Nth-match)`.  The same shape with
`#absolutePosition 1` (first) becomes a short-circuit scan.

---

## Germinating: model-specific post-decoder optimiser

Idea worth letting sit before we act on it.  This section sketches
the design space and the analytical critique that goes with it — no
implementation is planned until the points under "Triggers for
investing" below are met.

### The split (three concerns)

Today's path is `bytes → ObjectSpec → Smurf navigation` (roughly:
`parseTopLevel`, then `resolve`).  Two things blur together at the
join: the parser produces an `ObjectSpec` that is *both* the faithful
decoding of the AS-author's spec *and* the program the resolver will
execute against the smurf tree.  Those are different concerns —
they only happen to be the same data type today because the easy
cases work.

The proposed split:

1. **Decoder** — faithful `bytes → ObjectSpec`, pure syntax.  What
   `parseTopLevel` already is.  Has no opinion on cost, never
   rewrites.
2. **Optimiser** — `ObjectSpec → ObjectSpec`, **model-specific**.
   Knows this canister's cost surface: which accessors are O(1)
   vs O(N), which predicates are selective, which joins are cheap.
   Rewrites the spec into a semantically-equivalent but cheaper
   shape.  Lives outside the AE protocol entirely.
3. **Resolver** — walks the (possibly optimiser-rewritten) spec
   against the smurf tree.  Today's `resolve`, plus one new arm per
   internal custom form (see below).

### Internal custom KeyForms — the clean trick

To express "evaluate this lazily" or "this collection should
short-circuit at the first match" *without* polluting the public
wire format, the optimiser emits new `KeyForm` variants that the
decoder never produces and the encoder never emits.  Mnemonic:
`#indX` for the lazy version of `#indx`, `#tesX` for short-circuit
`#test`, etc.  The 4cc surface contract is unchanged — these
variants exist only inside the canister's data path between
optimiser and resolver.

(The capital-letter casing is a nod to Apple's "app-specific 4ccs
must contain at least one uppercase" rule.  Since these variants
never serialise, the rule formally doesn't apply, but the casing
is a useful "this is not standard AE" visual marker.)

### Why this is structurally good

- **Separation of concerns.**  Decoder correctness is byte-level
  parsing; optimiser correctness is rewrite preservation; resolver
  correctness is navigation.  Each is testable in isolation.
- **Model knowledge stays local.**  Today, if we wanted lazy
  collections, we'd weave the decision into `resolve` somehow —
  scattering data-model awareness across every spec arm.  With the
  split, all of it lives in one rewrite pass that consumes a known-
  shape input and produces a known-shape output.
- **No wire-format change.**  Internal forms never escape;
  canister-side flexibility doesn't propagate to AS clients, the
  bridge agent, or any future Candid path.  Important given the
  hard rule from `.claude/plans/ic-mator.md` that ObjectSpec is
  not a stable public schema.
- **Codegen alignment.**  The same `mo_to_lingo`-style pass that
  derives the canister's `__lingo` query from accessor structures
  can derive the cost model: `VarAccessor<T> #indexed` is O(1),
  `#named` is O(N) linear scan, `FlattenedSmurf` is O(parents ×
  children).  Mechanical, sound, evolves with the code.
- **Composable with the simple rewriter.**  The
  unfiltered-then-pick rewrite we already sketched (`every X then
  item N` → `X N`) is the first pass in this framework.  Adding
  `#indX` annotations on what survives is the second.

### Critique — where this gets hard

- **Optimiser correctness is subtle.**  Rewrites must preserve
  semantics under every interleaving of side effects (we have
  none today, but predicates with async-prim calls would change
  that), under predicate evaluation order, under failure
  propagation.  SQL optimisers ship correctness bugs that persist
  for years in major databases.  Each new rewrite needs property
  tests or a hand proof.
- **Cost-model accuracy.**  Three sources, each with a flaw:
  - *Hand-written per canister.*  Most accurate snapshot; drifts
    from reality as code evolves; can't compose with codegen.
  - *Accessor-derived* (the codegen path).  Mechanical and
    automatic, but blind to data distribution.  The classic
    example: `country = "Germany"` selects 60% of the bench but
    `country = "Liechtenstein"` selects 0%.  The cost model only
    sees "linear scan over [Client]" for both.
  - *Profile-driven.*  Captures distribution, but needs
    persistent counters across canister calls — awkward
    semantically (when do you reset? upgrade-stable?) and
    architecturally (now resolver is a writer, not just a reader).
- **"Internal" forms leak in error paths.**  `debug_show
  (#indX i)` exposes the internal form in trap messages, log
  output, the notFound spec carried by the smurf chain
  (`notFoundSmurf.toDesc` already does this).  Need a convention:
  optimiser output is normalised back to public forms before any
  user-visible serialisation, OR error machinery learns to render
  internal forms in a "diagnostic" mode that's clearly not the
  wire format.  Either is fine; pick one.
- **Per-canister optimiser growth.**  If every canister gets its
  own optimiser pass (hand-written), the codebase grows with the
  ecosystem and nobody maintains the older ones.  Codegen-derived
  optimisers compose better but have the cost-model-accuracy
  problem above.  Probably the right answer is: derive the
  baseline from accessors (the 80%), let canister authors override
  for hot spots (the 20%), with the override surface being a
  function on `ObjectSpec` that the canister author writes.
- **Premature optimisation risk.**  The bench works fine eagerly
  today.  We're not yet sure where actual hot spots are.  Without
  benchmark coverage that shows *which* shapes are slow under
  realistic workloads, we might invest in optimising things that
  don't matter.  The lazy `CollectionSmurf` rework (no optimiser,
  just smurf laziness) probably captures most of the win on its
  own; the cost-modelled optimiser is the bigger ROI when lazy is
  already in.

### Triggers for investing

Worth building when at least three of these are true:

- The bench has a query whose eager + simple-rewriter path is
  measurably the bottleneck (bench cycle counts show it; we have
  a regression test that pins the cycle delta).
- A second canister has joined the picture (so the model-
  specific argument actually has > 1 instance to specialise for).
- Lingo carries enough type structure to derive accessor cost
  classes mechanically.
- We have a debug/trace facility that surfaces optimiser
  decisions so bugs are diagnosable when (not if) they happen.
- The lazy `CollectionSmurf` / `FlattenedSmurf` rework has
  shipped and is the new baseline that optimiser wins are
  measured *against*.

### Pass ordering and interaction with the simple rewriter

When this lands, the two passes need a deliberate join order:

1. **Eliminations first.**  Unfiltered-then-pick collapses
   intermediate `#every` layers entirely.  Run this before
   anything else looks at them; otherwise the laziness pass wastes
   work tagging layers that get deleted.
2. **Laziness annotations second.**  Walk what survives,
   tag collection shapes with `#indX` where the cost model says
   eager materialisation is wasteful.
3. **Constant folding / predicate normalisation third** (later,
   if motivated).  Standard SQL stuff: push selective predicates
   first in conjunctions, fold `true and X` to `X`, etc.

### Analogs in the wider literature

- **SQL optimisers** (Cascades framework, Volcano).  Predicate
  pushdown, join reordering, index selection.  Mature; cost models
  are profile-driven there.  Our analog has fewer join
  permutations to consider (no general joins beyond
  `FlattenedSmurf`) so the search space is much smaller.
- **LINQ providers in .NET.**  Translate `IQueryable` expression
  trees into provider-specific query plans.  Our `ObjectSpec` is
  closer to a LINQ expression tree than to SQL AST.
- **Object-oriented database query** (O2, ObjectStore, late
  80s/early 90s).  Closer in spirit to our model than SQL.  Path
  expressions over typed object graphs; same cost-modelling
  tradeoffs we're contemplating.
- **GraphQL DataLoader.**  Solves the related "lazy resolution +
  batching" problem at the network boundary.  Not directly
  applicable (our boundary is in-canister) but the pattern of
  deferring fetches until forced is the same.

### One-line summary for future-me

Decoder is syntax, optimiser is model-specific semantics
preservation under cost rewrites, resolver is navigation.
Internal custom KeyForms (`#indX`) are the laziness ABI between
optimiser and resolver, kept off the wire.  Don't build until the
triggers list is satisfied.

---

## Germinating: indexing as a companion to laziness

Half-formed but worth sketching while it's fresh.  Today
`VarAccessor<T> #named` does a linear scan over the stable `[T]`:
for 100 clients it's a non-issue, for 10K or 100K it's the
dominant cost of any name lookup.  An **index** — a hashmap from
key → array offset, or a B-tree for range queries — turns O(N)
into O(1) or O(log N), at the cost of memory + maintenance.

Indexing belongs in the same conceptual layer as the optimiser:
both are model-specific knowledge plugged into a uniform query
runtime.  Laziness defers materialisation; indexing changes the
*shape* of the lookup itself.  Different lever, same goal.

### Where it pays

- **`X named "Y"` on large collections.**  The canonical case.
  100 clients = imperceptible, 10K = ~milliseconds of cycles, 1M
  = a real problem.  Hashmap on `name` → O(1) regardless.
- **Repeated lookups in one query.**  `repeat with c in lst /
  eval (name of c)` today causes one network call per element;
  even with the bridge co-located, each call re-walks the cl
  array.  A per-query index amortises the build cost across
  iterations.
- **Predicate selectivity hints.**  If `country` is indexed, the
  optimiser can pick *which* predicate in a conjunction to push
  through the index first.  `country = "Liechtenstein"` matches
  0 → answer is `#list []` without touching age/income filters.

### Where it doesn't

- **Tiny collections.**  Index overhead (memory + maintenance)
  outweighs the linear-scan savings below some threshold.  The
  bench's 100-client dataset is below it.
- **Range queries on un-ordered indexes.**  Hashmap helps `age =
  37` but not `age >= 45`.  Need a sorted/tree index for the
  latter — different data structure, different cost class.
- **High-mutation workloads.**  Every write to the underlying
  stable variable needs index maintenance.  Read-heavy is the
  sweet spot.

### Companion to the optimiser

This is where the user's intuition lands: the optimiser sees both
the **collection size** (via lingo / accessor metadata) and the
**query shape** (via the spec), and decides whether to wrap a
`VarAccessor<T>` with an **index-generator Smurf** for this query.

```
VarAccessor<T>("clnt", #named, …, getName)
   ↓ (optimiser, when cost model says "lookups by name on
   ↓  large collection, build index")
IndexedSmurf<T>(inner, getName, lazy: true)
   ↓ first lookup forces index build, caches result.
   ↓ subsequent lookups in this query: O(1).
```

Three index lifetimes worth considering:

1. **Per-query** — build on first lookup, drop at end of `eval`.
   Cheapest semantically (no cross-query state, no invalidation).
   Win when one query has many index hits.
2. **Cross-query cached** — build once, reuse across `eval`s.
   Invalidate on stable-variable mutation.  Needs a versioning or
   write-watching mechanism — non-trivial in Motoko, doable.
3. **Eager at canister init** — always present, predictable
   memory cost, no first-call latency spike.  Right when the
   index is small and lookups are constant-traffic.

For first cut, **per-query** is the cleanest: lazily built inside
the resolver's per-call state, no upgrade-stable ceremony, no
cross-call invariants to maintain.

### Codegen and lingo

The same `mo_to_lingo` codegen pass that derives accessor cost
classes can declare `indexable` accessors — those over stable
variables of size ≥ threshold with `#named` or `#test` access
patterns.  Lingo can then carry, per class:

```
indexes : vec record { key : text;  kind : variant { #hash; #tree } }
```

The optimiser reads lingo, sees "client has a hash index available
on `name`", and chooses the index-wrapping path when a query
includes `name = X`.

An explicit annotation surface (developer-side) is the natural
override for the 20% where automatic derivation gets it wrong:
`@indexed("name")` on the accessor declaration.

### Combined with laziness

Lazy collections and indexed lookups compose, but only at certain
shapes.  A few example interactions:

- `count of every client whose country = "Germany"` — lazy
  CollectionSmurf scans the cl array, increments a counter,
  no allocation.  Indexing on `country` would beat this only if
  the result set is much smaller than the full sweep (selectivity
  matters); the optimiser knows because lingo carries cardinality
  hints or the cost model has them.
- `client "Hans Müller" of root` — singular lookup.  Lazy doesn't
  help (no collection to defer); index turns O(N) → O(1).  Pure
  indexing win.
- `every client whose age >= 45 and country = "Germany"` —
  conjunction.  Lazy walks once, evaluates both predicates per
  element.  Index on `country` (if hash) reduces candidate set
  first, age filter applies to ~60 instead of 100.  Lazy +
  indexed = the SQL pattern of "use the most selective
  index-able predicate to drive the scan".
- `every card whose vali = "02/27"` — global card view via
  `FlattenedSmurf`.  Today an index would have to live on the
  flattened parent.children projection, which is more involved
  to maintain.  Probably not the first place to add indexes.

### Critique

- **Memory cost is real.**  A hashmap over 100K text keys is
  meaningful heap.  IC canister heap is large but not infinite,
  and the developer-visible cost is "my canister grew by N MB
  because the query layer decided to index".  Per-query indexes
  bound this (build, use, drop within one update call); cached
  indexes need careful budgeting.
- **Invalidation is hard.**  Cross-query indexes have to know
  when the underlying stable variable changes.  Motoko's
  `stable` keyword doesn't expose mutation notifications; we'd
  need either:
  - convention (developer calls `invalidate_index()` after
    mutating writes — error-prone),
  - wrapper types (`IndexedVar<T>` that intercepts mutations —
    clunky, viral), or
  - rebuild-on-every-query with cheap version check (acceptable
    if the version-check itself is O(1)).
- **Index choice is a dark art.**  SQL DBs ship query planners
  that pick *which* index to use given multiple options.  We
  don't want to ship that complexity in cut 1.  Probably:
  one index per `#named` accessor over a stable variable that
  the developer opts in to.  Multi-index selection comes much
  later, if ever.
- **Test surface explodes.**  Every accessor now has two execution
  paths (linear scan, indexed lookup).  Need property tests that
  they return identical results, and benchmarks that show the
  indexed path is actually faster at the claimed cardinality.

### Triggers for investing

- A bench (or real canister) query is measurably bottlenecked on
  `#named` linear scan.
- The collection in question is ≥ 1K elements (below that, the
  scan is too cheap to bother).
- Read/write ratio favours indexing (≥ 100:1 read:write per
  index, rough rule of thumb).
- Lazy `CollectionSmurf` has shipped — measure indexing *against*
  the lazy baseline, not against today's eager linear scan,
  otherwise we'd be measuring the wrong delta.
- Lingo carries enough per-accessor metadata to drive the
  optimiser's index-choice decision, OR a per-canister manual
  override surface is in place.

### Analog references

- **SQL B-tree / hash indexes.**  Same cost model, same
  maintenance tradeoffs, mature query-planner integration.
- **Object-oriented database indexing** (O2, ObjectStore).  Path-
  expression aware: indexes on `client.cards.number` are
  conceptually possible — closer to our FlattenedSmurf case.
- **Datomic / RDF stores.**  Index everything (EAVT, AEVT, AVET,
  VAET) at write time; trade write cost for read flexibility.
  Probably overkill for us, but worth knowing the extreme exists.
- **GraphQL DataLoader (again).**  Per-query lookup cache — same
  shape as our per-query index but at the resolver-batching layer.

### One-line summary for future-me

Indexing is the *shape-change* lever; laziness is the
*materialisation* lever.  Compose at the optimiser's call site:
optimiser sees collection size + query shape, wraps
`VarAccessor` with an `IndexedSmurf` when the indexed path
dominates.  Per-query lifetime first (no invalidation worries);
cached only when justified.  Lingo carries the `indexable`
metadata so the optimiser's decision is automatic.

---

### `Accessors.mo` library (the long-term shape)

Not yet started. The bench file is a hand-rolled instance of what
`Accessors.compile([entityHook])` would emit for one entity (`Client`).
The shape is converging:

- Per-entity hook supplies: class fourcc, primary-key form, list of
  property accessors, list of class-navigators (indexed/named/test/…).
- `Accessors.compile` emits the `Smurf` record + `Accessor` instances.
- Each property accessor is monomorphic (`func(parentBlob) : Smurf` with
  `from_candid<P>` baked in); `VarAccessor<T>` is the typed-fast-path
  bypass for stable-var-backed entities (parent's blob ignored).
- Pure data → codegen-friendly; library is the only consumer of the
  `Smurf` interface, every entity provider plugs in via hooks.

---

## Library axioms

The object-support library is **data-model-agnostic**.  It knows
nothing about the canister's entities — the sole way it learns
about them is by asking Smurfs via their common interface.

1. **Smurfs self-describe.** A Smurf exposes `class4cc`, `accessors`,
   `toDesc`, `filter`.  Those four fields are the library's entire
   surface.

2. **Accessors own navigation.** `Accessor.lookUp(parent, key)` is
   opaque to the library.  The accessor decides whether the result
   is a scalar entity, a leaf value, or a collection.

3. **Collections are Smurfs.** A `#test`-form accessor's `lookUp`
   returns one Smurf whose `toDesc` renders as `#list`.  Iteration
   lives inside the collection-Smurf — the library never iterates.

4. **Property leaves are Smurfs.** Single-entity property reads
   end at a `ValueSmurf` whose `toDesc` is `#value(...)`.
   Property *projection* across a collection is the `"prop"`
   accessor on a `CollectionSmurf`; it iterates source, calls
   `wrap`, finds each element's named-accessor, and gathers the
   resulting `Smurf`s into a `smurfMap` (see below).

5. **Errors propagate via `notFoundSmurf`** (empty accessors,
   throwing `toDesc`).  Type-level mismatches the library can't
   honour (`#uniqueID`, `#range` today) trap.

### Two collection shapes: 1→many vs many→many

The bench has two kinds of "list of Smurfs", and the distinction
matters because they map to the two AppleScript composition
patterns:

| | `CollectionSmurf<T>` (**1→many**) | `smurfMap` (**many→many**) |
|---|---|---|
| Origin | `actorSmurf.#test` over `[T]` | `CollectionSmurf.#prop/#named` broadcast |
| Backing | `[T]` (raw, typed) | `[Smurf]` (already-wrapped) |
| Parent context | one (the spec's container) | many (one per element) |
| `#test` (filter) | typed predicate over `T` — cheap | none today; would need predicate-over-Smurf |
| Accessor surface | curated + inherited from parent | distributive — union of elements' accessors |
| Composition | predicates AND-compose | functorial map across N contexts |
| AS pattern | `every X whose P` | `every X of every Y` |

`smurfMap(parent, elements)` is the Functor's lifted view: for every
`(fourcc, form)` pair *any* element exposes, it synthesises a
distributive accessor whose `lookUp` forwards the key to each
element's matching accessor and re-wraps the resulting `[Smurf]`
into another `smurfMap` — recursive `fmap`.  That's what makes
`third character of name of every client` compose: each
navigation step lifts through.

The typed-fast-path naturally lives in `CollectionSmurf` and not
in `smurfMap` because the broadcast wraps each element via
`wrap : T -> Smurf` — the raw `[T]` is *lost* the moment we cross
that boundary.  Predicates over a `smurfMap` would have to
structurally navigate each element (heavyweight) instead of
poking the raw value directly.  We defer that.

The library reduces to:

- `formOfKey : KeyForm → AccessorForm` — pure
- `lookupOfKey : KeyForm → ?LookupKey` — pure
- `resolve(spec, root) : Smurf` — walks the spec, asks each parent
  for the matching accessor, delegates `lookUp`
- `eval(spec, root) : ObjectSpec` — resolve then `target.toDesc()`

No predicate evaluation, no schema awareness, no entity catalog in
the library.  Per-entity hooks (`clientSmurf`, root `VarAccessor`s,
the `#test` clnt accessor on the root) wire all the typed bits.

### `Smurf*` — parallel async interface (future)

Today's `Smurf` is synchronous.  Once a Smurf needs to make an
IC call (cross-canister federation, certified-data lookup,
inter-actor query forwarding), introduce a parallel `Smurf*` whose
methods return `async*`:

```motoko
type Smurf* = {
  class4cc  : Text;
  accessors : [Accessor*];
  toDesc    : ()       -> async* ObjectSpec;
  filter    : BoolExpr -> async* Smurf*;
};
```

The library gains a parallel `resolve*`/`eval*`.  Synchronous Smurfs
lift trivially.  Async-native Smurfs (one that does `await
someCanister.get(id)` inside `lookUp`) plug in directly.

M0033 currently rejects `async* Smurf` because Smurf's function
fields aren't shared; the `Smurf*` form sidesteps this by making
the async boundary explicit on each method.

---

## Apple Events Compact Binary Encoding

For interoperability with macOS AppleScript/Automator (Use Case 7), the
`ObjectSpec` type maps directly onto Apple Event **object specifiers**
(`typeObjectSpecifier` = `'obj '`).

### AEDesc wire format (`AEFlattenDesc`)

Every descriptor is laid out as:

```
OSType   type     (4 bytes, big-endian)   — e.g. 'obj ', 'reco', 'list', 'null'
UInt32   length   (4 bytes, big-endian)   — byte length of data that follows
UInt8    data[length]                     — padded to 2-byte boundary
```

A **record** (`typeAERecord` = `'reco'`) prepends a 4-byte item count, then
for each key-value pair: a 4-byte OSType keyword followed by a nested
descriptor.

A **list** (`typeAEList` = `'list'`) similarly: 4-byte count, then each
element as a nested descriptor (no keyword).

### Object specifier layout

An object specifier is a `'reco'` with exactly four keys:

| Key (`OSType`) | Motoko field | Notes |
|---|---|---|
| `'want'` | `class_` | `typeType` descriptor; e.g. `'cwin'`, `'docu'`, `'prop'` |
| `'form'` | key form | `typeEnumerated`; `'indx'` / `'name'` / `'ID  '` / `'prop'` / `'rang'` / `'test'` |
| `'seld'` | key data | type depends on form: `typeSInt32`, `typeUnicodeText`, etc. |
| `'from'` | `container` | nested object specifier, or `typeNull` for application root |

### KeyForm → AE enum mapping

| Motoko `KeyForm` | AE form enum | `'seld'` type |
|---|---|---|
| `#absolutePosition n` | `'indx'` | `typeSInt32` (negative = from end) |
| `#name t` | `'name'` | `typeUnicodeText` |
| `#uniqueID n` | `'ID  '` | `typeSInt32` or `typeUInt32` |
| `#property p` | `'prop'` | `typeType` (4-char OSType or mapped string) |
| `#range (s1, s2)` | `'rang'` | `typeRangeDescriptor` record (`'star'`/`'stop'`) |
| `#test expr` | `'test'` | `typeCompDescriptor` / `typeLogicalDescriptor` |

### Open-source reference

`py-appscript` (`aem` submodule) contains the most readable implementation of
the full encoder/decoder. The Carbon headers `AERegistry.h` and `AEObjects.h`
define all OSType constants.

### Spec-compliance probe (`nix run .#ae-decoder`)

The canister's `writeDesc` is reverse-engineered from the format observed
empirically (Apple's documentation is incomplete, and `AEFlattenDesc` adds
list-prefix bytes not derivable from the public headers). To keep us honest,
we ship a parallel **decoder** built on the same py-appscript stack:

- `nix/ae-decoder.nix` exposes `nix run .#ae-decoder` (Darwin-only).
- Reads raw binary from stdin (pair with `xxd -r -p` to convert hex blobs).
- Calls `aem.ae.unflattendesc(bytes)` — the same path Apple's CoreServices
  invokes when an AppleScript reply lands. If our wire bytes are spec-
  compliant, this succeeds; if not, it throws.
- Walks the resulting `AEDesc` tree via `getitem(i, '****')` (lists) /
  `getparam(k, '****')` (records and obj specifiers) and pretty-prints
  the structure.

The probe runs against every non-Candid `Reply: 0x646c6532...` line in
`test/bench/ok/object-spec.drun-run.ok`. As of 2026-05-20, **24/24
decode cleanly** — covering:

- `obj ` (typeObjectSpecifier) — clnt/card classes, `#name` and compound
  `#test` keys, nested up to 3 levels (predicate AND of two compares).
- `list` (typeAEList) — empty, of `long`, of `utxt`, of `obj `.
- `utxt` (typeUnicodeText) — including BMP codepoints (Müller).
- `long` (typeSInt32) — element counts and signed integers.
- `tru ` (typeTrue) — boolean leaves.
- `null` — embedded as `from` in chain-anchored obj specs.

Usage:

```bash
sed -n '5p' test/bench/ok/object-spec.drun-run.ok |
  sed 's/.*0x//' | xxd -r -p |
  nix run .#ae-decoder
```

Yields:

```
=== stdin (102 bytes) ===
'obj ' record
  ['want'] -> 'type' leaf, data='clnt'
  ['form'] -> 'enum' leaf, data='name'
  ['seld'] -> 'utxt' leaf, data='Hans Müller'
  ['from'] -> 'null' leaf, data=''
```

Add new shapes to the bench, regenerate the `.ok`, and re-probe — any
wire-format drift surfaces immediately as an `AEUnflattenDescFromBytes`
error rather than as a downstream Script-Editor / ICmator failure
hours later.

### Lingo — canister self-description

A query the canister advertises (e.g. `__lingo : () -> (Lingo) query`)
describing its entity relationships: class 4ccs, property tables per
class, the predicate operators it supports, the forms (`#name` /
`#absolutePosition` / `#test` / …) it recognises for each class.  Lives
on the Candid wire because the lingo schema is *stable metadata about
the OSL surface*, not an `ObjectSpec` value — the public-Candid hard
rule from `.claude/plans/ic-mator.md` doesn't apply.

For the first cut: **entity relationships only.  No verbs, no method
metadata, no capability flags.**  Just enough for a remote consumer
(ICmator's Rust agent, in our case) to know which classes the canister
serves, how they nest, and what properties hang off each.  Verbs and
methods come later, once the entity description is settled.

The Lingo type is the natural output of an `mo_to_lingo.ml`-style
codegen pass — same machinery `mo_to_idl.ml` already uses to derive
`.did` from Motoko types, but reading off `actorSmurf.accessors` (and
nested `*Smurf.accessors`) rather than the Motoko type graph.  A
canister never advertises an accessor it can't serve, and never omits
one it actually has.

### Versioning — deferred

Open question worth flagging now even though we won't act on it:
should the lingo type be a flat record or a `variant { #v1 : LingoV1;
#v2 : LingoV2 }` envelope?  The latter lets the schema evolve
without breaking older clients that have cached lingo across canister
upgrades.  We'll add the envelope when we have a concrete shape
change to absorb; for the first version it's overhead with no
payoff.  This note is the placeholder.

---

## Parenthetical Codec Annotations (proposed Motoko syntax)

To bridge `ObjectSpec` to external encodings (AE binary, JSON, CBOR) at canister
boundaries without changing the Candid IDL, we propose a **parenthetical**
annotation on `public` functions:

```motoko
(with encoder = fromObjectSpecifier; decoder = toObjectSpecifier)
public func foo(spec : ObjectSpec) : async ObjectSpec { spec };
```

The parenthetical `(with encoder = …; decoder = …)` is a **per-function codec
hint**: before the argument is decoded from the wire format, `decoder` is
applied; the return value is passed through `encoder` before it hits the wire.
The Candid signature is unchanged — `ObjectSpec` is still the declared type.

### Semantics sketch

```
decoder : WireBlob -> ObjectSpec    // wire → Motoko (ingress)
encoder : ObjectSpec -> WireBlob    // Motoko → wire (egress)
```

Where `WireBlob` could be:
- `Blob` carrying the AE compact binary (for direct AE integration)
- `Text` carrying JSON (for HTTP / LLM clients)
- Absent (identity) — default Candid behaviour unchanged

This makes the wire format *per-endpoint* rather than *per-type*, so a canister
can expose the same object model over Candid (to other canisters), AE binary
(to macOS), and JSON (to LLMs) on three separate endpoints without duplicating
the resolver logic.

### Interaction with the AE binary format

```motoko
(with decoder = AE.decodeObjectSpec; encoder = AE.encodeObjectSpec)
public func queryAE(spec : ObjectSpec) : async ObjectSpec {
  resolve(spec)
};
```

`AE.decodeObjectSpec : Blob -> ObjectSpec` parses the flattened AEDesc bytes
(see above) into the Motoko variant tree. `AE.encodeObjectSpec` serialises it
back. Both are pure Motoko; no RTS changes required.

### Open design questions for parentheticals

- Are encoder/decoder applied at the Motoko layer (before/after Candid
  decode/encode) or injected into the IDL stub?
- Should the syntax be a proper Motoko attribute (`@codec(…)`) rather than a
  parenthetical, to fit the existing decoration model?
- How does this interact with `shared` functions and cross-canister calls —
  does the annotation propagate to the caller's stub?
- Can a single parenthetical annotate multiple arguments?
  ```motoko
  (with decoder = {spec = AE.decode; token = Token.decode})
  public func foo(spec : ObjectSpec, token : Token) : async () { … };
  ```

---

## Open Questions

- Should `Token` be a typed variant (closed world) or an opaque `Blob`
  (open, but loses type safety at library boundary)?
- Should the specifier grammar be versioned (to allow extension without
  breaking existing certified proofs)?
- For `formTest` over large collections: lazy iteration vs. batch-limit
  parameter?
- Should the HTTP endpoint speak JSON, CBOR, or AE compact binary? The
  parenthetical codec annotation makes all three viable simultaneously — each
  endpoint gets its own `(with decoder = …; encoder = …)` without duplicating
  resolver logic.

---

## Aggregation in `BoolExpr` — `#countOf` extension plan

**Status:** shelved until after the 2026-05-22 demo. The headline query
on slide 5 (`every client whose count of (every card whose valid is
false) > 1`) is shown as aspirational. AppleScript already parses it
end-to-end; the canister-side decoder is what needs to extend.

### Why shelve

- The talk's job is to *water mouths*, not deliver a feature. The
  inner half (count of invalid cards = 6) already works in
  fixture `06-invalid-cards.applescript` and demonstrates the
  primitive; the outer half can land later without affecting
  anything else.
- Risk is low *structurally* but non-trivial *protocol-wise* (one
  new mutual-recursion edge between two evaluators — see "Tricky
  bit" below). Worth doing deliberately, not under demo-day pressure.

### Proposed extension

```motoko
type BoolExpr = {
  #compare : { prop : Text;  op : Comparison;  value : CandidValue };
  #countOf : { sub : ObjectSpec;  op : Comparison;  value : Int };   // ← new
  #and_    : (BoolExpr, BoolExpr);
  #or_     : (BoolExpr, BoolExpr);
  #not_    : BoolExpr;
  #always;
};
```

Semantics: when evaluating `#countOf { sub; op; value }` against an
iterated outer element `e`, run `eval(sub, e)` (i.e. with `e` as the
container for `sub`), interpret the result as a collection, take its
cardinality, compare against `value` with `op`. The whose-clause keeps
or drops `e` based on the comparison.

### Risk dimensions

| Surface | Risk | Why |
|---|---|---|
| Type-level | low | Pure additive variant; we own both producer + consumer. |
| Wire (AE format) | low | AppleScript already packs nested object specifiers in whose-clause comparisons. Format unchanged. |
| Smurf protocol | low–medium | `Smurf.filter : BoolExpr → Smurf` gets one new case in its dispatcher; the protocol shape doesn't change. |
| Cost model | medium | A whose-clause goes from `O(n × constant)` to `O(n × cost(sub))`. For 100 clients × ~2 cards each it's negligible; on bigger DBs deserves a depth cap and a cycles-aware short-circuit. |
| Contextual scoping | **medium** | See below. |
| Failure semantics | low | `notFoundSmurf` policy already exists; the new edge needs to compose cleanly with it. |

### The tricky bit — contextual scoping

Current eval signature:

```motoko
eval(spec : ObjectSpec, root : Smurf) : async* …
```

Today, BoolExpr's predicates are **flat**: `#compare { prop; op; value }`
only reads one property of the iterated element against a constant.
There is no path from `BoolExpr` eval back to `ObjectSpec` eval.

`#countOf { sub; … }` needs `sub` evaluated **relative to the iterated
outer element**, not root. So the eval path now contains a mutual
recursion edge:

```
ObjectSpec.eval ──→ Smurf.filter ──→ BoolExpr.eval ──┐
        ▲                                            │
        └──────────── #countOf re-enters here ───────┘
```

Mechanically that's one extra parameter on the BoolExpr evaluator (the
current iterated Smurf, to be passed as container of `sub`), but the
trap-policy across that edge needs deliberate thought: when `sub` hits
`notFoundSmurf` mid-walk, does the outer whose-clause:

1. **Skip** the outer element silently (treat count as 0), or
2. **Trap** the entire outer eval (current style for hard misses), or
3. **Surface** as a partial result with a per-element error?

Recommend (1) for `#countOf` specifically: a missing inner specifier
genuinely means "this outer element has 0 matching inner elements".
This mirrors AppleScript host semantics — `count of (its things whose
…)` returns 0 for elements with no `things`.

### Implementation steps

1. Extend `BoolExpr` with the `#countOf` variant. Compiler warnings will
   flag every pattern match; chase them one by one.
2. Extend the predicate evaluator (`evalBool` or wherever the
   per-element predicate lives) to accept the iterated Smurf and to
   call into `eval(sub, currentSmurf)` for `#countOf`.
3. Compute cardinality on the returned ObjectSpec — for `#list n` it's
   `n.size()`; for `#obj { … }` it's 1 (singleton); for everything else
   trap with a clear "cannot count this descriptor" message.
4. Wire the decoder: incoming AE comparisons whose left side is a
   nested object specifier need to project to `#countOf` (default — AE
   doesn't distinguish between "evaluate as count" and "evaluate as
   collection-of-items"; the comparison operator's right-hand integer
   is the heuristic).
5. Add fixtures:
   - `07-clients-multi-invalid.applescript` — the headline query.
   - `08-clients-no-invalid.applescript` — outer with count == 0.
   - `09-clients-and-clause.applescript` — `#countOf` inside `#and_`
     ("Germans with > 1 invalid card").
6. Documentation: extend the eval-policy doc with the trap-policy
   decision and the depth cap (suggest: 4 nesting levels max, trap
   beyond).

### Open questions

- Should the `value` field of `#countOf` be `Int` or `Nat`? AppleScript
  permits "count > -1" (always true), so `Int` is more honest, but `Nat`
  catches an off-by-one class of bugs at the type level. Lean toward
  `Int` for AE-fidelity.
- Do we want to also lift `count of` *outside* a whose-clause as a
  result type (i.e. `every client whose ...` returning `count of` rather
  than a list)? This is a separate axis — the `pcnt` accessor already
  handles it for non-nested cases.
- Cost-limit policy: hard cap at depth 4, or cycles-aware short-circuit
  via `Cycles.balance()` checks? Tend toward depth cap for simplicity;
  the canister isn't likely to be the cycles-tight component in
  practice.

### Cross-references

- `~/motoko/test/bench/object-spec.mo` — current `BoolExpr` + `eval`
  implementation. Search for `#compare` to find pattern-match sites.
- `~/ICmator/agent/src/lingo.rs` — agent-side AE encoding /
  decoding. The whose-clause packer is in the surrounding modules.
- `~/ICmator/agent/src/main.rs` — agent's stdin→canister-call→stdout
  glue. The reject-bubble path is the call-site for the AE-error
  structured-payload work below.
- `~/ICmator/slides/index.html` slide 5 — "Where we're heading" —
  audience-facing version of this plan.
- `~/ICmator/tests/fixtures/06-invalid-cards.applescript` — the
  inner half of the headline query (works today; passes `make check`
  once the stack is up).

---

## AE error codes — from stringly-typed reject to structured payload

**Status:** companion to the `#countOf` extension; not currently
wired even though the canister already emits the literal string.

### The current shape

In `object-spec.mo`, the 404 path is:

```motoko
let notFoundSmurf(parent : Smurf) : Smurf = {
  …
  toDesc = func() : async* ObjectSpec {
    throw error ("Error (errAENoSuchObject = -1728) in "
                  # debug_show (await* parent.toDesc()))
  };
  …
};
```

The canister `throw error <text>` lands on the wire as a Candid reject
response with the text string in the body.  `icmator-agent` today
runs `agent.update(...).call_and_wait().await?` and `?`-bubbles the
`ic_agent::AgentError::ReplicaError` (or similar) up as an
`anyhow::Error` straight to stderr.  Nothing pattern-matches the
string.  Script Editor sees a generic agent-failed condition rather
than a typed AppleEvent error.

### The slide-deck stopgap

What the slide currently labels "404 path" is *aspirational on the
host side*.  The canister puts a stable literal in the reject text
(`errAENoSuchObject = -1728`) precisely so an agent-side parser has
something deterministic to grep — but the parser doesn't yet exist.

### Three plausible designs

| Option | Canister change | Wire shape | Cleanliness |
|---|---|---|---|
| **A. Regex on reject text** | none | unchanged | works, but stringly-typed; brittle to canister-side rewording |
| **B. JSON-tagged reject text** | swap `error "…"` for `error "AE:" # JSON.encode {code; ctx}` | unchanged | parseable prefix, but still stringly-typed |
| **C. Variant return type** | replace `async ObjectSpec` with `async { #ok : ObjectSpec; #ae_error : { code : Int; context : ?Text } }` | new variant on the wire | clean, typed, but breaks the success-only signature and every existing caller |

### Recommended path

**Option C, gated behind a single per-method codec.**  The encoder
already lives behind the `(with encoder = …)` parenthetical, so we
can attach a *result-codec* that handles `#ok` vs `#ae_error`
without touching method shape downstream:

```motoko
type AEResult = {
  #ok       : ObjectSpec;
  #ae_error : { code : Int; context : ?Text };
};

(with encoder = AE.encodeResult; decoder = AE.decodeObjectSpec)
public func queryAE(spec : ObjectSpec) : async AEResult { … };
```

`AE.encodeResult` flattens `#ok` to the existing AE descriptor and
`#ae_error` to a `keyErrorNumber + keyErrorString` AE reply.  The
agent then has two clean paths:

```rust
match decode_ae_result(&response_blob) {
  AEResult::Ok(spec_bytes)         => write_stdout(spec_bytes),
  AEResult::Error { code, context } => write_stdout(pack_ae_error(code, context)),
}
```

— no regex, no stringly-typed coupling between canister and agent.

### Migration order

1. Land `#ae_error` as an additional variant alongside `#ok` (no
   field rename), keep both throw-path and return-path live for one
   release.
2. Switch `notFoundSmurf.toDesc` from `throw error` to a
   `return #ae_error { code = -1728; context = ?... }` over a
   bench-only flag.
3. Once the bench is green end-to-end with the typed path, retire
   the throw-text path entirely.  `query.md` and the slide get
   updated to "canonical typed path; no regex".

### Open questions

- Should the canister carry an enum of known AE codes
  (`errAENoSuchObject`, `errAEEventNotHandled`, …) or just raw
  `Int`?  AE has ~50 codes; an enum sweetens the canister code but
  adds a churn-prone surface.  Lean toward raw `Int` for
  AE-fidelity (consistent with the `#countOf.value` decision above).
- Should `#ae_error` carry a partial result?  Some queries are
  partly resolvable (e.g. `every X whose …` where one `X` is
  notFound but the rest are fine).  Probably out of scope for v1 —
  treat 404 as fatal for the whole spec.
- Does the AE error code surface in `BoolExpr.#countOf` failure
  modes too?  Per the trap-policy decision above, a `notFound` mid-
  walk inside `#countOf` skips the outer element (counts as 0) and
  does NOT raise an AE error.  So the two extensions stay
  independent.
- **`#contains` with object-specifier list items** (open issue).
  `BoolExpr.#contains.values : [CandidValue]` only handles scalar
  literals.  When AppleScript builds a `whose` list from property
  references — e.g.
  `{ country of client 1, country of client 2 } contains country`
  — the list items arrive on the wire as `OBJ`-typed AE descriptors,
  not scalars.  `parseInListBody` currently traps on these with
  "AE: unsupported value type".
  Resolution: change `values` to `[ObjectSpec]` (or a union
  `CandidValue | ObjectSpec`) and resolve each specifier via `eval`
  before comparing.  Because `eval` is `async*` and needs the
  `actorSmurf`, `parseInListBody` (or the `#contains` arm of
  `evalBoolExpr`) must become `async*` with the Smurf root threaded
  through.  The payoff: `whose X in {Y of A, Y of B}` style
  cross-object comparisons become expressible.
