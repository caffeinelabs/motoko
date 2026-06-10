# Joins over an AppleScript → Internet Computer bridge

Design notes from a working session.

**Setup.** An **AppleScript-to-IC bridge**: a scriptable Mac front end that receives
Apple events and forwards them to an Internet Computer **canister**. The canister
returns its scripting vocabulary **dynamically** (aete-like suites); the bridge
**synthesises an `.sdef` on the fly** from that. The canister's data are Candid
`vec record { … }` — "kinda-tables" (e.g. `employees`, `managers`), **no SQL**.

**Goal.** Express a relational **join** (employees ⋈ managers on `cubicle`) from
AppleScript.
**Hard constraint.** Avoid **n·m round-trips** — every Apple event / canister call
is expensive "ping-pong".

---

## The one principle everything follows

> **The join predicate travels as DATA; the join executes in exactly ONE place; one round trip.**

Never evaluate `cubicle of left = cubicle of right` as live AppleScript —
correlating two collections pair-by-pair is exactly what forces n·m calls. Once the
key is a *property code* (or record/enum) instead of a live expression, the
bridge/canister pulls each side once and joins in compiled code.

**AppleScript corollary:** the grammar is **closed**. An aete/sdef adds *terms*
(classes, properties, elements, commands, enums) — **never** new infix operators or
clause keywords. So there is no `A joined with B where left = right`; the join is a
**verb**, or a **view** (a class), and the predicate is **data** — a parameter, or a
field of the view's `id` — never a clause.

---

## The headline pattern — a verb-less, specifier-only join

Model `join` as a **class**, not a verb. A specific join is one *object*, identified by
its spec via the `id` key form; its joined tuples are its `rows`. Pure object-specifier
navigation — no command:

```applescript
rows of (join id {left: manager, right: employee, on: cubicle})

-- composite key (the floor/cubicle office, below):
rows of (join id {left: manager, right: employee, on: {floor, cubicle}})

-- filter the RESULT (legal: single-class test on rows, pushed down):
(rows of (join id {left: manager, right: employee, on: cubicle})) whose floor is 1
```

**Why it's sound**
- A join **is** uniquely determined by (left class, right class, key) — so the spec
  literally *is* its `id` (`formUniqueID` carrying a record). Different spec → different
  join object.
- The match condition lives **in the id** (`on:`), as data — never a correlated `where`
  (`left's cubicle = right's cubicle` can't bind `left`/`right`, and would be n·m).
- `whose`/`where` survives only as a **post-join filter on `rows`** (single-class
  `formTest`, pushed into the query). That is the *only* legal `where` here.
- Operands are **class constants** (singular `manager`/`employee`) — metadata, no
  pre-fetch; the canister does the whole join in one round trip.

**Reading the result — each `row` is an object with properties** (= the joined columns):

```applescript
properties of row 1 of (join id {…})       -- whole tuple: {cubicle:"A3", floor:1, employee:"Alice", manager:"Dana"}
manager of every row of (join id {…})       -- ONE column across all rows: {"Dana","Erin"}  (a projection)
properties of every row of (join id {…})    -- all columns, all rows: list of records
manager of (first row of (join id {…}) whose employee is "Alice")   -- filter, then pluck → "Dana"
```

`<property> of every row` *is* a **SELECT/projection**: the bridge maps the property code
→ column, so the canister returns just that column — one round trip, not the whole table.
Filtering (`whose`) and projection (`… of …`) compose, and both push down.

*Optional, powerful:* type a row's `employee`/`manager` properties as the **`employee`/
`manager` classes** (object references), not `text`. Then `manager of row 1` is a real
object and you can navigate deeper — `floor of manager of row 1` — i.e. the join result
links back into the graph instead of flattening to strings.

**Selecting & addressing rows.** Pick a row by an attribute value with `whose` — a single
property against a **constant** (pushed down):

```applescript
every row of (join id {…}) whose floor is 1
manager of (first row of (join id {…}) whose employee is "Alice")
```

Property access is the ordinary possessive: `row's left` / `left of row` (valid iff
`left` is a declared property). There is **no** `row's property left` form; to
disambiguate a name that clashes with a keyword, pipe-quote (`row's |left|`) or use the
raw chevron (`row's «property xxxx»`) — never the word `property`.

**Don't** correlate two row properties in the filter: `whose left's cubicle = right's
cubicle` is the n·m cross-product-then-filter trap in disguise (it forces the product to
materialise), it's *redundant* after an `on:` join (every surviving row already matches),
and same-element property-vs-property tests are fragile anyway. `whose` on rows =
property **vs constant**; the join condition lives in `on:`.

**aete:**
```xml
<class name="join" code="join" description="a virtual join, identified by its spec">
  <property name="id" code="ID  " type="record" description="{left, right, on} — the spec"/>
  <element type="row"><accessor style="index"/><accessor style="test"/></element>
</class>
<class name="row" code="jrow">
  <property name="cubicle"  code="cube" type="text"/>
  <property name="floor"    code="flor" type="integer"/>
  <property name="employee" code="emp1" type="text"/>   <!-- or type="employee" for a navigable ref -->
  <property name="manager"  code="mgr1" type="text"/>
</class>
```

Resolution: `join id {…}` → bridge decodes the spec (class codes + key codes) → a join
handle; asking for its `rows` (optionally `whose`-filtered, with a chosen property =
projection) → **one** canister join query → row objects. Verb-less, condition-as-data,
no n·m.

This is the synthesis of the thread: **view-class** (verb-less, composes with `whose`) +
**spec-as-`id`** (operands + key *are* the object's identity) + **predicate-as-data**
(key in the id, not a clause).

---

## The universal `row` class (wide schema, sparse instance)

Because the aete is generated, declare `row` at **generation time** as the **union of
every property and every element type across all canister entities** — one "great `row`
class" that unlocks all columns and sub-objects. Any given *runtime-joined* row populates
only a subset; the rest simply aren't there.

- **Misses are runtime user errors.** Asking for a property/element a particular join
  didn't produce → return **`errAENoSuchObject` (-1728)**. Accepted cost: existence checks
  move **compile-time → runtime** — every script compiles (the term is in the dictionary)
  but may fault on a join that lacks the field. That's the price of the universal row.
- **`properties of row` reports what's present.** The resolver builds the reply record,
  so return **present-only** keys (not all-declared-with-`missing value`) — that *is* the
  "what's here" report.
- **Elements have no built-in introspection** (`properties` is a special aggregate; there
  is no `elements` analog). Two options:
  - **probe a named class** — `every employee of row` / `count employees of row` → the
    instances, `{}`, or `errAENoSuchObject`. You must know the class to ask.
  - **declare the mirror** — since you own the aete, add a read-only `present elements`
    property returning the element-class tokens the row actually has
    (`present elements of row ⇒ {employee, manager}`). The element-side twin of
    `properties`.
- **Decide empty vs absent** for a class *declared* on `row` but unpopulated in a given
  join: `{}` (present-but-empty) or `errAENoSuchObject` (n/a for this row). Pick one, and
  keep `present elements` consistent with the per-class probe.

**Modeling rule of thumb** (what's introspectable for free):
- **one-per-row** sub-object (the `left`/`right` sides) → an **object-reference property**
  — shows up in `properties`, and navigates: `floor of left of row`.
- **many-per-row** → an **element** (gains `every`/`whose`/`count`) + the `present
  elements` reporter.

So most "is it here?" questions are answered by `properties` alone; `present elements`
carries only the genuinely collection-valued part.

---

## Alternative — a `join` verb (the imperative sibling)

The verb form is the *ad-hoc* sibling of the headline: the same spec expressed as a
command rather than an object. Reach for it for one-off joins; reach for the class/`id`
form (above) when you want the result to **compose** (a command's result can't take a
postfix `whose`; a class's `rows` can).

```xml
<suite name="Canister DB Suite" code="cdb1">
  <enumeration name="join key"  code="jnky">          <!-- closed set of joinable columns -->
    <enumerator name="cubicle" code="cube"/>
    <enumerator name="age"     code="age "/>
  </enumeration>
  <enumeration name="join type" code="jnty">
    <enumerator name="inner" code="jnin"/>
    <enumerator name="left"  code="jnlf"/>
    <enumerator name="right" code="jnrt"/>
  </enumeration>

  <command name="join" code="cdb1join"
           description="Relational join of two element classes by a shared key.">
    <direct-parameter description="the two sides">
      <type type="type"      list="yes"/>   <!-- {employee, manager}  (class constants) -->
      <type type="specifier" list="yes"/>   <!-- {a reference to (…), a reference to (…)} -->
    </direct-parameter>
    <parameter name="on"    code="onky" optional="yes" description="join key property/properties">
      <type type="join key"/><type type="join key" list="yes"/>
    </parameter>
    <parameter name="in"    code="inrt" type="specifier"  optional="yes" description="scope (default = app/canister root)"/>
    <parameter name="where" code="whrx" type="text"       optional="yes" description="result filter, pushed into the query"/>
    <parameter name="as"    code="astp" type="join type"  optional="yes" description="join kind (default inner)"/>
    <result><type type="record" list="yes"/></result>
  </command>
</suite>
```

```applescript
tell application "ICBridge"
    join {employee, manager} on cubicle               -- leanest: class constants, canister-side
    join {employee, manager} on {cubicle, age}        -- composite (tuple equi-join)
    join {employee, manager} in root as left          -- key inferred from shared column
end tell
```

### Direct parameter — the two sides, three forms (best → worst)

1. **Class constants** `{employee, manager}` — pure metadata (`typeType` codes), **zero
   pre-fetch**, canister owns the whole join. **Default choice.**
2. **`a reference to (…)` specifiers** — `{a reference to (employees of root whose active is true), …}` —
   *unresolved* object specifiers; use when you must **pre-qualify** a side. The bridge
   folds the embedded `whose` into one canister query. Must use `a reference to`, else…
3. ~~Bare specifiers~~ `{employees of root, managers of root}` — **trap**: AppleScript
   auto-dereferences them → eager `get` ×2, both tables hauled Mac-side *before* the
   join. Avoid.

### Scope — `in`, optional

The **application object is the always-available implicit root**; there is no built-in
`root`. A container-less specifier arrives as the **`typeNull` container** = "the whole
canister". So make `in` optional and default to that. Add an explicit `database "x"`
only if the canister hosts **multiple namespaces**.

### Key — `on <property>`, sent as a **property code**, never a string

- single: `on cubicle` → ships `«class cube»` (`typeType`); the canister maps the code → column.
- composite: `on {cubicle, age}` → list of codes → **tuple equi-join** (conjunction; order-free).
- asymmetric column names per side: pairs `on {{cubicle, deskId}, {age, seniority}}`
  or a record `on {left:{cubicle, age}, right:{deskId, seniority}}`.
- type the param as `type` **or — cleaner — an enumeration of join keys** (closed set:
  unambiguous, self-documenting, dodges the bare-token parser ambiguity).
- the literal "send a property code" escape hatch is the chevron: `on «class cube»` (always unambiguous).
- `missing value` in a key column → match SQL (drop the row), but document it.

### Filter — `where <test>`, pushed down

A command's result is *data*, so it can't take a postfix `whose`. Express the result
filter as the `where` parameter; the canister folds it into its query so the full
cross-product never materialises.

### Result

A list of records (record keys = property **codes** from the schema registry), **or** a
synthetic view class (below) when you want the result to compose with `whose`.

---

## Alternative shapes (all good — pick by situation)

- **Fixed VIEW class** (`staffing`) — a *pre-baked* special case of the headline: when a
  join is reached for constantly, bake its spec into a named class so no `id` is needed —
  `every staffing whose cubicle is "A3"` instead of `rows of (join id {…})`. Same
  composability and pushdown; the spec is implicit. (Deciding line for the verb vs either
  class form: a command's result can't be `whose`-filtered, a class's elements can.)

- **Relationship as navigation** — model `manager of employee` (property) and
  `employees of manager` (element); the join becomes navigation resolved app-side
  (Mail's `sender of message` philosophy):
  `get {name, name of its manager} of every employee`. Most idiomatic when the relation
  is fixed and one-to-one / one-to-many.

- **Indexed nested-loop, expressible in plain AppleScript *today*** — loop the small
  side, resolve each key to a constant, filter the other with `whose key = const`; the
  bridge turns each `whose` into a **pushed-down indexed canister query**. O(outer) round
  trips, each an index hit. Good stop-gap before a join verb exists, *if* the canister
  has an index on the key:
  ```applescript
  repeat with m in (every manager)
      set c to cubicle of m
      repeat with e in (every employee whose cubicle is c)   -- indexed canister query
          set end of joined to {employee:(name of e), manager:(name of m), cubicle:c}
      end repeat
  end repeat
  ```

---

## Composite keys

A key is an ordered **tuple** of column values, wearing two hats:
- **identity** (which one row): `row id {floor:1, cubicle:"A3"}` — `formUniqueID` + record.
- **join** (which rows match): the spec's `on {floor, cubicle}`.

**Worked example — cubicle numbers repeat per floor**, so `(floor, cubicle)` is the
natural composite key (cubicle alone over-joins):

```
employees                       managers
Alice  floor 1  cubicle A3      Dana  floor 1  cubicle A3
Bob    floor 2  cubicle A3      Erin  floor 2  cubicle A3
Carol  floor 1  cubicle B1
```

`on cubicle` would match Alice to **both** Dana and Erin (both "A3"). `on {floor,
cubicle}` pairs Alice→Dana, Bob→Erin (Carol drops, or `manager: missing value` under a
left join):

```applescript
rows of (join id {left: manager, right: employee, on: {floor, cubicle}})
-- ⇒ {{floor:1, cubicle:"A3", employee:"Alice", manager:"Dana"},
--    {floor:2, cubicle:"A3", employee:"Bob",   manager:"Erin"}}
```

**Encoding pitfall:** don't string-concat the components (`"A3"+"2"` collides with
`"A"+"32"`, and you lose types). Use a real tuple/struct key, or a length-prefixed /
order-preserving byte encoding — defined **once** as a value object, reused for both
identity and join.

**Canister synergy:** Candid has no key/index concept — indexing is the canister's job. A
composite key in a **BTree** (`StableBTreeMap<(K1,K2),V>`, order-preserving encoding)
gives point lookups *and* **prefix scans**: index on `(cubicle, age)` and `whose cubicle
is c` becomes a range `(c, MIN)..=(c, MAX)`. So one composite index serves identity
lookup, point joins, **and** the indexed-nested-loop prefix join — put the column you
probe on **first**.

---

## Where the join actually runs

- **Canister-side query method** (you own the canister): one event → one query → joined
  view. Cheapest. **Caveat:** query **response-size cap (~2 MiB)** + instruction limit —
  paginate / push the `where` filter down for big joins.
- **Bridge-side hash join** (`NSMutableDictionary`): fetch each side once, build + probe,
  O(n+m). For data across canisters or canisters you can't change. (Native AppleScript
  records **can't** be runtime-keyed — labels are compile-time literals — so the hash
  table must be `NSDictionary`, not a record.)

Either way the `.sdef` hides the engine: callers write `rows of (join id {…})`,
`every staffing`, or `join {…}` and never know which side did the work.

---

## The reflective superpower

Because the canister emits its own aete, it can **auto-generate join vocabulary from
schema metadata**: any two tables sharing a column → advertise a `staffing` view class
(and/or a specialised `join`). You declare the relationship **once in the schema**, and
it surfaces as AppleScript terms for free — the honest version of "a join operator".

---

## Dynamic-terminology gotchas (the real work)

1. **Stable 4-char codes (OSType).** Suite / event / parameter / property / enum codes
   must be **deterministic across regenerations** — compiled `.scpt`s store *codes*, not
   names; a shifting code silently breaks saved scripts. Mint from a **persistent
   registry** (in the canister — it owns the schema); never reuse a retired code.
2. **Terminology cache invalidation.** OSA aggressively caches an app's terminology.
   Either **snapshot the aete per connection** (simplest), or **version it** and define a
   reload path for live-mutable schemas (you then own cache coherence).
3. **Result-record keys are dynamic codes too** → from the same registry, so
   `cubicle of (item 1 of result)` resolves.

---

## Reference — object-specifier key forms (addressing elements)

| form | code | AppleScript | use |
|---|---|---|---|
| name | `name` | `employee "Alice"` | **text only** — never reaches a non-text operand |
| index | `indx` | `employee 1` | absolute position |
| unique id | `ID␣␣` | `employee id 42`, `employee id {cubicle:"A3", seat:2}` | **non-text identity — incl. a record for composite keys** |
| test | `test` | `employee whose cubicle is "A3" and seat is 2` | relational / composite lookup; **push to canister** |
| range | `rang` | `employees 1 thru 5` | homogeneous slice — **not** a join |
| relative | `rele` | `employee after x` | — |

Advertise per element the forms it supports. For **non-text identity** advertise `ID␣␣`
(id property typed to the real key — incl. a `reco` for composites) and `test`, and
**don't** advertise `name` → then `employee "x"` is a clean *compile-time* error.

## Reference — `a reference to` vs `alias`

- **`a reference to <specifier>`** — lazy object specifier to *any* scriptable object or
  variable; re-resolved on each access; not persistent. **The deferral mechanism for app
  objects** (what the bridge wants to receive *unresolved*).
- **`alias`** — a data type for *files/folders*: must exist at creation, self-heals
  across renames/moves (Alias Manager), persistent. **Not** an app-object deferral tool.
- (`file` / `POSIX file` — a file *specifier* that may not exist and doesn't track moves.)

## If you ever need a join *without* the bridge (plain AppleScript)

There is **no relational join** in the language. Options: a manual nested-`repeat`
(O(n·m), fine for a handful of rows); an **AppleScriptObjC `NSMutableDictionary` hash
join** (records can't be runtime-keyed, so use `NSDictionary`); or `do shell script` to
**`jq`** (`group_by` + index = a hash join), **`sqlite3`** (`json_each`), or **`join(1)`**.

---

## Blind alleys (don't revisit)

- `whose`/`where` as a join — filters **one** collection; a test clause references only
  `its`, can't correlate two.
- Custom infix operator / a correlated `where left = right` (incl.
  `join id {…} where left's x = right's x`) — grammar is closed, `left`/`right` can't
  bind, live correlation re-introduces n·m. Fold the condition into the spec's `on:`.
- `… thru …` for a join — `thru` is a **homogeneous range** (one class, comparable
  boundaries); a join is heterogeneous. Category error; class constants aren't valid
  boundaries anyway.
- `#byname` with a non-text name — `formName` is text-bound; use `id` / `whose`.
- Bare specifiers in the `join` list — auto-dereferenced → eager double-fetch.
- `on "cubicle"` (string key) — stringly-typed; send the property code / enum instead.

---

## Next (if resumed)

- **Resolver sketch:** decode the direct-param list (`typeType` vs `objectSpecifier`),
  extract any embedded `formTest`, map property codes via the registry, emit **one**
  canister join query (folding `where`).
- **Canister-side** generic `join(tableA, tableB, keys, filter, kind)` query method, and
  how it returns the dynamic-key record vec the bridge re-codes into `AERecord`s.
- **Decide** terminology policy: fixed-per-session (snapshot) vs live-mutable (versioned + reload).
