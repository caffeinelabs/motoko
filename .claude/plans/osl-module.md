# OSL.mo — Extraction Plan

Extract the data-model-agnostic Object Support Library from
`test/bench/object-spec.mo` into a standalone `OSL.mo` module that any
canister can import.

---

## Motivation

`object-spec.mo` fuses three concerns:

1. **AE codec** — binary encoding/decoding of Apple Event Object Specifiers
2. **OSL** — data-model-agnostic spec walker (Smurf protocol, resolve/eval)
3. **Bench** — the specific Client/CreditCard/Char data model and tiny* demos

Separating (1)+(2) into `OSL.mo` lets any canister expose an AEOM query
interface by importing the module and providing its own Smurfs.

---

## Target layout

```
test/bench/
  OSL.mo          ← new: the reusable library
  object-spec.mo  ← reduced to bench data model + glue + tinyN demos
```

---

## OSL.mo contents

### Wire types (fully OSL)
- `CandidValue`, `Comparison`, `BoolExpr`, `KeyForm`, `ObjectSpec`

### Predicate evaluation (OSL)
- `cmp` — type-polymorphic CandidValue comparison

### Smurf protocol (OSL)
- `LookupKey`, `Smurf`, `Accessor`
- `notFoundSmurf` — AE-404 sentinel
- `findAccessor` — linear fourcc+form lookup on a Smurf
- `iteri` — positional fold helper
- `smurfMap` — distributive lift over `[Smurf]` (for broadcast results)

### Generic collection Smurfs (OSL — currently in bench)
- `VarAccessor<T>` — stable `[T]` with wrap/getName
- `CollectionSmurf<T>` — filtered view with predicate composition
- `FlattenedSmurf<P,E>` — 1→many join across a parent collection

### AE decoder (fully OSL)
- `Reader` class
- All 4cc constants (`DLE2`, `OBJ`, `NULL`, `LOGI`, `CMPD`, `LIST`, …)
- `u32`, `cc4ToText`, `utxtToText`
- `parseDescBody`, `parseObjBody`, `parseTopLevel`
- `parseValueBody`, `parseValue`, `parseDescFromBody`, `parseInListBody`
- `parseBoolExprBody`, `parseBoolExpr`, `parseLogiBody`, `parseCmpdBody`

### AE encoder (fully OSL)
- `Writer` class
- `textToCC4`, `valueDescLen`, `boolExprDescLen`, `seldBodyLen`, `encDescLen`
- `textToUtf16`, `utf16Units`, `compareOpCC`
- `writeValue`, `writeLogiHeader`, `writeBoolExpr`, `writeObjBody`, `writeDesc`
- `encodeAE`

### Spec walker (fully OSL)
- `formOfKey`, `lookupOfKey`, `resolve`, `eval`

### Lingo types (OSL — no bench coupling)
- `LingoValueType`, `LingoAccess`, `LingoProperty`, `LingoElement`
- `LingoClass`, `Lingo`

---

## Stays in object-spec.mo

### Bench data model
- `CreditCard`, `Client`, name pools, `twoDigits`, `clients` DB
- `cardMonth`, `cardYear`, `cardIsValid`, `today`
- `arrayOfChars`, `firstWord`, `lastWord`

### PropReader machinery (bench-specific metadata)
- `PropReader<T>`, `lookupReader`, `lingoPropsOf`
- `propReaders`, `cardPropReaders`, `charPropReaders`
- `evalBoolExpr`, `evalCardPred`, `evalCharPred`

### Bench Smurfs
- `clientPropAccessor`, `clientSmurf`
- `cardSmurf`, `charSmurf`
- `actorSmurf`, `clntCollection`

### Instrumented wrappers (glue)
- `encoder`, `decoder` — wrap OSL's `encodeAE`/`parseTopLevel` with
  `debugPrint` profiling
- `go`, `ae` — `(with encoder; decoder)` public entry points

### Lingo derivation + endpoint (glue)
- `smartValueSmurf` — bench-specific SmartLeaf extension
- `lingo()` — returns bench Lingo built via `lingoPropsOf`

### Demos
- All `tinyN` functions

---

## Tricky cases

### `ValueSmurf`
Currently GLUE: the class body is data-model-agnostic (CandidValue leaf
wrapper with char navigation), but it calls `charSmurf` which is bench-specific.

**Resolution:** parametrize `ValueSmurf` with a `charWrapper : (Char, Nat, Smurf) -> Smurf`
function argument (defaulting to the bench's `charSmurf`).  Once
parametrized, `ValueSmurf` is fully OSL.

### `encoder` / `decoder`
These are thin instrumentation wrappers around `encodeAE` / `parseTopLevel`.
Their body logic is OSL but the `debugPrint` calls are bench noise.
Options:
a) Leave them in object-spec.mo as bench-specific wrappers.
b) Give `encodeAE` / `parseTopLevel` a `profile : Bool` flag.
c) Rely on the `(with encoder; decoder)` mechanism — `go` wires the
   raw OSL functions in directly, removing the need for named wrappers.

Lean toward (a) for now.

---

## Migration steps

1. Create `OSL.mo` as a `module { … }` wrapping all OSL items listed above.
2. In `object-spec.mo`, add `import OSL "OSL"` and open the module
   (`let { CandidValue; Smurf; … } = OSL`), or qualify with `OSL.`.
3. Parametrize `ValueSmurf` with `charWrapper`; pass `charSmurf` at
   instantiation site in object-spec.mo.
4. Run `make -C test/bench object-spec.only` and accept any cycle-count
   delta in the .ok file.
5. Add a `//CALL query lingo` regression test to confirm the lingo
   endpoint still matches `bench_lingo()` in icmator-agent.

---

## Open questions

- Should `OSL.mo` live under `test/bench/` (local to the bench) or
  somewhere more prominent (e.g. `src/prelude/`, `test/lib/`)?
  Suggests a future `caffeinelabs/osl` mops package once stable.
- Should `PropReader<T>` and `lingoPropsOf` move to OSL?
  They are generic but currently carry bench descriptions.  Could be
  OSL if the metadata is defined as an opaque `{ fourcc; lingoName;
  valueType; description }` record type exported from OSL.
