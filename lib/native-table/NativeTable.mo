/// Tables whose rows the native-table runtime keeps outside the garbage-collected heap.
///
/// A table maps keys of type K to rows of type R. K is a scalar (Nat, Int, Nat8 to Nat64,
/// Int8 to Int64, Float, Bool, Char, Text, Blob or Principal), or a tuple of 2 to 7 of them. R is
/// a scalar, or a record of immutable fields where each field is a scalar, an immutable array, or
/// an option of either. An array's elements are scalars, options of scalars, or records whose
/// fields are scalars or options of scalars.

import Prim "mo:⛔";

module {

  /// A table, with the field lists of its indexes and their numbers.
  public type NativeTable<K, R> = {
    id : Nat64;
    codec : Prim.NtCodec<(K, R)>;
    indexes : [([Text], Nat)]
  };

  /// A table not created yet, with the field lists of the indexes it will have, each with whether
  /// it is unique, and whether it removes rows left with every field null (`withoutNullRows`).
  public type Builder<K, R> = { codec : Prim.NtCodec<(K, R)>; indexes : [([Text], Bool)]; noNullRows : Bool };

  /// The value of one field of an index key: `#null_`, `#bool`, `#nat`, `#int`, `#float`,
  /// `#text`, `#blob` or `#principal`. A Nat or Int stands for a value of any integer type or Char
  /// in the field type's range; null for a field that is an option.
  public type Cell = Prim.NtCell;

  /// One end of a filter's range: a cell, included (`#incl`) or excluded (`#excl`).
  public type Bound = Prim.NtBound;

  /// What a filter asks of the value of one field: `#eq`, to equal the cell, which is `#null_` to
  /// ask for null on an option field; `#range { low; high }`, to lie within `low` and `high`, an
  /// end that is null being open. A null value lies in no range. A cell is of the field's kind, as
  /// in an index key.
  public type Filter = Prim.NtFilter;

  /// A scalar type a field of the row can have.
  public type Scalar = {
    #bool;
    #nat8;
    #nat16;
    #nat32;
    #nat64;
    #int8;
    #int16;
    #int32;
    #int64;
    #nat;
    #int;
    #float;
    #char;
    #text;
    #blob;
    #principal
  };

  /// The type of a field of the row: its scalar type, whether the field is an option, and for an
  /// array, whether its elements are options. `array` is null for a field that is not an array.
  /// For an array, `scalar` is its elements' scalar type; for an array of records, each field of
  /// the record is listed as `"field.sub"`, with that field's scalar type, and `array` says
  /// whether that field is an option.
  public type FieldType = { scalar : Scalar; optional : Bool; array : ?{ optional : Bool } };

  /// A builder of a new, empty table with no indexes: add indexes with `createIndex`, then
  /// create the table with `build`.
  public func empty<K, R>(codec : (implicit : Prim.NtCodec<(K, R)>)) : Builder<K, R> {
    { codec; indexes = []; noNullRows = false }
  };

  /// The builder with one more index, on the named fields of the row, in key order (1 to 7 of
  /// them). An index declared twice is one index.
  public func createIndex<K, R>(self : Builder<K, R>, fields : [Text]) : Builder<K, R> {
    declare(self, fields, false)
  };

  /// `createIndex`, of an index that holds each value for one row at most: `add`, `insert`,
  /// `update` and `upsert` trap, writing nothing, when they would give a second row the value
  /// of its fields another row has, and `tryAdd` returns false. A value with null in one of its
  /// fields is never refused. An index declared both ways is one index, unique.
  public func createUniqueIndex<K, R>(self : Builder<K, R>, fields : [Text]) : Builder<K, R> {
    declare(self, fields, true)
  };

  func declare<K, R>(self : Builder<K, R>, fields : [Text], unique : Bool) : Builder<K, R> {
    let n = self.indexes.size();
    {
      self with
      indexes = Prim.Array_tabulate<([Text], Bool)>(n + 1, func i = if i < n { self.indexes[i] } else { (fields, unique) })
    }
  };

  /// The builder of a table where `update` and `upsert` remove the row they leave with every field
  /// null, and an `upsert` of nulls only to a key with no row inserts nothing and returns false. The
  /// table keeps this through `clear` and `evolve`. A row with a field that is not an option is never
  /// removed.
  public func withoutNullRows<K, R>(self : Builder<K, R>) : Builder<K, R> {
    { self with noNullRows = true }
  };

  /// Creates the table with its indexes, so that each index holds every row from the start.
  /// Indexes cannot be added to the table afterwards.
  public func build<K, R>(self : Builder<K, R>) : NativeTable<K, R> {
    let (id, ids) = Prim.ntCreate(self.codec, self.indexes, self.noNullRows);
    // Index ids count up from 1 in the order of each field list's first declaration.
    let indexes = Prim.Array_init<([Text], Nat)>(ids.size(), ([], 0));
    var n = 0;
    for i in ids.keys() {
      if ids[i] > n {
        indexes[n] := (self.indexes[i].0, ids[i]);
        n += 1
      }
    };
    { id; codec = self.codec; indexes = Prim.Array_tabulate<([Text], Nat)>(n, func i = indexes[i]) }
  };

  /// The table under the row type `R2`, for an upgrade's migration function: the same table, with
  /// the same rows and indexes. `R2` may add fields of an option type, which the rows stored before
  /// read as null, and may drop fields; an index on a dropped field goes with it, and the other
  /// indexes stay as they are. It does not rename a field or change a field's type. Traps, changing
  /// nothing, when a field keeps its name but changes its type, when a new field is not an option,
  /// or when the old or the new row type is a scalar and the two differ. Its cost does not grow
  /// with the table: at most one row group of 4,096 rows is packed. The memory of a dropped field,
  /// and of an index that goes with it, is not given back. Once the row type has changed, use only
  /// the table this returns: a call on `self` traps, unless a later `evolve` returns the table to
  /// that row type.
  public func evolve<K, R1, R2>(self : NativeTable<K, R1>, codec : (implicit : Prim.NtCodec<(K, R2)>)) : NativeTable<K, R2> {
    ignore Prim.ntEvolve(self.id, self.codec, codec);
    let fields = fieldsOf(codec);
    func kept(index : [Text]) : Bool {
      for field in index.vals() {
        var found = false;
        for (name, _) in fields.vals() {
          if name == field { found := true }
        };
        if not found { return false }
      };
      true
    };
    let indexes = Prim.Array_init<([Text], Nat)>(self.indexes.size(), ([], 0));
    var n = 0;
    for index in self.indexes.vals() {
      if kept(index.0) {
        indexes[n] := index;
        n += 1
      }
    };
    { id = self.id; codec; indexes = Prim.Array_tabulate<([Text], Nat)>(n, func i = indexes[i]) }
  };

  /// The table with only the fields of the row type `P`, for reads: a view. `P` has some of the
  /// fields of `R`, each with the same name and type, and every read of the view reads only those
  /// fields of the row. Field names given to its calls can be any field of `R`, and `fieldTypes`
  /// lists the fields of `R`. `P` comes from the declared type, as for `evolve`; or give all three
  /// types: `view<K, R, P>()`. Traps when `P` has a field `R` lacks, or a field of another type.
  /// `remove` and `delete` on a view remove the whole row; `add`, `insert`, `update` and `evolve`
  /// on a view trap. After `evolve`, make views from the table it returns.
  public func view<K, R, P>(self : NativeTable<K, R>, codec : (implicit : Prim.NtCodec<(K, P)>)) : NativeTable<K, P> {
    { id = self.id; codec = Prim.ntView(self.codec, codec); indexes = self.indexes }
  };

  /// The number of rows.
  public func size<K, R>(self : NativeTable<K, R>) : Nat {
    Prim.ntSize(self.id, self.codec)
  };

  /// Whether the table has no rows.
  public func isEmpty<K, R>(self : NativeTable<K, R>) : Bool {
    size(self) == 0
  };

  /// Inserts the row under the key, or replaces the row the key has.
  public func add<K, R>(self : NativeTable<K, R>, key : K, row : R) {
    ignore Prim.ntPut(self.id, self.codec, key, row)
  };

  /// Inserts the row under the key, or replaces the row the key has. Returns true when the key
  /// is new to the table.
  public func insert<K, R>(self : NativeTable<K, R>, key : K, row : R) : Bool {
    Prim.ntPut(self.id, self.codec, key, row)
  };

  /// `add`, but returns false, writing nothing, when a unique index (`createUniqueIndex`) has
  /// the row's value for another row; else true.
  public func tryAdd<K, R>(self : NativeTable<K, R>, key : K, row : R) : Bool {
    Prim.ntTryPut(self.id, self.codec, key, row)
  };

  /// Sets the fields of the record `fields` in the row the key has, and keeps the row's other
  /// fields. Returns true when the key has a row; when not, writes nothing and returns false.
  /// `fields` has some of the fields of `R`, each of the same name and type, or a `Nat` for an
  /// `Int` field (`?Nat` for `?Int`), so `{ score = 5 }` needs no annotation; write `null` with
  /// its type, `{ note = null : ?Text }`. Only the indexes on fields whose value changes move.
  /// Traps when `fields` has a field `R` lacks, or a field of another type, and on a view.
  public func update<K, R, P>(self : NativeTable<K, R>, key : K, fields : P, codec : (implicit : Prim.NtCodec<(K, P)>)) : Bool {
    Prim.ntUpdate(self.id, self.codec, codec, key, fields)
  };

  /// Sets the fields of `fields` in the row the key has, as `update` does; when the key has no row,
  /// inserts one from them, with null in the row's other fields. Returns true when the key is new
  /// to the table. A field of `R` that `fields` leaves out must be an option, and a field of
  /// `fields` must be a field of `R` of the same type, or a `Nat` for an `Int` (`?Nat` for `?Int`):
  /// when not, the call does not compile. Traps on a view.
  public func upsert<K, R, P>(self : NativeTable<K, R>, key : K, fields : P, codec : (implicit : Prim.NtCodec<(K, P, R)>)) : Bool {
    Prim.ntUpsert(self.id, self.codec, codec, key, fields)
  };

  /// Adds the elements of each array of the record `fields` at the end of that array field of the
  /// row the key has, and keeps the row's other fields: `ignore docs.append(id, { log = [entry] })`.
  /// Returns true when the key has a row; when not, writes nothing and returns false. A null array
  /// field of the row counts as empty, and a null in `fields` adds nothing. Traps as `update` does,
  /// and when a field of `fields` is not an array field.
  public func append<K, R, P>(self : NativeTable<K, R>, key : K, fields : P, codec : (implicit : Prim.NtCodec<(K, P)>)) : Bool {
    Prim.ntAppend(self.id, self.codec, codec, key, fields)
  };

  /// The row the key has, or null.
  public func get<K, R>(self : NativeTable<K, R>, key : K) : ?R {
    Prim.ntGet(self.id, self.codec, key)
  };

  /// Whether the key has a row.
  public func containsKey<K, R>(self : NativeTable<K, R>, key : K) : Bool {
    switch get(self, key) {
      case null { false }
      case _ { true }
    }
  };

  /// Removes the row the key has, if any.
  public func remove<K, R>(self : NativeTable<K, R>, key : K) {
    ignore Prim.ntRemove(self.id, self.codec, key)
  };

  /// Removes the row the key has. Returns true when the key had a row.
  public func delete<K, R>(self : NativeTable<K, R>, key : K) : Bool {
    Prim.ntRemove(self.id, self.codec, key)
  };

  /// Removes every row, with its index entries: `size` is 0, and sums, counts, `min` and `max`
  /// start again from no rows. Its cost does not grow with the table. The memory the rows used
  /// goes back in steps of the table's upkeep: each write runs one, and `maintain` runs more. The
  /// indexes and the row type stay as they are.
  public func clear<K, R>(self : NativeTable<K, R>) {
    ignore Prim.ntClear(self.id, self.codec)
  };

  /// The rows a scan or a walk hands back per call to the table.
  let batch : Nat64 = 64;

  /// The values of `first` and of the batches `fetch` hands back after it, in order, from the
  /// place `from`, none when it is null. `fetch` gets the place to go on from, and returns a batch
  /// and the place after it, or null after the last batch.
  func batches<T, P>(first : [T], from : ?P, fetch : P -> ([T], ?P)) : { next : () -> ?T } {
    var values = first;
    var at = 0;
    var rest = from;
    object {
      public func next() : ?T {
        loop {
          if at < values.size() {
            at += 1;
            return ?values[at - 1]
          };
          switch rest {
            case null { return null }
            case ?from {
              let (more, after) = fetch(from);
              values := more;
              at := 0;
              rest := after
            }
          }
        }
      }
    }
  };

  /// The keys and rows of the table, in the table's own order: the order their keys were first
  /// added, not key order. `add` over a key the table has, `update` and `upsert` keep a row's
  /// place; a key removed and added again comes last. Every row that stays in the table while the
  /// iterator runs comes exactly once, also when other rows are added or removed in between.
  public func entries<K, R>(self : NativeTable<K, R>) : { next : () -> ?(K, R) } {
    batches<(K, R), Nat64>([], ?0, func from = Prim.ntScanEntries(self.id, self.codec, null, from, batch))
  };

  /// The keys of the table, in the order of `entries`. It reads no field of the rows.
  public func keys<K, R>(self : NativeTable<K, R>) : { next : () -> ?K } {
    batches<K, Nat64>([], ?0, func from = Prim.ntScanKeys(self.id, self.codec, from, batch))
  };

  /// The rows of the table, in the order of `entries`.
  public func values<K, R>(self : NativeTable<K, R>) : { next : () -> ?R } {
    batches<R, Nat64>([], ?0, func from = Prim.ntScanValues(self.id, self.codec, from, batch))
  };

  /// The keys and rows of the table newest first: the order of `entries` reversed. Every row that
  /// stays in the table while the iterator runs comes exactly once.
  public func reverseEntries<K, R>(self : NativeTable<K, R>) : { next : () -> ?(K, R) } {
    batches<(K, R), Nat64>([], ?0xFFFF_FFFF_FFFF_FFFF, func from = Prim.ntScanBack(self.id, self.codec, from, batch))
  };

  /// The scalar type of a type code in the table's layout.
  func scalarOf(code : Nat8) : Scalar {
    switch code {
      case 0x01 { #bool }
      case 0x02 { #nat8 }
      case 0x03 { #nat16 }
      case 0x04 { #nat32 }
      case 0x05 { #nat64 }
      case 0x06 { #int8 }
      case 0x07 { #int16 }
      case 0x08 { #int32 }
      case 0x09 { #int64 }
      case 0x0A { #nat }
      case 0x0B { #int }
      case 0x0D { #float }
      case 0x0E { #char }
      case 0x0F { #text }
      case 0x10 { #blob }
      case 0x11 { #principal }
      case _ { Prim.trap("native table: the layout has an unknown type code") }
    }
  };

  /// The fields of the row of `codec` and their types, in the order the table keeps them.
  func fieldsOf<T>(codec : Prim.NtCodec<T>) : [(Text, FieldType)] {
    // The layout, numbers little-endian: a 20-byte header that ends with the u32 number of
    // columns, then per column, the key first: its type codes, its u32 label hash, the u16 length
    // of its name and the name, which is empty for the key and for a row that is a scalar. The type
    // codes are a scalar's; a key's that is a tuple: 0x22, the u16 number of its parts, then each
    // part's; or an array's: 0x24, then its element's, a scalar's or a record's: 0x21, the u16
    // number of its fields, then each field as a column; each after the option code 0x20 when it
    // is an option
    let layout = Prim.blobToArray(Prim.ntCodecLayout(codec));
    func byte(at : Nat) : Nat = Prim.nat8ToNat(layout[at]);
    func u16(at : Nat) : Nat = byte(at) + byte(at + 1) * 0x100;
    // The scalar of the codes at `at`, whether it is an option, and the place after the codes
    func scalar(at : Nat) : (Scalar, Bool, Nat) {
      let optional = layout[at] == 0x20;
      let code = if optional { at + 1 } else { at };
      (scalarOf(layout[code]), optional, code + 1)
    };
    // The name after the label hash at `at`, and the place after the name
    func name(at : Nat) : (Text, Nat) {
      let length = u16(at + 4);
      let bytes = Prim.Array_tabulate<Nat8>(length, func i = layout[at + 6 + i]);
      switch Prim.decodeUtf8(Prim.arrayToBlob(bytes)) {
        case ?name { (name, at + 6 + length) }
        case null { Prim.trap("native table: the layout has a name that is not text") }
      }
    };
    let fields = Prim.Array_init<(Text, FieldType)>(layout.size(), ("", { scalar = #bool; optional = false; array = null }));
    var n = 0;
    func add(name : Text, type_ : FieldType) {
      fields[n] := (name, type_);
      n += 1
    };
    let columns = u16(16) + u16(18) * 0x1_0000;
    // The key, a scalar or a tuple, has no name
    var at = name(if layout[20] == 0x22 { 23 + u16(21) } else { 21 }).1;
    var c = 1;
    while c < columns {
      let optional = layout[at] == 0x20;
      let list = if optional { at + 1 } else { at };
      if layout[list] != 0x24 {
        let (scalar_, optional, after) = scalar(at);
        let (field, next) = name(after);
        if field != "" { add(field, { scalar = scalar_; optional; array = null }) };
        at := next
      } else if layout[list + 1] != 0x21 {
        let (scalar_, elements, after) = scalar(list + 1);
        let (field, next) = name(after);
        add(field, { scalar = scalar_; optional; array = ?{ optional = elements } });
        at := next
      } else {
        // A field of an array of records: each of the record's fields, as "field.sub"
        let count = u16(list + 2);
        var sub = list + 4;
        var i = 0;
        while i < count {
          sub := name(scalar(sub).2).1;
          i += 1
        };
        let (field, next) = name(sub);
        sub := list + 4;
        i := 0;
        while i < count {
          let (scalar_, elements, after) = scalar(sub);
          let (subName, nextSub) = name(after);
          add(field # "." # subName, { scalar = scalar_; optional; array = ?{ optional = elements } });
          sub := nextSub;
          i += 1
        };
        at := next
      };
      c += 1
    };
    Prim.Array_tabulate<(Text, FieldType)>(n, func i = fields[i])
  };

  /// The fields of the row and their types, in the order the table keeps them: every field of a
  /// record row, none when the row is a scalar. Their names are the ones `createIndex`, `sum`,
  /// `sumCount`, `avg`, `min`, `max`, `entriesWhere`, `countWhere` and `sumWhere` take. A field of
  /// an array of records is listed once per field of the record, as `"field.sub"`.
  public func fieldTypes<K, R>(self : NativeTable<K, R>) : [(Text, FieldType)] {
    fieldsOf(self.codec)
  };

  /// The number of rows whose fields under the index on `index` have the values of `key`, one
  /// cell per field of the index, in its order, or per each of its first fields only.
  public func count<K, R>(self : NativeTable<K, R>, index : [Text], key : [Cell]) : Nat {
    Prim.ntCount(self.id, self.codec, self.indexes, index, key)
  };

  /// The place a walk goes on from, null after its last batch.
  func rest(token : Blob) : ?Blob = if token.size() == 0 { null } else { ?token };

  /// The keys and rows of the index on `index`, in its order or backward, from the key `from` on,
  /// or from the first, to the key `to`, or to the last, from the `offset`-th of them on.
  func walk<K, R>(self : NativeTable<K, R>, index : [Text], reverse : Bool, from : ?[Cell], to : ?[Cell], offset : Nat) : {
    next : () -> ?(K, R)
  } {
    let (start, key) : (Nat64, [Cell]) = switch from {
      case null { (0, []) }
      case ?key { (2, key) }
    };
    // From the `offset`-th row: one of 2^61 or more, more rows than a table holds, ends the walk as
    // any past its last row. A Nat below 2^30 compares fastest.
    let nth : Nat64 = if offset < 0x4000_0000 or offset < 0x2000_0000_0000_0000 { Prim.natToNat64(offset) } else { 0x2000_0000_0000_0000 };
    let (first, token) = Prim.ntWalk(self.id, self.codec, self.indexes, index, reverse, start | nth << 2, key, to, "" : Blob, batch);
    batches<(K, R), Blob>(
      first,
      rest(token),
      func token {
        let (more, after) = Prim.ntWalk(self.id, self.codec, self.indexes, index, reverse, 0 : Nat64, [], to, token, batch);
        (more, rest(after))
      },
    )
  };

  /// The keys and rows of the table in the order of the index on `index`, keys rising, the rows
  /// of one key in the table's own order: from the key `from` on, or from the first, up to the
  /// key `to`, or to the last, both included. A key has one cell per field of the index, in its
  /// order. Rows whose fields under the index are null are in it too: null comes before every
  /// value. The iterator fetches 64 rows per call to the table.
  public func indexEntries<K, R>(self : NativeTable<K, R>, index : [Text], from : ?[Cell], to : ?[Cell]) : {
    next : () -> ?(K, R)
  } {
    walk(self, index, false, from, to, 0)
  };

  /// As `indexEntries`, from its `offset`-th row on, 0 its first. The rows before it are counted in
  /// the index, not read, so a page of a walk costs about the same at any offset.
  public func indexEntriesAt<K, R>(self : NativeTable<K, R>, index : [Text], from : ?[Cell], to : ?[Cell], offset : Nat) : {
    next : () -> ?(K, R)
  } {
    walk(self, index, false, from, to, offset)
  };

  /// As `indexEntries`, keys falling: from the key `from` down, or from the last, to the key `to`,
  /// or to the first, both included. The rows of one key still come in the table's own order.
  public func reverseIndexEntries<K, R>(self : NativeTable<K, R>, index : [Text], from : ?[Cell], to : ?[Cell]) : {
    next : () -> ?(K, R)
  } {
    walk(self, index, true, from, to, 0)
  };

  /// As `reverseIndexEntries`, from its `offset`-th row on, as `indexEntriesAt`.
  public func reverseIndexEntriesAt<K, R>(self : NativeTable<K, R>, index : [Text], from : ?[Cell], to : ?[Cell], offset : Nat) : {
    next : () -> ?(K, R)
  } {
    walk(self, index, true, from, to, offset)
  };

  /// Removes the rows `indexEntries(index, from, to)` gives, with their index entries, in one call;
  /// returns how many.
  public func removeIndexRange<K, R>(self : NativeTable<K, R>, index : [Text], from : ?[Cell], to : ?[Cell]) : Nat {
    Prim.ntRemoveRange(self.id, self.codec, self.indexes, index, from, to)
  };

  /// The first key and row in the order of the index on `index`, or backward, whose key's first
  /// field is not null; null when no row has one.
  func end<K, R>(self : NativeTable<K, R>, index : [Text], reverse : Bool) : ?(K, R) {
    let (rows, _) = Prim.ntWalk(self.id, self.codec, self.indexes, index, reverse, 1 : Nat64, [], null, "" : Blob, 1 : Nat64);
    if rows.size() == 0 { null } else { ?rows[0] }
  };

  /// The key and row with the smallest key under the index on `index` whose first field is not
  /// null, the first of them in the table's own order; null when no row has such a key.
  public func minEntry<K, R>(self : NativeTable<K, R>, index : [Text]) : ?(K, R) {
    end(self, index, false)
  };

  /// The key and row with the largest key under the index on `index` whose first field is not
  /// null, the first of them in the table's own order; null when no row has such a key.
  public func maxEntry<K, R>(self : NativeTable<K, R>, index : [Text]) : ?(K, R) {
    end(self, index, true)
  };

  /// Each distinct key of the index on `index` with the number of rows that have it, keys rising
  /// in the index's order, so a key whose first field is null comes first. A key has one cell per
  /// field of the index, in its order. The iterator fetches 64 keys per call to the table, and
  /// reads their counts then, so writes in between show in the keys still to come.
  public func groupCounts<K, R>(self : NativeTable<K, R>, index : [Text]) : {
    next : () -> ?([Cell], Nat)
  } {
    let (first, token) = Prim.ntGroupCounts(self.id, self.codec, self.indexes, index, "" : Blob, batch);
    batches<([Cell], Nat), Blob>(
      first,
      rest(token),
      func token {
        let (more, after) = Prim.ntGroupCounts(self.id, self.codec, self.indexes, index, token, batch);
        (more, rest(after))
      },
    )
  };

  /// The keys and rows of the table whose field `field` meets `filter`, in the table's own order,
  /// as `entries` gives them: every matching row that stays in the table while the iterator runs
  /// comes exactly once. The field can be of any type, with an index or without. The iterator
  /// fetches 64 matching rows per call to the table.
  public func entriesWhere<K, R>(self : NativeTable<K, R>, field : Text, filter : Filter) : {
    next : () -> ?(K, R)
  } {
    let (first, next) = Prim.ntScanEntries(self.id, self.codec, ?(field, filter), 0 : Nat64, batch);
    batches<(K, R), Nat64>(first, next, func from = Prim.ntScanEntries(self.id, self.codec, ?(field, filter), from, batch))
  };

  /// The number of rows whose field `field` meets `filter`.
  public func countWhere<K, R>(self : NativeTable<K, R>, field : Text, filter : Filter) : Nat {
    Prim.ntCountWhere(self.id, self.codec, field, filter)
  };

  /// The sum and the number of the non-null values of the field `sumField` over the rows whose
  /// field `field` meets `filter`, (0, 0) when there are none; null when `sumField` is not of an
  /// integer type.
  public func sumWhere<K, R>(self : NativeTable<K, R>, sumField : Text, field : Text, filter : Filter) : ?(Int, Nat) {
    Prim.ntSum(self.id, self.codec, sumField, ?(field, filter))
  };

  /// The sum of the non-null values of the named field over every row, 0 when there are none;
  /// null when the field is not of an integer type. The table keeps the sum up to date as rows
  /// are written, so this is one read at any table size.
  public func sum<K, R>(self : NativeTable<K, R>, column : Text) : ?Int {
    do ? { Prim.ntSum(self.id, self.codec, column, null)!.0 }
  };

  /// The sum and the number of the non-null values of the named field over every row, (0, 0) when
  /// there are none; null when the field is not of an integer type. One read at any table size,
  /// as for `sum`.
  public func sumCount<K, R>(self : NativeTable<K, R>, column : Text) : ?(Int, Nat) {
    Prim.ntSum(self.id, self.codec, column, null)
  };

  /// The average of the non-null values of the named field over every row, their sum over their
  /// number; null when there are none, or when the field is not of an integer type.
  public func avg<K, R>(self : NativeTable<K, R>, column : Text) : ?Float {
    switch Prim.ntSum(self.id, self.codec, column, null) {
      case ?(s, n) { if n == 0 { null } else { ?(Prim.intToFloat(s) / Prim.intToFloat(n)) } }
      case null { null }
    }
  };

  /// The smallest non-null value of the named field over every row; null when there is none, or
  /// when the field is not of an integer type. Each row group of the table keeps its smallest and
  /// largest value as rows are written, so this reads one value per row group, and the rows of a
  /// group whose smallest value a removal or a replacement took away.
  public func min<K, R>(self : NativeTable<K, R>, column : Text) : ?Int {
    Prim.ntExtreme(self.id, self.codec, column, false)
  };

  /// The largest non-null value of the named field over every row; null when there is none, or
  /// when the field is not of an integer type. Read as `min` reads the smallest.
  public func max<K, R>(self : NativeTable<K, R>, column : Text) : ?Int {
    Prim.ntExtreme(self.id, self.codec, column, true)
  };

  /// Runs the table's upkeep in whole steps. A step compacts one group of 4,096 rows that removals
  /// or replacements have left with many removed rows or changed fields, drops a group with no row
  /// left, gives back part of the memory of the rows `clear` removed, or gives back pages that
  /// removed rows left empty. Each write runs one step first when work is pending; this runs steps
  /// until the call has used about `budget` instructions, and can go past that by the last step it
  /// starts.
  /// Returns true when work is still pending. Call it, from a timer for example, to get the work
  /// done on a table that receives no writes.
  public func maintain<K, R>(self : NativeTable<K, R>, budget : Nat) : Bool {
    Prim.ntMaintain(self.id, self.codec, Prim.natToNat64(budget))
  }

}
