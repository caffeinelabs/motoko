# OSL — Object Support Library: agent guide & pitfall list

A skill sheet for anyone (human or agent) building a canister on top of `OSL.mo`,
or extending OSL itself. The goal is to **not re-discover the traps** — they cost
real debugging time, and most are non-obvious AppleScript/AEOM interactions.

This is a living document. Append a pitfall the moment you hit one.

---

## What OSL is (and isn't)

OSL lets an Internet Computer canister speak the **AppleScript Object Model**
(AEOM) over the wire — Apple Event object specifiers ⇄ Candid — so a Mac bridge
(see `~/ICmator`) can drive the canister with `every client whose …`,
`properties of …`, `id of …`, etc.

**OSL owns** (data-model-agnostic; it never knows *your* schema):
- the wire value/spec types — `CandidValue`, `ObjectSpec`, `KeyForm`, `BoolExpr`;
- the **Smurf protocol** (`Smurf`, `Accessor`, `LookupKey`);
- AE encode/decode — `encodeAE` / `parseTopLevel` (the `(with encoder; decoder)`
  annotations route these);
- the resolver — `eval` / `resolve`;
- the universal `properties` accessor — `pAllAccessor`;
- the generic Smurf constructors — `simpleLeaf`, `VarAccessor<T>`,
  `CollectionSmurf<T>`, `FlattenedSmurf<P,E>`, `smurfMap`, `findAccessor`,
  `notFoundSmurf`;
- the **Lingo vocabulary types** — `Lingo`, `LingoClass`, `LingoProperty`, ….

**The canister writer owns** (in e.g. `object-spec.mo`):
- per-entity `wrap : T -> Smurf` builders (e.g. `clientSmurf`);
- the typed `[PropReader<T>]` tables + `lookupReader` (the predicate fast-path);
- the **`lingo()`** content — which classes/properties/elements exist (you know
  the schema). OSL only hands you `LingoClass` to populate.
- helpers like `lingoPropsOf` / `unionProps` (currently canister-side; candidates
  to lift into OSL as optional helpers).

---

## The Smurf protocol

```
Smurf    = { class4cc : Text;
             accessors : [Accessor];
             toDesc    : () -> async* ObjectSpec;   // render IDENTITY (a ref path)
             filter    : BoolExpr -> Smurf }         // `whose` (pass-or-empty / accumulate)

Accessor = { kind   : { #property; #element };       // AEOM split — see pitfall 2
             form   : { #indexed; #named; #test; #id };
             fourcc : Text;
             lookUp : (parent : Smurf, key : LookupKey) -> Smurf }   // navigate IN; notFoundSmurf on miss
```

- `toDesc` renders *where the object is* (its specifier), for the reply — not its
  values. `properties of X` (pALL) is what renders the *values/columns*.
- Reuse the generic constructors; don't hand-roll element navigation.

---

## Writing a canister on OSL (recipe)

1. For each entity `T`, a `wrap(t) : Smurf` whose `accessors` expose:
   - its **properties** — `kind = #property`, usually `lookUp = ValueSmurf/simpleLeaf(<value>)`;
   - its **elements** — `kind = #element`, via `VarAccessor`/`CollectionSmurf`/`FlattenedSmurf`.
2. A `[PropReader<T>]` table: `{ fourcc; lingoName; valueType; description; read : T -> CandidValue }`.
   Feeds predicate eval (`lookupReader` → `evalPred`) and terminology (`lingoPropsOf`).
3. `lingo()`: one `LingoClass` per class (`name/code/plural/properties/elements`);
   `properties = lingoPropsOf <table>`. This is what the bridge turns into the `.sdef`.
4. A root Smurf exposing the top-level element collections.

---

## Pitfalls (read before you debug)

1. **Never name a property `id`** (nor `name`, `index`, …). A property whose
   `lingoName` is `id` *shadows AppleScript's `id` key form* (`<class> id <spec>`,
   how you address an object by its unique id). AppleScript then reads
   `left join id {…}` as "the text property `id`" and fails to coerce the value
   → **-1700**. Rename the property (e.g. `"order key"`); the **wire 4cc code is
   unconstrained** — resolution binds on the *code*, so `id of order` still works
   via the standard id property. (Cost real time on the `left join` union, which
   pulled in order's `id`.)

2. **Tag every `Accessor.kind` correctly.** `#property` accessors appear in
   `properties of X` (pALL); `#element` (collections) don't. Mis-tagging
   pollutes or empties `properties`. Synthetic accessors (`pcnt` count, `prop`
   project) are `#element`. `kind` must survive `smurfMap`'s merge — it does, but
   keep it threaded if you touch the merge (join rows depend on it).

3. **Pick the addressing form by identity, not habit.** Text-named objects
   (a person) → `name` (formName): `client "Hans Müller"`. Key-identified objects
   (an order keyed by `"ORDnnnn"`) → `id` (formUniqueID): `order id "ORD0000"`.
   Advertise the form you support; for non-text identity, advertise `id` and
   **not** `name`, so `order "x"` is a clean compile error rather than a stringly
   hack. (Note: scalar `id`-form decode for a *single* object is still a TODO —
   `parseObjBody`'s `ID` branch currently decodes only list/record join payloads.)

4. **`pALL` is universal — never store it in a Smurf's `accessors`** (it would
   list itself). `resolve` falls back to `OSL.pAllAccessor` for `#property "pALL"`.

5. **Definedness (M0016).** A `transient let` (e.g. the root Smurf) whose init
   closures reference functions defined *later* in the actor fails. Define the
   helpers **above** it, or wrap the mutually-recursive group in
   `object { public let … }` and destructure: `transient let { a; b } = object { … }`.

6. **AppleScript record labels are property codes — and *query* labels are aete
   "extras", not schema.** To accept a keyed record arg — `id {left: …, right: …,
   on: …}` — the labels must be **declared** so AppleScript maps them to 4cc
   codes; the clean AEOM form is a `<record-type>`. Crucially, labels for a
   *query/join* construct (`left`/`right`/`on`, codes `Left`/`Righ`/`On  `) are
   **not data properties** — no entity has them — so they belong in the **query
   layer's static suite** (the agent's `icmator_static_lingo`, the aete extras),
   **NOT** the canister's data `lingo()`. Keep the two separate: data schema is
   canister-owned (`lingo()` → classes/properties); the join *mechanism* is
   query-owned. Undeclared labels → syntax error (-2741), or AppleScript stuffs
   them into user-defined `usrf` fields (different wire shape). A *positional*
   list `{a, b}` needs no labels but can't carry an `on:` key — plumbing, not a
   real join.

   Where terminology comes from (don't conflate): the bridge's `.sdef` =
   **agent static suite** (`icmator_static_lingo` — `eval`/`query`/`target`, the
   `canister` class, and join/query extras) **+ the canister's `lingo()`** (the
   data classes). Data → canister; query vocabulary → agent extras.

7. **Encoder/decoder symmetry.** A new `CandidValue`/`KeyForm` variant needs
   *both* a `writeValue`/`writeObjBody` arm (encode) **and** a
   `parseValueBody`/`parseObjBody` arm (decode), or you trap on the wire. New
   exhaustive `switch`es over `CandidValue`/`ObjectSpec` also need the arm (or a
   wildcard) — a missing case is a non-exhaustive-match warning that breaks the
   `.tc` golden.

8. **The "great row"/universal-property union must dedup by code** (use
   `unionProps`) **and respect pitfall 1** (no `id`/`name`/`index` label clashes).

---

## Specifier surface (what callers write)

- `properties of X` → `pALL` → the present `#property` codes (the object's columns).
- `<prop> of X` → property; `every X` / `X N` / `X "name"` / `X id <v>` /
  `X whose <test>` → elements.
- `count of (every X whose …)` → the `pcnt` accessor.
- The grammar is **closed** (no new infix operators); model new capabilities as
  classes/properties/elements/verbs. Join design lives in
  `.claude/plans/joins.md`.

---

## Build / test loop (ICmator)

- Canister wasm: `nix build .#object-support` (from `~/ICmator`); but **iterate via
  `make redeploy-fresh`** — it builds from the *working tree* (`--override-input`),
  reinstalls, and relaunches the app, dodging the stale-`flake.lock` trap and the
  `icp deploy` Candid-compat check.
- Golden round-trip: `make check` (needs the app up + canister deployed).
- After a canister reinstall the app can wedge — `make start`/`redeploy-fresh`
  relaunches it.
