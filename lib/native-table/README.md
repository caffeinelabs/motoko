# NativeTable

A `NativeTable<K, R>` maps keys of type `K` to rows of type `R`. Its rows live in the table's own
pages, outside the garbage-collected heap, and survive upgrades with the actor.

- `K`: a scalar: `Nat`, `Int`, `Nat8`–`Nat64`, `Int8`–`Int64`, `Float`, `Bool`, `Char`, `Text`,
  `Blob` or `Principal`.
- `R`: a scalar, or a record of immutable fields, each a scalar or an option of one.

The library is not part of the compiler's releases: it comes from a checkout of the Motoko
repository, in the folder `lib/native-table`. Give that folder to the compiler as the package
`native-table`, with `moc --package native-table <path to lib/native-table>`, and import it:

```motoko no-repl
import NativeTable "mo:native-table/NativeTable"
```

- Native tables need the native compiler, `moc`. `moc.js` rejects a program that uses them with
  error M0280.
- Tables run in compiled code only. The interpreter, `moc -r`, traps on a table call with
  `native tables need compiled code`.
- A table is never freed: it keeps its memory while the canister lives, also when nothing refers to
  it any more. There is no call that drops a table.
- Tables live in their own part of Wasm memory, taken in steps of 64 MiB: the first table takes
  64 MiB at once, and the tables grow by 64 MiB whenever they fill what they have.
- Tables survive upgrades that keep main memory. An upgrade by graph copy traps while a table exists
  (see Traps).
- A stable table's key type cannot change in an upgrade. Its row type changes through `evolve` in the
  upgrade's migration function (see Change the row type at upgrade).

## Create

Indexes are declared when the table is built and cannot be added later.

```motoko no-repl
type Order = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; note : ?Text };

let orders = NativeTable.empty<Nat, Order>()
  .createIndex(["city"])
  .createIndex(["amount"])
  .createIndex(["city", "placed"]) // several fields: ordered by city, then by placed
  .build();

let plain = NativeTable.empty<Text, Nat>().build() // no indexes
```

- An index has 1 to 7 fields; a table has at most 63 indexes.
- Declaring the same field list twice gives one index.
- Every index holds every row from the start, and every write keeps it current.

## Write

| Call | Does |
|---|---|
| `t.add(k, r)` | inserts `r` under `k`, or replaces the row `k` has |
| `t.insert(k, r) : Bool` | as `add`; true when `k` was new |
| `t.remove(k)` | removes the row `k` has, if any |
| `t.delete(k) : Bool` | as `remove`; true when `k` had a row |

Change one field:

```motoko no-repl
switch t.get(k) {
  case ?r { t.add(k, { r with paid = true }) }
  case null {}
}
```

## Read by key

| Call | Gives |
|---|---|
| `t.get(k) : ?R` | the row `k` has, or null |
| `t.containsKey(k) : Bool` | whether `k` has a row |
| `t.size() : Nat`, `t.isEmpty() : Bool` | the number of rows |

## Scan

| Call | Gives |
|---|---|
| `t.entries()` | every `(K, R)` |
| `t.keys()`, `t.values()` | every key, every row |

Scans go in the table's own order, not key order. A row that stays in the table while a scan runs
comes exactly once, even when other rows are added or removed in between. For key order, walk an
index.

## Read through an index

An index is named by its field list, as declared. A key has one cell per field of the index, in
its order:

| Cell | For fields of type |
|---|---|
| `#nat(n)` | `Nat`, `Nat8`–`Nat64`, `Char` |
| `#int(i)` | `Int`, `Int8`–`Int64` |
| `#float(f)`, `#bool(b)`, `#text(t)`, `#blob(b)`, `#principal(p)` | the same type |
| `#null_` | an option field that is null |

| Call | Gives |
|---|---|
| `t.count(index, key) : Nat` | the number of rows with that key |
| `t.indexEntries(index, from, to)` | the `(K, R)` from key `from` to key `to`, both included, smallest first; `null` for an open end |
| `t.reverseIndexEntries(index, from, to)` | the same, largest first: from `from` down to `to` |
| `t.minEntry(index) : ?(K, R)` | the row with the smallest key whose first field is not null |
| `t.maxEntry(index) : ?(K, R)` | the row with the largest such key |
| `t.groupCounts(index)` | each distinct key with its number of rows, as `([Cell], Nat)`, smallest key first |

```motoko no-repl
let zurich = orders.count(["city"], [#text("Zurich")]);

for (id, o) in orders.indexEntries(["amount"], ?[#nat(100)], ?[#nat(500)]) { /* … */ };

let biggest = orders.maxEntry(["amount"]);

for (key, n) in orders.groupCounts(["city"]) { /* … */ }; // ([#text("Basel")], 12), ([#text("Bern")], 7), …

// Zurich orders from the last 7 days, newest first
for (id, o) in orders.reverseIndexEntries(["city", "placed"], ?[#text("Zurich"), #int(now)], ?[#text("Zurich"), #int(now - week)]) { /* … */ }
```

- `count` takes a full key: one cell for every field of the index.
- Rows whose indexed field is null are in the index; null comes before every value.
- Rows with equal keys come in the table's own order.
- A walk fetches 64 rows per call to the table, `groupCounts` 64 keys.

## Filters

A filter asks something of the value of one field, of any type, with an index or without:

```motoko no-repl
type Bound = { #incl : Cell; #excl : Cell };
type Filter = { #eq : Cell; #range : { low : ?Bound; high : ?Bound } }
```

- `#eq(cell)`: the value equals the cell. `#eq(#null_)` asks for null on an option field.
- `#range { low; high }`: the value lies within both ends, each included (`#incl`), excluded
  (`#excl`) or open (`null`). A null value lies in no range.
- A cell is of the field's kind, as in an index key.

| Call | Gives |
|---|---|
| `t.entriesWhere(field, filter)` | the `(K, R)` whose field meets the filter, in the table's own order |
| `t.countWhere(field, filter) : Nat` | their number |
| `t.sumWhere(sumField, field, filter) : ?(Int, Nat)` | the sum and the number of the non-null values of `sumField` over those rows, `(0, 0)` when there are none; null when `sumField` is not of an integer type |

```motoko no-repl
let unpaid = orders.countWhere("paid", #eq(#bool(false)));

for (id, o) in orders.entriesWhere("placed", #range { low = ?#incl(#int(now - week)); high = null }) { /* … */ };

let zurich = orders.sumWhere("amount", "city", #eq(#text("Zurich"))) // ?(sum, rows with an amount)
```

- Each reads the table's rows. On an integer field, a row group whose smallest and largest value
  the filter rules out is skipped, and one whose values all meet it is counted without reading
  its rows.
- `entriesWhere` fetches 64 matching rows per call to the table. As in a scan, a row that stays in
  the table while it runs comes exactly once.

## Totals

Integer fields only: `Nat`, `Int`, `Nat8`–`Nat64`, `Int8`–`Int64`, and options of them, with an
index or without. The table keeps the totals current on every write, so `sum`, `sumCount` and
`avg` are one read at any table size. It also keeps each row group's smallest and largest value,
so `min` and `max` read one value per group of 4,096 rows.

| Call | Gives |
|---|---|
| `t.sum(field) : ?Int` | the sum of the non-null values; 0 when there are none |
| `t.sumCount(field) : ?(Int, Nat)` | the sum and the number of non-null values |
| `t.avg(field) : ?Float` | their average; null when there are none |
| `t.min(field) : ?Int` | the smallest non-null value; null when there is none |
| `t.max(field) : ?Int` | the largest non-null value; null when there is none |

Each is null for a field of any other type. After a removal or a replacement takes away a group's
smallest or largest value, `min` or `max` reads that group's rows until a write brings the group
an exact one again.

## Other

| Call | Does |
|---|---|
| `t.fieldTypes() : [(Text, FieldType)]` | the row's fields and their types: `{ scalar; optional }` |
| `t.maintain(budget) : Bool` | runs whole steps of the table's upkeep until about `budget` instructions are used; true while work is pending |

A step of upkeep compacts one group of 4,096 rows that removals or replacements have left with
many removed rows or changed fields, or drops a group with no row left; `maintain` can go past
`budget` by the step it started last. Each write already runs one step when work is pending. Call
`maintain`, from a timer for example, for a table that receives no writes.

## Change the row type at upgrade

With enhanced migration (`moc --enhanced-migration ./migrations`), the table is made by a migration,
and a later migration calls `evolve` on it. `evolve` returns the same table, with the same rows and
indexes, under the new row type.

The first migration makes the table. Orders have a `note`:

```motoko no-repl
// migrations/20261101_000000_Orders.mo
import NativeTable "mo:native-table/NativeTable";

module {
  type Order = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; note : ?Text };

  public func migration(_ : {}) : { orders : NativeTable.NativeTable<Nat, Order> } {
    { orders = NativeTable.empty<Nat, Order>().createIndex(["city"]).createIndex(["amount"]).build() }
  }
}
```

A later migration drops `note` and adds `coupon`:

```motoko no-repl
// migrations/20261110_000000_Coupon.mo
import NativeTable "mo:native-table/NativeTable";

module {
  type OldOrder = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; note : ?Text };
  type Order = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; coupon : ?Text };

  public func migration(old : { orders : NativeTable.NativeTable<Nat, OldOrder> })
    : { orders : NativeTable.NativeTable<Nat, Order> } {
      { orders = old.orders.evolve() } // drops note, adds coupon
    }
}
```

The actor declares the table without an initializer:

```motoko no-repl
// src/main.mo
import NativeTable "mo:native-table/NativeTable";

actor {
  type Order = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; coupon : ?Text };

  let orders : NativeTable.NativeTable<Nat, Order>;

  public query func order(id : Nat) : async ?Order {
    orders.get(id) // an order stored before the upgrade has coupon = null
  }
}
```

- `evolve` may add fields of an option type. Rows stored before read them as null.
- It may drop fields. An index on a dropped field goes with it; the other indexes stay as they are.
- Memory a dropped field used is not given back: an index that goes with it keeps its memory, and
  the field's values stay in the row groups packed before, until a group is rewritten (for example
  once 1,024 of its rows are removed).
- `evolve` only adds and drops fields: it does not rename a field or change a field's type.
- It traps, changing nothing, when a field keeps its name but changes its type, when a new field is
  not an option, or when the row type is a scalar or becomes one. The key type cannot change.
- `evolve` takes the new row type from the migration's declared result type, as in the example:
  `let t = old.orders.evolve()` alone does not compile. To write the types out, give all three:
  `old.orders.evolve<Nat, OldOrder, Order>()`.
- Its cost depends on the number of fields, and it packs at most one group of 4,096 rows written
  since the last pack: it does not grow with the table.
- Indexes cannot be added at upgrade.
- After `evolve`, use only the table it returns.
- Without `evolve`, a change of the row type does not compile.

## Traps

Each message starts with `native table: `.

| Message | Raised by | When |
|---|---|---|
| `the row type has no field of that name` | `build`; `sum`, `sumCount`, `avg`, `min`, `max`, `entriesWhere`, `countWhere`, `sumWhere` | a field name that the row lacks: in a field list given to `createIndex`, or given to the call |
| `an index has 1 to 7 fields` | `build` | a field list given to `createIndex` that is empty or has more than 7 fields |
| `a table has at most 63 indexes` | `build` | a 64th distinct field list given to `createIndex` |
| `no index on these fields` | `count`, `indexEntries`, `reverseIndexEntries`, `minEntry`, `maxEntry`, `groupCounts` | a field list the table was not built with an index on |
| `index key does not match the index's fields` | the index reads; `entriesWhere`, `countWhere`, `sumWhere` | a key with the wrong number of cells, or a cell of the wrong kind; in a filter, a cell of the wrong kind, `#null_` for a field that is not an option, or `#null_` as a range end |
| `index key value out of range` | the index reads; `entriesWhere`, `countWhere`, `sumWhere` | a cell, of a key or a filter, outside its field's range, for example `#nat(300)` for a `Nat8` |
| `a field changes its type` | `evolve` | a field keeps its name but changes its type |
| `a new field is not an option` | `evolve` | a field the old row type lacks is not of an option type |
| `a row that is a scalar cannot change` | `evolve` | the old or the new row type is a scalar, and the two differ |
| `not a table of this key and row type` | every call on a table | a handle that does not name a table this program made with these key and row types, for example a record built by hand, or a table value from before an `evolve` that changed its row type |
| `graph-copy stabilization is not possible while a table exists` | `__motoko_stabilize_before_upgrade` | the start of an upgrade by graph copy, once the canister has made a table |

## With OQL

A native table serves OQL queries through OQL's `NativeEntity` adapter, which needs OQL's support for
native tables; it is not part of this library, and the example below is not compiled here. The row
needs an `id` field with its own index, and the adapter reads the table's indexes:

```motoko no-repl
let orders = NativeTable.empty<Nat, Order>().createIndex(["id"]).createIndex(["city"]).build();

include Expose({
  entities = [
    orders.entity("order", "Order", "id").edge("customer", "customer").public_().build(),
    customers.entityWith("customer", "Customer", "id").public_().build(),
  ]
})
```

One JSON query can then join native tables and OQL tables.
