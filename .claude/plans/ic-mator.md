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

### Composability follow-on

The returned obj specifiers are reusable.  Once name-lookup is wired,
this should work:

```applescript
tell application "ICmator"
    set hits to (eval (every client whose country = "Germany"))
    set who to item 1 of hits
    -- `who` is now `client "Hans Müller" of application "ICmator"`.
    -- A subsequent `eval` (or `get`) targeting `who` resolves the
    -- specifier on the canister side via the `#named "clnt"` accessor
    -- already exposed in the bench.
    eval (yearly income of who)
end tell
```

i.e., the AS-side specifier `client "Hans Müller" of application
"ICmator"` round-trips through the bridge: flatten descriptor →
canister sees `#obj { class_ = "clnt"; container = #root; key = #name
"Hans Müller" }` → resolve via `VarAccessor<Client>("clnt", #named,
…)` (already there) → return `ValueSmurf(#int32 c.yearlyIncome)` →
unflatten back as a `typeSInt32`.  No protocol change needed —
just the property-lookup `eval` path Swift-side, and the existing
canister surface handles it.

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
