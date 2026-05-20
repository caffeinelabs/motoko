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
