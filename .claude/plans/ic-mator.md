# ICmator — macOS AppleScript ↔ IC bridge daemon

A scriptable macOS agent process that translates inbound Apple Events
(from Script Editor, Automator, Shortcuts-via-AS, etc.) into IC ingress
calls against a canister that speaks the AE binary format on its wire.

Companion plan to [`query.md`](./query.md), which defines the
canister-side `ObjectSpec` / Smurf protocol.  ICmator is the macOS
front-end: it owns the SDEF, the AE handlers, and the IC transport.

The clean design property: because the canister's `(with encoder;
decoder)`-annotated methods already speak AE wire bytes natively
(see `test/bench/object-spec.mo`'s `go` method and the `parseObjBody`/
`writeObjBody` codec), the bridge is a **pure byte pass-through** —
no JSON, no Candid-on-macOS, no schema duplication.  AppleScript →
AE bytes → Motoko canister → AE bytes → AppleScript, the same bytes
the whole way.

## Demo target

Friday 2026-05-22, 13:30.

Smallest end-to-end pitch:

```applescript
tell application "ICmator"
    get every client whose country = "Germany"
end tell
```

Result is a list of 60 client object specifiers visible in Script
Editor's result pane.  Also supports `count`, `first`, and the
property/filter shapes the bench's `tinyN` methods already exercise.

## AppleScript addressability — the hard constraint

The AE target (`keyAddressAttr`) must be a **process** —
`typeApplicationURL` (`eppc://`), `typeProcessSerialNumber`, or
`typeKernelProcessID`.  Script Editor uses Bonjour/mDNS to discover
remote machines, but only those running macOS Remote Apple Events;
there is no open-ended extension point for custom address types.

`canister "x" of network "y"` as a bare root (without `tell
application`) is **not natively supported**.  Hence ICmator: a
locally-registered application that holds AE object specifiers
internally and forwards them.

## Architecture

```
┌─────────────────────┐    AE descriptor (raw bytes)   ┌──────────────────┐
│ Script Editor /     │ ─────────────────────────────► │ ICmator.app      │
│ Automator /         │                                 │ (Swift agent)    │
│ Shortcuts / etc.    │ ◄───────────────────────────── │                  │
└─────────────────────┘    AE-encoded reply             └────────┬─────────┘
                                                                 │ stdin/stdout
                                                       ┌─────────▼─────────┐
                                                       │ icmator-agent     │
                                                       │ (Rust subprocess) │
                                                       └─────────┬─────────┘
                                                                 │ ic-agent
                                                                 │ HTTPS+CBOR
                                                       ┌─────────▼─────────┐
                                                       │ Canister `go`     │
                                                       │ (object-spec.mo)  │
                                                       └───────────────────┘
```

## Process model

- **Single `.app` bundle** (`ICmator.app`), `LSUIElement = YES` — no
  Dock icon, no menu bar, no UI.
- **LaunchAgent** at `~/Library/LaunchAgents/ai.caffeine.icmator.plist`,
  `KeepAlive = true`, `RunAtLoad = true`, points to the .app's
  executable inside the bundle.
- **Even though headless, must be a registered .app**, not a true
  `/Library/LaunchDaemons/` daemon.  LaunchDaemons run before
  WindowServer/AE plumbing exists and can't receive Apple Events.

## Bundle setup

`ICmator.app/Contents/Info.plist`:
- `NSAppleScriptEnabled = YES` — opt-in to AppleScript dispatch.
- `OSAScriptingDefinition = ICmator.sdef` — points at the SDEF in
  `Resources/`.
- `LSUIElement = YES` — agent, not a regular app.
- `CFBundleIdentifier = ai.caffeine.icmator`
- `CFBundleExecutable = ICmator` (the Swift binary)

`ICmator.app/Contents/Resources/ICmator.sdef` (XML):
- Declares `network` / `canister` / and the per-canister classes
  (start with `client` from the bench: `name`, `country`, `age`,
  `yearlyIncome`).
- Properties carry the same 4ccs the canister uses (`name`, `cntr`,
  `age `, `inco`).
- Element-of relationships drive `every X of Y` resolution.

## Event handlers

At launch (in `applicationDidFinishLaunching` or main directly):

```swift
let mgr = NSAppleEventManager.shared()
mgr.setEventHandler(self,
    andSelector: #selector(handleGet(_:withReplyEvent:)),
    forEventClass: kCoreEventClass, andEventID: kAEGetData)
mgr.setEventHandler(self,
    andSelector: #selector(handleCount(_:withReplyEvent:)),
    forEventClass: kCoreEventClass, andEventID: kAECountElements)
```

Optionally `kAESetData` for setters (not needed for the demo).

## Inbound event handling

1. Pull the descriptor for `keyDirectObject` from the event — it's an
   `NSAppleEventDescriptor` wrapping the ObjectSpec.
2. **Flatten via `AEFlattenDesc`** (or `descriptor.aeDesc.data` after
   `AECoerceDesc(typeWildCard)`) — that gives the exact bytes that
   the canister's `parseTopLevel` already understands.  No transcoding.
3. Write the bytes to `icmator-agent`'s stdin.
4. Read response bytes from `icmator-agent`'s stdout.
5. Wrap as `NSAppleEventDescriptor(descriptorType: typeAEList, data:
   replyBytes)` (or `typeObjectSpecifier`, etc. — depends on what the
   canister returned).
6. `replyEvent.setDescriptor(resultDesc, forKeyword: keyDirectObject)`.

## IC transport — `icmator-agent`

A small Rust binary (~50–80 lines):

```rust
use ic_agent::{Agent, export::Principal};
use std::io::{Read, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("IC_URL").unwrap_or("http://127.0.0.1:4943".into());
    let canister = std::env::var("CANISTER_ID")?;
    let method = std::env::args().nth(1).expect("method name");

    let agent = Agent::builder().with_url(url).build()?;
    agent.fetch_root_key().await?;             // local replica only

    let mut args = Vec::new();
    std::io::stdin().read_to_end(&mut args)?;

    let response = agent
        .query(&Principal::from_text(canister)?, method)
        .with_arg(args)
        .call().await?;

    std::io::stdout().write_all(&response)?;
    Ok(())
}
```

Swift spawns this per AE event via `Process` + `Pipe`.  Fork cost ~5–15ms.

### Why not other transports

- **`dfx canister call`**: 200–500ms per invocation (dfx-config read,
  network open).  Demo-killer.
- **`dfx canister sign` + replay**: same per-call cost.  Replay not
  possible anyway — each IC request envelope carries `ingress_expiry`
  + nonce; the signature is bound to those.
- **Native Swift signing**: doable (CryptoKit Ed25519, SwiftCBOR,
  URLSession), ~250–400 lines.  Saves the subprocess.  Defer to
  post-demo cleanup — too much surface area for Friday.
- **`agent-js` in JavaScriptCore**: modern `@dfinity/agent-js` is ESM
  with Node-only crypto imports; getting it running in JSC is more
  work than the Rust subprocess.

## Installation

```bash
make app                    # assembles ICmator.app into ./build
make agent                  # builds icmator-agent (cargo)
make install                # copies .app to ~/Applications/, registers LaunchAgent
launchctl load ~/Library/LaunchAgents/ai.caffeine.icmator.plist
```

Demo verification:
1. `open -a "Script Editor"`, File → Open Dictionary → ICmator.  The
   scripting suite shows with `client`, `every`, `whose`, properties.
2. Type the dream query, hit Run.  Result pane shows the AE-decoded list.

## Milestone: `script-editor-works` (2026-05-20)

What lit up:

```applescript
tell application "ICmator"
    eval (every client whose country = "Germany")
end tell
```

Script Editor renders the result pane as

```
{client "Hans Müller" of application "ICmator",
 client "Anna Müller" of application "ICmator",
 client "Otto Müller" of application "ICmator",
 …}
```

— a native AppleScript list of `typeObjectSpecifier`s, *not* a hex
string or a `«data ...»` opaque blob.  Each entry is a real specifier
that *could* be the target of a follow-on `eval`/`get` once the
property-lookup leg is wired.

Three things had to be true for this:

1. **`eval` is a custom verb** (event class `'ICma'`, event id
   `'eval'`) — bypasses Cocoa scripting's class-element-resolution
   that ate the standard `get every` syntax with a -1728 before our
   handler ran.  `query "..."` stays as a parallel probe.
2. **Inbound flatten** is the identity — `NSAppleEventDescriptor.data`
   already returns the full wire form (dle2 + type + length + body).
   Empirical only; documented nowhere clear.
3. **Outbound wire is Apple-canonical** — the canister's
   `writeDesc` for `#list` was missing 24 bytes of list-prefix that
   Apple's `AEFlattenDesc` emits (8 alignment + 8 sub-header
   `[0x18 + 'list']` + 8 count+pad).  Patched (motoko commit
   `81c20e33a`); now `AEUnflattenDescFromBytes` accepts the bytes
   and returns a proper `typeAEList` descriptor.

Probe sandbox at [`probe/flatten-probe.swift`](https://github.com/ggreif/ic-mator/blob/main/probe/flatten-probe.swift)
generates Apple's reference output for any descriptor shape — keep
it for future format diffs (records, deeper nested objs, etc.).

## Milestone: `references-reusable` (2026-05-20)

The composability follow-on is now live.  This script returns 60
real client names, every step going through the bridge:

```applescript
tell application "ICmator"
    set lst to eval (every client whose country = "Germany")
    set out to {}
    repeat with c in lst
        set end of out to eval (name of c)
    end repeat
    out
end tell
```

Result: `{"Hans Müller", "Anna Müller", "Otto Müller", …, "Ingrid Koch"}`
— 60 entries, umlauts rendering correctly, every name fetched by
the canister resolving the AS-side reference `c`.

Five things had to come together:

1. **UTF-16 BMP** (canister) — `textToUtf16` / `utxtToText` were
   ASCII-only (stuffed UTF-8 bytes into low UTF-16 halves).  Real
   UTF-16 BE now; "Müller" no longer renders as "MÃ¼ller".  Non-BMP
   surrogates still trap.
2. **`parseObjBody` learns `formName`** (canister) — Apple's
   `formName` (4cc `'name'`) used to be undecodable; AS-sent
   `name of c` referencing a previously returned `client "X"`
   trapped at parse time.
3. **AS-property resolve fallback** (canister) — AS sends
   `class_ = "prop"` + `key = #property X` for property access;
   the bench's singleton convention (tiny2/tiny8) uses `class_ = X`.
   `resolve()` now falls back to `findAccessor(parent, X, #named)`
   when literal `"prop"` is unknown.  Collection-broadcast path
   (where a real `"prop"` accessor exists) is unaffected.
4. **`handleGet` wired through** (app) — `forward()` no longer
   short-circuits to the fixed `every card` blob.  It pulls
   `keyDirectObject`, flattens, forwards.  (Cocoa scripting still
   intercepts the standard `get` verb before us, which is why the
   demo wraps property access in `eval (name of c)` — see point 5.)
5. **SDEF `eval` result type `data` → `any`** (app) — we return a
   `typeAEList` descriptor (via `AEUnflattenDescFromBytes`), not raw
   bytes.  Declaring `data` made AS coerce/reject and fall back to
   resolving the source specifier, which Cocoa can't do.

Tagged `references-reusable` on `ggreif/ic-mator` (commit `609ca50`),
motoko side at commit `1a5b8d575` on `gabor/encoder`.

### Known shapes that still don't compose

- `name of <list>` — AS doesn't broadcast `name` over a list value.
  Must iterate via `repeat with c in lst / eval (name of c)`.  The
  batched form `eval (name of every client whose country = "Germany")`
  works too and is faster (1 AE event, 1 IC call).
- Properties whose 4cc is not in the SDEF for `client` (e.g.
  `nameOnCard`) — AS compiles but fails before iteration.

## Milestone: multi-canister addressing (2026-05-20)

Shipped on ICmator `main` (commit `5be9389`).  Two shapes work today:

```applescript
-- per-call
tell application "ICmator"
  tell canister "t63gs-up777-77776-aaaba-cai"
    eval (every client whose country = "Germany")
  end tell
end tell

-- session default
tell application "ICmator" to target canister "t63gs-…-cai"
tell application "ICmator" to eval (every client)   -- uses default
```

Implementation notes (deviations from the original design sketch):

- **No `set defaultTarget to …` property.** Cocoa scripting intercepts
  `set X to Y` on application properties before our `setEventHandler`
  fires, regardless of whether the SDEF declares the property as
  `text` or `canister`-typed.  Shipped a custom `target` verb
  instead (4cc `ICma`/`trgt`).  AS-idiomatic enough:
  `to target canister "X"` reads naturally and extends cleanly to
  `to target canister "X" of network "Y"` once the `network` class
  lands — same verb, AS layers the `of`-qualifier into the obj-spec's
  `from` chain, our handler peels both anchors in one walk.
- **No `keySubjectAttr`.** AS doesn't put `tell ICmator's canister "X"`
  in the subject attribute — it embeds the canister obj-spec as the
  `from` container of whatever spec the inner `tell` block evaluates.
  Swift's `peelCanisterAnchor` walks the `from` chain, strips any
  `'ICcn'`-want layer, and returns both the rewritten descriptor
  (with the anchor replaced by `null`) and the extracted canister id.
- **Rust agent argv path.** `icmator-agent <method> [<canister-id>]`;
  falls back to `CANISTER_ID` env when argv[2] is absent.  Existing
  single-canister scripts unchanged.

### Original design sketch (kept for context)

Today ICmator is wired to a single hardcoded canister: `icmator-agent`
reads `CANISTER_ID` at process launch, the Swift side has no
session-level concept of "which canister".  The next step is letting
scripts pick a target per `tell`:

```applescript
-- per-call
tell application "ICmator"'s canister "t63gs-up777-77776-aaaba-cai" to ¬
    eval (every client whose country = "Germany")

-- session default
tell application "ICmator" to ¬
    set defaultTarget to canister "rwlgt-iiaaa-aaaaa-aaaaa-cai"
tell application "ICmator" to eval (every client)        -- uses the default
```

This is groundwork that must land *before* `#root` rewriting, because
the OSL's `#root` is a per-canister identity — rewriting only makes
sense once the bridge knows which canister's root it's rewriting.

### SDEF surface

```xml
<class name="canister" code="ICcn" plural="canisters">
    <property name="id" code="ID  " type="text" access="r"
              description="The canister's IC principal as text."/>
</class>

<class name="application" code="capp">
    <elements>
        <element type="canister" access="r"/>
        <element type="client"   access="r"/>   <!-- kept for backward compat -->
    </elements>
    <property name="default target" code="dtgt" type="canister" access="rw"
              description="Canister used when no nested `tell` selects one."/>
</class>
```

4cc choice: `ICcn` (uppercase first letter — Apple's rule for app-
specific OSTypes), `dtgt` for the property, `ID  ` (with trailing
spaces) for the canister-id property.

### Wire-level dispatch

AppleScript translates `tell A's B to verb obj` into an AE event whose
**`keySubjectAttr`** (`'subj'`) attribute is an obj-spec referring to
`B` (here: `canister "X"`).  The event's `keyDirectObject` still
carries `obj` (the actual specifier we want evaluated).

Our `handleEval` therefore grows two passes:

1. Inspect `event.attributeDescriptor(forKeyword: keySubjectAttr)`.
   - Present and shaped as an `'obj '` with `want = 'ICcn'`, `form =
     'name'`, `seld = utxt(id)` → use *that* canister id.
   - Absent → fall back to a session-default (a Swift `@property` on
     `ICmator` initialised from `defaultTarget` if previously set,
     else from the `CANISTER_ID` env var as today).
2. Pass the resolved id to `icmator-agent` as the first argv after
   the method name: `icmator-agent go <canister-id>`.  Rust agent
   already builds an `Agent` per call; just route the principal
   in instead of reading the env var.

For the `set defaultTarget` path, intercept `kAESetData` on
`'capp'/'dtgt'` in Swift via `setEventHandler` (mirror of how we
already win against Cocoa scripting for `eval`).  The handler unpacks
the canister obj-spec from `keyAEData`, extracts the id, stores it on
`self.defaultTarget`.  No persistence across restarts in the first
cut — make it sticky later via `UserDefaults` once it's proven.

### What does NOT change (yet)

- The canister still owns its own `#root` semantics.  We are only
  selecting *which* canister `eval` talks to; the bytes on the wire
  to that canister are unchanged.
- No `network` AE class.  The IC URL stays a single env var per
  ICmator process (local replica vs mainnet vs a fork is one ICmator
  install per environment, for now).
- The `client` AE class element on `application` continues to work
  unchanged — it implicitly addresses the default target.  Demos
  written today keep working.

### What this unlocks

- The `_aeLingo` query (canister returns its SDEF / accessor surface)
  can be issued per canister — different canisters can advertise
  different vocabularies and ICmator can lazily fetch each.
- `#root` rewriting becomes well-defined: when forwarding to canister
  `X`, the bridge can rewrite a `#root` that came from a previous
  reply against canister `Y` into the right cross-canister specifier
  (or trap with a clear error).  Without per-call canister selection
  there is no `Y` to rewrite *from*.

## Next: AE-native lingo query

`_aeLingo` is the canister's self-description endpoint.  ICmator
issues it per canister (any time it sees an unfamiliar target) and
caches the result.  The reply is itself an AE-encoded structure so
the same wire-format work we've already done carries it.

Concrete shape (sketch, may change once we start typing it):

```
record {
  class_4ccs : list of obj { 4cc : type;  parent : type or null };
  property_4ccs : list of obj { class : type;  4cc : type;  value_type : type };
  forms_supported : list of enum;       -- name / indx / test / rang …
  predicate_ops : list of enum;         -- =, !=, <, >, contains, …
  capability_flags : list of enum;      -- read, count, set, create, delete
}
```

ICmator caches `(canister-id → lingo)` so a session needs at most one
lingo round-trip per canister.  The bench's `object-spec.mo` would
expose this as a `(with encoder)` query method named `_aeLingo`; pure-
Candid canisters would expose it as a Candid query returning a typed
record (handled by the next section's bridge).

This is the **next concrete step** — multi-canister addressing
without lingo means clients have to hardcode every canister's
vocabulary, which doesn't compose.

## Best-of-both-worlds: Candid ↔ AE bridge in the agent

The big epiphany (2026-05-20): the wire-format translation between
Candid and AE doesn't have to live in *each* canister.  It can live
in `icmator-agent` (Rust), as a reusable transcription of the OSL's
schema-mapping rules.  Then ICmator can talk to **any** IC canister
— including pure-Candid ones with no awareness of AE — and surface
their state through AppleScript.

### Egress (canister → AS)

`icmator-agent` sniffs the response bytes.  If they start with
`4449444c` (`DIDL`, Candid magic), the agent:

1. Decodes the bytes as Candid (using the canister's `.did` or an
   inline type passed via the lingo lookup).
2. Walks the resulting typed value through the OSL's schema map
   (port of `writeDesc` rules to Rust) and emits an AE descriptor
   tree: `record → typeAERecord`, `vec → typeAEList`, `variant →
   typeObjectSpecifier`, `text → utxt`, `int32/nat32 → long`, etc.
3. Returns those bytes to ICmator, which unflattens normally.

For AE-native canisters (`(with encoder)` annotations like
`object-spec.mo`'s `go`), the response bytes start with `646c6532`
(`dle2`) and the agent passes them through unchanged.

### Ingress (AS → canister)

Symmetric.  For each call the agent looks up the target canister's
**format flag** (Candid vs AE-native, discovered via lingo and
cached):

- **AE-native target** → flatten the AS-side descriptor and forward
  bytes as today.
- **Candid target** → parse the AS-side descriptor into the agent's
  internal `ObjectSpec`, then Candid-encode against the canister's
  `.did` signature, forward those bytes.  The canister sees a
  perfectly ordinary Candid call.

### Schema map lives in Rust

The OSL's `writeDesc` / `parseTopLevel` body is currently Motoko, but
the rules are mechanical: each Motoko/Candid type maps to one AE
descriptor type, with a small surface (~10 mappings).  Rust port
goes into a new crate (likely `icmator-codec`) shared between the
agent and the canister's pure-Rust dependents (e.g. future cdks).

### Lingo discovery handles both

The lingo query — at the canister, not the agent — is what tells the
agent *which path to take*.  Lingo replies can declare:

- `wire = ae`     — pass-through both directions.
- `wire = candid` — agent does the Candid ↔ AE translation, using
  the lingo's accompanying type definitions.  Restricted to **stable
  Candid types**: primitives, simple records, vecs, variants whose
  shape doesn't depend on `ObjectSpec`.

### Hard rule: no public Candid `ObjectSpec`

The `ObjectSpec` Motoko/Candid type is **not stable** — its variants
(`#obj`/`#root`/`#value`/`#list`), field names, and nested
`KeyForm`/`BoolExpr` shapes are an evolving research surface.
Pinning it as a public Candid interface type would freeze the
language we use to talk about queries before it's actually settled,
and any change would break every dependent canister.

So **canisters that want to expose OSL semantics publicly MUST go
through the AE wire** (`(with encoder; decoder)` annotation).  AE
bytes have no Candid type signature — the canister carries the
encoder/decoder as code, and ObjectSpec stays purely internal.
Schema evolution is a code-version concern, not an IDL-break.

What the Candid path in the agent is for, then: **non-OSL endpoints**
on the same canister.  Status, metrics, cycles balance, admin
queries — Candid-shaped, stable-shaped, low-risk to wrap.  The agent
translates these so AS can read them, but it does **not** try to
re-derive OSL semantics from a Candid-shaped ObjectSpec record.

### Network targeting — same handle shape

The richer addressing form combines both:

```applescript
tell application "ICmator" to target canister "bla-cai" of network "testnet"
```

AS compiles this to an obj-spec `canister "bla-cai"` whose `from`
container is itself an obj-spec `network "testnet"`.  Our existing
`peelCanisterAnchor` walks the `from` chain linearly; adding a
`peelNetworkAnchor` is the same shape with `want = 'ICnt'` and a
different state field (e.g. `defaultNetworkURL`).  No new AS syntax
or AE plumbing — just one more class declaration in the SDEF and
one more switch arm in `handleTarget`.

### Defaults

- **`network` omitted → mainnet.**  If a `tell` or `target` provides
  a canister but no `of network`, the agent assumes mainnet
  (`https://ic0.app`).  Local-replica work explicitly opts in via
  `target network "local"` (or env-var override at ICmator launch
  for the demo).
- **`canister` omitted → error**, unless a session default is set
  via prior `target canister "X"`.  Mainnet without a canister has
  nothing concrete to call.  The error should surface as a
  meaningful AS message ("no canister target — call `target canister
  …` first or wrap your `tell` in `tell canister …`"), not silent
  `null`.

### Open design questions

- **Where does the Candid schema come from?** Three candidates: (a)
  the canister's own `.did` fetched via the IC management canister
  (`canister_status` or `__get_candid_interface_tmp_hack`); (b) baked
  into the lingo reply; (c) shipped alongside ICmator as a static
  config bundle.  (a) is the most decentralised, (b) the most
  self-contained, (c) the most demo-ready.
- **Record-shape conventions.** Candid records are unordered and
  named-field; AE records carry an ordering and use 4-char keywords.
  The schema map needs a stable Candid-label → 4cc mapping
  (truncate? hash? require a side-table from lingo?).  This is the
  same problem `mo_to_idl.ml` already solves for Motoko↔Candid;
  borrow its rules.
- **Variants ↔ obj specs.** A Candid variant `{ #obj : ...; #value :
  ...; ... }` maps cleanly to AE typeObjectSpecifier vs typeAEList vs
  leaves.  Matches the bench's `ObjectSpec` definition almost 1:1.
  Generic variants (no ObjectSpec shape) translate to AE records
  with a `kind` discriminator.
- **Tuples and recursive types.** Candid tuples have no AE
  equivalent (use typeAEList of mixed types).  Recursive Candid
  types need cycle detection in the schema walker.
- **Where do `update` vs `query` decisions live?** Today the env
  var `IC_CALL_KIND` picks update.  Lingo should advertise per-
  method.

### Why this is still big (with the hard rule applied)

- **Read-only access to admin/status surfaces** of any canister
  without per-canister porting.  Cycle balances, controllers, last
  heartbeat — all Candid-shaped, stable, agent-translatable.
- **OSL semantics live behind AE.** The encoder annotation is the
  canonical, stable interface.  ObjectSpec can evolve without
  breaking deployed canisters, because the bytes-on-the-wire never
  carried a Candid type schema for it in the first place.
- **One schema language, two wire formats — clean separation.**
  Candid for "frozen / stable / no-OSL" shapes; AE bytes for the
  OSL surface.  The agent picks per call.

## Scope cut for Friday

**In**:
- `client` class only — `name`, `country`, `age`, `yearlyIncome`.
- `kAEGetData` and `kAECountElements`.
- `whose` predicates (the existing `BoolExpr` shape — `and`/`or`/`not`/
  comparisons).
- `every X`, `first X`, `Nth X`, `X named "..."`.
- Local `dfx start` running, canister deployed.

**Out**:
- `card` class (one class for demo clarity).
- `network`/`canister` AE classes (single hardcoded target).
- Code-signing / notarization (right-click-Open Gatekeeper bypass).
- Production error handling (return `errAEEventNotHandled` on weird).
- `kAESetData` (setters need a different mental model).

## Modern alternative — App Intents (macOS 13+)

Not the chosen path for ICmator, but worth knowing about:

App Intents are discoverable, require no SDEF, and are installable by
third parties.  A "Query IC Canister" Intent accepts a canister ID,
network ID, and `ObjectSpec`-derived parameters and returns typed
results.  Different model from AppleScript but better suited for
Siri/Spotlight/Shortcuts.

App Intents don't speak AE on the wire, so the canister-side codec
wouldn't be reused — that loses the pass-through elegance.

## Directory layout

```
~/ICmator/
├── flake.nix              # devShell with rust + swift/xcrun + plutil
├── Makefile               # app/agent/install targets
├── src/
│   ├── main.swift         # AE handlers, Process spawning
│   └── ICmator.sdef       # scripting definition
├── resources/
│   └── Info.plist         # bundle metadata
├── agent/
│   ├── Cargo.toml         # icmator-agent
│   └── src/main.rs        # the ic-agent subprocess
└── build/                 # .app bundle output
```

## Long-term

- Native Swift signing (drop the Rust subprocess).
- Multi-canister: SDEF declares `network` and `canister` AE classes;
  AppleScript can write `tell application "ICmator" to tell network
  "ic" to tell canister "abc-..." to get every client ...`.
- **AE terminology fetch** via a *custom AE-native endpoint* on the
  canister — **not** the `.did`.  Candid types (records, variants,
  `vec`, tuples) are the wrong vocabulary for SDEF generation; AE
  needs classes-with-4ccs, plural names, element-of relationships,
  AE-typed properties.  The canister already knows its terminology
  (`class4cc` on every Smurf, `fourcc`/`form` on every accessor); a
  `terminology() : async TerminologyDoc` query returns the
  AE-flavoured tree, ICmator turns it into the in-memory SDEF (or
  writes one into `Contents/Resources/`) at startup.  Per-canister
  schema with zero `.did` round-tripping.
  - **`_aeLingo` (query)** — AppleScript-side handle to the same
    mechanism: `tell application "ICmator" to _aeLingo` returns the
    canister's current terminology tree as a script-readable
    structure (or just the SDEF XML).  Useful for ad-hoc inspection
    in Script Editor, for the "tell canister X" multi-canister
    addressing, and as a one-call introspection probe.  Stretch /
    *if time permits*.
  - **4cc casing convention** — application-specific 4ccs must
    contain at least one uppercase ASCII character.  Apple reserves
    all-lowercase OSTypes for system use; mixing case avoids future
    collisions when Apple introduces new reserved codes.  The bench
    currently uses all-lowercase (`clnt`, `card`, `name`, `cntr`,
    `vali`, etc.) — fine for the local demo but the synthesised
    SDEF should rewrite to e.g. `Clnt`/`Card`/`Name`/`Cntr`/`Vali`
    before exposure.  Same rule applies to user-defined class names
    via `_aeLingo`.
- App Intents twin: same backend, different surface — useful for
  Siri/Spotlight.
