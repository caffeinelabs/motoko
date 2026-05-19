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

macOS Automator and AppleScript speak Apple Events whose object references map
directly onto `ObjectSpec`. The dream syntax:

```applescript
tell application "IC Bridge"
    get balance of account id 42 of canister "bla-gurr-xump" of network "yurrt-pomjj-uer"
end tell
```

**How AppleScript addressing works:** the AE target (`keyAddressAttr`) must be
a process — `typeApplicationURL` (`eppc://`), `typeProcessSerialNumber`, or
`typeKernelProcessID`. Script Editor uses Bonjour/mDNS to discover remote
machines, but only those running macOS Remote Apple Events; there is no
open-ended extension point for custom address types. `canister "x" of network
"y"` as a bare root (without `tell application`) is not natively supported.

**Practical path — intermediary scriptable app/daemon:**
A background process ("IC Bridge", launchd-managed, no UI) with an **SDEF**
that declares `network` containing `canister` as first-class AE object classes.
AppleScript resolves `canister "x" of network "y"` as an AE object specifier
tree inside the IC Bridge process; IC Bridge translates it to a canister query
and returns results. The AEOM hierarchy maps isomorphically onto `ObjectSpec`:

```
tell application "IC Bridge"             →  objectAt(spec)  on  canister
    get balance of account id 42 of …       resolved by IC Bridge → HTTP/Candid
end tell
```

- No custom glue code per canister — any canister implementing `Queryable`
  appears automatically in IC Bridge's object model once registered
- Automator action: "Query Canister Object" wraps the same AE call
- The AE binary encoding (see above) is what IC Bridge sends over the wire;
  no JSON translation needed for the macOS side

**Modern alternative — App Intents (macOS 13+):** for Shortcuts/Spotlight
integration, App Intents are discoverable, require no SDEF, and are
installable by third parties. A "Query IC Canister" Intent accepts a canister
ID, network ID, and `ObjectSpec`-derived parameters and returns typed results.
Different model from AppleScript but better suited for Siri/Spotlight/Shortcuts.

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

1. **Smurfs self-describe.** A Smurf exposes `accessors`, `class4cc`,
   `toDesc`, `isNotFound`, `filter` (and `blob` for Candid-backed
   children).  Those six fields are the library's entire surface.

2. **Accessors own navigation.** `Accessor.lookUp(parent, key)` is
   opaque to the library.  The accessor decides whether the result
   is a scalar entity, a leaf value, or a collection.

3. **Collections are Smurfs.** A `#test`-form accessor's `lookUp`
   returns one Smurf whose `toDesc` renders as `#list`.  Iteration
   lives inside the collection-Smurf — the library never iterates.

4. **Property leaves are Smurfs.** Property projection across a
   collection is a `"prop"` accessor on the collection-Smurf; given
   `#named propName` it iterates source, calls `wrap`, finds the
   element's named-accessor, gathers the resulting `ValueSmurf`s
   into a `listSmurf`.  Single-entity property reads end at a
   `ValueSmurf` whose `toDesc` is `#value(...)`.

5. **Errors propagate via `notFoundSmurf`** (`isNotFound = true`,
   `toDesc = #root`).  Type-level mismatches the library can't
   honour (`#uniqueID`, `#range` today) trap.

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
  blob       : ()       -> async* Blob;
  class4cc   : Text;
  accessors  : [Accessor*];
  toDesc     : ()       -> async* ObjectSpec;
  filter     : BoolExpr -> async* Smurf*;
  isNotFound : Bool;
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
