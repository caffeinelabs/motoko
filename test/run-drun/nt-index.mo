import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Index reads match a model, on indexes on a Text (also keys of hundreds of bytes and of a
// thousand), an option Nat with nulls, a Nat of a thousand values (many batches), Float, Char and
// option Text fields, an option Nat that is always null, and on several fields: a Bool and an
// option Nat, an Int past 64 and 128 bits and a Text, a Nat8 and an Int16, a Nat64, an Int64 and an
// Int8, a Blob and a Principal. For each index: its distinct keys with their numbers of rows; the
// count of each key, and of the first also with Int cells for the fields of unsigned types and
// Char, the index named by another list of the same fields; the count by its first field, and by
// its first two; walks forward and backward from a
// key, to a key, between two keys and between values no row has, and over every key of the
// indexes on a Text, on a Nat and on a field that is always null, each handing back the rows it
// covers exactly once, in key order, across batches; and the rows with the smallest and the
// largest key whose first field is not null. The table goes through inserts, upkeep,
// replacements and removals, inserts again and the removal of every row. A field list declared
// twice is one index. A walk of the groups resumed after writes goes on in order and hands back
// the keys the writes added past it. maintain has no work on an empty table. Unique indexes refuse
// a second row's value. A field the row does not have, an empty field list, fields the table has
// no index on, and keys with a cell of the wrong kind, the wrong number of cells, no cell or a cell
// out of range trap
actor {
  type Cell = NativeTable.Cell;
  type Row = {
    s : Text;
    on : ?Nat;
    a : Nat;
    f : Bool;
    c : Int;
    n8 : Nat8;
    i16 : Int16;
    fl : Float;
    ch : Char;
    ot : ?Text;
    n64 : Nat64;
    i64 : Int64;
    i8 : Int8;
    bl : Blob;
    p : Principal;
    z : ?Nat
  };

  transient let indexes : [[Text]] = [
    ["s"],
    ["on"],
    ["a"],
    ["f", "on"],
    ["c", "s"],
    ["n8", "i16"],
    ["fl"],
    ["ch"],
    ["ot"],
    ["n64", "i64", "i8"],
    ["bl", "p"],
    ["z"],
  ];

  // The table, with the index on s declared twice
  func table() : NativeTable.NativeTable<Nat, Row> {
    var b = NativeTable.empty<Nat, Row>();
    for index in indexes.values() { b := b.createIndex(index) };
    b.createIndex(["s"]).build()
  };

  let rows = table();

  let m = 3_000;

  // The rows the table should hold, by key
  let model = Prim.Array_init<?Row>(m, null);

  // A value from 0 to 2^64 - 1 that the key `k` and the step `s` pick
  func mix(k : Nat, s : Nat) : Nat {
    var x = Prim.natToNat64(k * 1_000_003 + s * 7_919 + 1);
    x := (x ^ (x >> 30)) *% 0xbf58_476d_1ce4_e5b9;
    x := (x ^ (x >> 27)) *% 0x94d0_49bb_1331_11eb;
    Prim.nat64ToNat(x ^ (x >> 31))
  };

  // A Text of `n` times `c`
  func repeat(c : Text, n : Nat) : Text {
    var t = "";
    var i = 0;
    while i < n {
      t #= c;
      i += 1
    };
    t
  };

  // Short words, words longer than 16 bytes, words of hundreds of bytes, the empty one, one with
  // a character of two bytes, and the largest, of 1,000 bytes
  transient let words : [Text] = [
    "apple",
    "banana",
    "cherry",
    "kiwi",
    "lemon",
    "a fruit with a long name",
    "a fruit with a longer name",
    "",
    "\u{e9}clair",
    repeat("x", 300),
    repeat("y", 700),
    repeat("x", 301),
    repeat("\u{e9}", 500),
  ];

  transient let blobs : [Blob] = ["", "\00", "\00\01", "\ff"];

  transient let principals : [Principal] = [
    Prim.principalOfBlob(""),
    Prim.principalOfBlob("\01"),
    Prim.principalOfBlob("\01\00"),
  ];

  func row(k : Nat, s : Nat) : Row {
    let r = mix(k, s);
    {
      s = words[r / 3 % words.size()];
      on = if r % 5 == 0 { null } else { ?(r / 5 % 40) };
      a = r / 7 % 1_000 * 1_000_003;
      f = r % 2 == 0;
      c = switch r / 11 % 6 {
        case 0 { -(2 ** 130) - r / 13 % 3 }
        case 1 { 2 ** 70 + r / 13 % 3 }
        case 2 { -(2 ** 64) }
        case _ { r / 13 % 9 - 4 }
      };
      n8 = Prim.natToNat8(r / 17 % 256);
      i16 = Prim.intToInt16(r / 19 % 7 * 10_000 - 32_768);
      fl = Prim.intToFloat(r / 23 % 41 - 20) / 4.0;
      ch = Prim.nat32ToChar(Prim.natToNat32(if r % 9 == 0 { 0x1F600 } else { 0x61 + r / 29 % 26 }));
      ot = if r / 31 % 4 == 0 { null } else { ?words[r / 37 % 5] };
      n64 = if r % 3 == 0 { 18_446_744_073_709_551_615 } else { Prim.natToNat64(r / 41 % 3) };
      i64 = if r % 4 == 0 { -9_223_372_036_854_775_808 } else if r % 4 == 1 {
        9_223_372_036_854_775_807
      } else { Prim.intToInt64(r / 43 % 3 - 1) };
      i8 = Prim.intToInt8(r / 47 % 5 * 60 - 128);
      bl = blobs[r / 53 % blobs.size()];
      p = principals[r / 59 % principals.size()];
      z = null
    }
  };

  // The key of a row under the index on `index`
  func keyOf(r : Row, index : [Text]) : [Cell] {
    func opt(v : ?Nat, c : Nat -> Cell) : Cell = switch v {
      case null { #null_ }
      case ?n { c(n) }
    };
    func cell(field : Text) : Cell = switch field {
      case "s" { #text(r.s) }
      case "on" { opt(r.on, func n = #nat(n)) }
      case "a" { #nat(r.a) }
      case "f" { #bool(r.f) }
      case "c" { #int(r.c) }
      case "n8" { #nat(Prim.nat8ToNat(r.n8)) }
      case "i16" { #int(Prim.int16ToInt(r.i16)) }
      case "fl" { #float(r.fl) }
      case "ch" { #nat(Prim.nat32ToNat(Prim.charToNat32(r.ch))) }
      case "ot" {
        switch r.ot {
          case null { #null_ }
          case ?t { #text(t) }
        }
      }
      case "n64" { #nat(Prim.nat64ToNat(r.n64)) }
      case "i64" { #int(Prim.int64ToInt(r.i64)) }
      case "i8" { #int(Prim.int8ToInt(r.i8)) }
      case "bl" { #blob(r.bl) }
      case "p" { #principal(r.p) }
      case "z" { opt(r.z, func n = #nat(n)) }
      case _ { Prim.trap("no such field") }
    };
    Prim.Array_tabulate<Cell>(index.size(), func i = cell(index[i]))
  };

  // The order of two cells of one kind, null first: -1, 0 or 1
  func cmp(x : Cell, y : Cell) : Int {
    switch (x, y) {
      case (#null_, #null_) { 0 }
      case (#null_, _) { -1 }
      case (_, #null_) { 1 }
      case (#nat(a), #nat(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#int(a), #int(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#float(a), #float(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#text(a), #text(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#blob(a), #blob(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#principal(a), #principal(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#bool(a), #bool(b)) { if a == b { 0 } else if b { -1 } else { 1 } }
      case _ { Prim.trap("cells of two kinds") }
    }
  };

  // The order of two keys of one index, field by field
  func order(x : [Cell], y : [Cell]) : Int {
    for i in x.keys() {
      let o = cmp(x[i], y[i]);
      if o != 0 { return o }
    };
    0
  };

  // The groups the table hands back for the index on `index`, in a new array
  func groups(index : [Text]) : [([Cell], Nat)] {
    let all = Prim.Array_init<([Cell], Nat)>(m + 1, ([], 0));
    var n = 0;
    for g in rows.groupCounts(index) {
      all[n] := g;
      n += 1
    };
    Prim.Array_tabulate<([Cell], Nat)>(n, func i = all[i])
  };

  // A key with an Int cell in place of each Nat cell
  func ints(key : [Cell]) : [Cell] = Prim.Array_tabulate<Cell>(
    key.size(),
    func i = switch key[i] {
      case #nat(n) { #int(n) }
      case c { c }
    },
  );

  // The number of answers for the index on `index` that are not the model's: keys out of order
  // or of no row, counts other than the model's or the index's, rows whose key is missing
  func wrongs(index : [Text], gs : [([Cell], Nat)]) : Nat {
    var wrong = 0;
    for i in gs.keys() {
      let (key, count) = gs[i];
      if key.size() != index.size() or count == 0 or rows.count(index, key) != count { wrong += 1 };
      if i > 0 and order(gs[i - 1].0, key) >= 0 { wrong += 1 }
    };
    // Each row's key found among the groups, by halving
    let tally = Prim.Array_init<Nat>(gs.size(), 0);
    for r in model.values() {
      switch r {
        case ?r {
          let key = keyOf(r, index);
          var lo = 0;
          var hi = gs.size();
          while lo < hi {
            let mid = (lo + hi) / 2;
            if order(gs[mid].0, key) < 0 { lo := mid + 1 } else { hi := mid }
          };
          if lo < gs.size() and order(gs[lo].0, key) == 0 { tally[lo] += 1 } else {
            wrong += 1
          }
        }
        case null {}
      }
    };
    for i in gs.keys() { if tally[i] != gs[i].1 { wrong += 1 } };
    // The rows of each first field and of each first two, counted by them alone: the groups that
    // start with them, which come together
    for p in [1, 2].values() {
      var i = 0;
      while p < index.size() and i < gs.size() {
        let first = Prim.Array_tabulate<Cell>(p, func j = gs[i].0[j]);
        var sum = 0;
        while i < gs.size() and order(first, gs[i].0) == 0 { sum += gs[i].1; i += 1 };
        if rows.count(index, first) != sum { wrong += 1 }
      }
    };
    wrong
  };

  // Prints, after `step`, the number of rows, the number of answers that are not the model's,
  // and the number of keys of each index
  func check(step : Text) {
    var wrong = 0;
    let keys = Prim.Array_init<Nat>(indexes.size(), 0);
    for i in indexes.keys() {
      let index = indexes[i];
      let gs = groups(index);
      wrong += wrongs(index, gs);
      // The first key counted with Int cells, the index named by another list of its fields
      if gs.size() > 0 and rows.count(Prim.Array_tabulate<Text>(index.size(), func j = index[j]), ints(gs[0].0)) != gs[0].1 {
        wrong += 1
      };
      keys[i] := gs.size()
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; keys = Prim.Array_tabulate<Nat>(keys.size(), func i = keys[i]) })
  };

  // The number of things wrong with the walk of the index on `index` from `from` to `to`,
  // backward when `reverse`: a row that is not the model's row of its key, a key that comes
  // twice, a key out of order or out of the bounds, and a number of rows other than the number
  // the groups `gs` give the keys within the bounds; and the number of rows walked
  func walked(index : [Text], gs : [([Cell], Nat)], from : ?[Cell], to : ?[Cell], reverse : Bool) : (Nat, Nat) {
    let dir : Int = if reverse { -1 } else { 1 };
    // Whether `key` lies from `from` on and up to `to`, in the walk's direction
    func within(key : [Cell]) : Bool {
      let after = switch from {
        case null { true }
        case ?f { order(key, f) * dir >= 0 }
      };
      let before = switch to {
        case null { true }
        case ?t { order(key, t) * dir <= 0 }
      };
      after and before
    };
    var want = 0;
    for (key, count) in gs.values() { if within(key) { want += count } };
    let walk = if reverse { rows.reverseIndexEntries(index, from, to) } else {
      rows.indexEntries(index, from, to)
    };
    let seen = Prim.Array_init<Bool>(m, false);
    var n = 0;
    var wrong = 0;
    var last : ?[Cell] = null;
    for (k, r) in walk {
      n += 1;
      let key = keyOf(r, index);
      if k >= m or seen[k] or model[k] != ?r or not within(key) { wrong += 1 } else {
        seen[k] := true
      };
      switch last {
        case ?l { if order(l, key) * dir > 0 { wrong += 1 } }
        case null {}
      };
      last := ?key
    };
    if n != want { wrong += 1 };
    (wrong, n)
  };

  // The number of things wrong with the walks of the index on `index` from `from` to `to`,
  // backward when `reverse`, from their `offset`-th row, at a few offsets: a row of the first
  // three that is not the whole walk's row there, or one too few or too many
  func skips(index : [Text], from : ?[Cell], to : ?[Cell], reverse : Bool) : Nat {
    func walk(offset : Nat) : { next : () -> ?(Nat, Row) } = if reverse {
      rows.reverseIndexEntriesAt(index, from, to, offset)
    } else { rows.indexEntriesAt(index, from, to, offset) };
    let keys = Prim.Array_init<Nat>(m, 0);
    var n = 0;
    let all = if reverse { rows.reverseIndexEntries(index, from, to) } else { rows.indexEntries(index, from, to) };
    for (k, _) in all { keys[n] := k; n += 1 };
    var wrong = 0;
    for offset in [0, 1, 2, 63, 64, 65, n / 2, Prim.abs(n - 1 : Int), n, n + 1, 2 ** 70].values() {
      var got = 0;
      label page for (k, _) in walk(offset) {
        if offset + got >= n or keys[offset + got] != k { wrong += 1 };
        got += 1;
        if got == 3 { break page }
      };
      if got < 3 and offset + got < n { wrong += 1 }
    };
    wrong
  };

  // The number of things wrong with the row with the smallest key whose first field is not null,
  // or the largest when `largest`: its key is not that key, it is not the model's row of its
  // key, or not the first row of the walk from its key; or there is a row and should be none, or
  // none and should be one
  func ends(index : [Text], gs : [([Cell], Nat)], largest : Bool) : Nat {
    var want : ?[Cell] = null;
    for (key, _) in gs.values() {
      switch (key[0], want) {
        case (#null_, _) {}
        case (_, null) { want := ?key }
        case (_, ?_) { if largest { want := ?key } }
      }
    };
    let got = if largest { rows.maxEntry(index) } else { rows.minEntry(index) };
    switch (got, want) {
      case (null, null) { 0 }
      case (?(k, r), ?key) {
        let first = if largest { rows.reverseIndexEntries(index, ?key, null).next() } else {
          rows.indexEntries(index, ?key, null).next()
        };
        if order(keyOf(r, index), key) != 0 or k >= m or model[k] != ?r or first != ?(k, r) { 1 } else { 0 }
      }
      case _ { 1 }
    }
  };

  // Prints the number of things wrong with the walks and the ends of every index, and the number
  // of rows walked
  public func walks() : async () {
    var wrong = 0;
    var rowsWalked = 0;
    func add((w, n) : (Nat, Nat)) {
      wrong += w;
      rowsWalked += n
    };
    for index in indexes.values() {
      let gs = groups(index);
      let n = gs.size();
      // Every key: of many batches, of long Texts, and all null
      if index == ["s"] or index == ["a"] or index == ["z"] { add(walked(index, gs, null, null, false)) };
      // From and to the first key and the last key, and between keys
      if n > 1 {
        let (first, lo, hi, last) = (gs[0].0, gs[n / 3].0, gs[n / 2].0, gs[n - 1].0);
        add(walked(index, gs, ?lo, ?hi, false));
        add(walked(index, gs, ?hi, ?lo, true));
        add(walked(index, gs, null, ?first, false));
        add(walked(index, gs, null, ?last, true));
        add(walked(index, gs, ?first, null, true));
        add(walked(index, gs, ?last, null, false));
        add(walked(index, gs, ?hi, ?lo, false));
        if index == ["a"] or index == ["c", "s"] or index == ["z"] or index == ["bl", "p"] {
          wrong += skips(index, null, null, false) + skips(index, null, null, true) + skips(index, ?lo, ?hi, false) + skips(index, ?hi, ?lo, true)
        }
      };
      wrong += ends(index, gs, false) + ends(index, gs, true)
    };
    // Bounds that are values no row has
    let a = ["a"];
    let gs = groups(a);
    add(walked(a, gs, ?[#nat(5_000_000)], ?[#nat(50_000_000)], false));
    add(walked(a, gs, ?[#nat(50_000_000)], ?[#nat(5_000_000)], true));
    Prim.debugPrint("walks " # debug_show { wrong; walked = rowsWalked })
  };

  func put(k : Nat, r : Row) {
    rows.add(k, r);
    model[k] := ?r
  };

  func drop(k : Nat) {
    rows.remove(k);
    model[k] := null
  };

  public func empty() : async () {
    check("empty");
    Prim.debugPrint(debug_show { maintain = rows.maintain(1_000_000) })
  };

  public func fill() : async () {
    var k = 0;
    while k < m {
      put(k, row(k, 1));
      k += 1
    };
    check("fill");
    Prim.debugPrint(debug_show rows.indexes);
    // The keys of the index on a Blob and a Principal, with their counts
    Prim.debugPrint(debug_show groups(["bl", "p"]))
  };

  // Runs the upkeep until no work is pending
  public func upkeep() : async () {
    while rows.maintain(100_000_000) {};
    check("upkeep")
  };

  // A third of the rows replaced by rows of other values, a fifth removed
  public func change() : async () {
    var k = 0;
    while k < m {
      if k % 5 == 0 { drop(k) } else if k % 3 == 1 { put(k, row(k, 2)) };
      k += 1
    };
    check("change")
  };

  // The removed rows inserted again, with other values
  public func refill() : async () {
    var k = 0;
    while k < m {
      if model[k] == null { put(k, row(k, 3)) };
      k += 1
    };
    check("refill")
  };

  // A walk of the groups of the index on a, stopped after 100 keys; writes that add keys before
  // and past it and remove rows; the walk resumed: its keys still rise, and it hands back the
  // keys added past every key there was, with their counts
  public func resume() : async () {
    let it = rows.groupCounts(["a"]);
    var last : [Cell] = [];
    var taken = 0;
    while taken < 100 {
      switch it.next() {
        case ?(key, _) { last := key }
        case null {}
      };
      taken += 1
    };
    var k = 0;
    while k < 300 {
      put(k, { row(k, 4) with a = if k % 2 == 0 { k % 3 } else { 2 ** 80 + k % 4 } });
      k += 1
    };
    var wrong = 0;
    var added = 0;
    for (key, count) in it {
      if order(last, key) >= 0 { wrong += 1 };
      last := key;
      switch key[0] {
        case #nat(a) {
          if a >= 2 ** 80 {
            added += 1;
            if count != rows.count(["a"], key) { wrong += 1 }
          }
        }
        case _ { wrong += 1 }
      }
    };
    Prim.debugPrint("resume " # debug_show { wrong; added });
    check("resumed")
  };

  public func clear() : async () {
    var k = 0;
    while k < m {
      drop(k);
      k += 1
    };
    check("clear")
  };

  // A unique index on email, and one on team and badge declared also as a plain index: a value a
  // row has, with no null field, is refused for another row, and a row keeps its own. tryAdd gives
  // false; add, update and upsert trap; neither writes. evolve keeps the indexes unique
  type Person = { email : ?Text; team : Text; badge : Nat };
  let people = NativeTable.empty<Nat, Person>().createUniqueIndex(["email"]).createIndex(["team", "badge"]).createUniqueIndex(["team", "badge"]).build();

  public func unique() : async () {
    people.add(1, { email = ?"a@x"; team = "red"; badge = 1 });
    people.add(2, { email = null; team = "red"; badge = 2 });
    let tried = [
      people.tryAdd(3, { email = ?"a@x"; team = "blue"; badge = 1 }),
      people.tryAdd(3, { email = ?"c@x"; team = "red"; badge = 2 }),
      people.tryAdd(1, { email = ?"a@x"; team = "red"; badge = 1 }),
      people.tryAdd(3, { email = null; team = "blue"; badge = 1 }),
      people.update(2, { email = ?"b@x" }),
    ];
    Prim.debugPrint(debug_show { tried; size = people.size(); three = people.get(3); indexes = people.indexes })
  };
  public func uniqueAdd() : async () { people.add(4, { email = ?"b@x"; team = "green"; badge = 1 }) };
  public func uniqueUpdate() : async () { ignore people.update(3, { team = "red"; badge = 1 }) };
  public func uniqueUpsert() : async () { ignore people.upsert(4, { email = ?"a@x"; team = "green"; badge = 1 }) };
  public func uniqueAfter() : async () {
    let p : NativeTable.NativeTable<Nat, { email : ?Text; team : Text; badge : Nat; note : ?Text }> = people.evolve();
    let tried = p.tryAdd(4, { email = ?"b@x"; team = "green"; badge = 1; note = null });
    Prim.debugPrint(debug_show { tried; size = p.size(); three = p.get(3) })
  };

  public func unknownField() : async () {
    ignore NativeTable.empty<Nat, Row>().createIndex(["nope"]).build()
  };
  public func noFields() : async () { ignore NativeTable.empty<Nat, Row>().createIndex([]).build() };
  // The fields of an index in another order are another index
  public func noIndex() : async () { ignore rows.count(["s", "c"], [#text("a"), #int(3)]) };
  public func wrongKind() : async () { ignore rows.count(["s"], [#nat(3)]) };
  public func wrongLength() : async () { ignore rows.reverseIndexEntries(["c", "s"], null, ?[#int(3)]) };
  public func noCells() : async () { ignore rows.count(["c", "s"], []) };
  public func tooLarge() : async () { ignore rows.count(["n8", "i16"], [#nat(256), #int(0)]) };
  public func surrogate() : async () { ignore rows.count(["ch"], [#nat(0xD800)]) }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress empty "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress refill "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress resume "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress unique "DIDL\x00\x00"
//CALL ingress uniqueAdd "DIDL\x00\x00"
//CALL ingress uniqueUpdate "DIDL\x00\x00"
//CALL ingress uniqueUpsert "DIDL\x00\x00"
//CALL ingress uniqueAfter "DIDL\x00\x00"
//CALL ingress unknownField "DIDL\x00\x00"
//CALL ingress noFields "DIDL\x00\x00"
//CALL ingress noIndex "DIDL\x00\x00"
//CALL ingress wrongKind "DIDL\x00\x00"
//CALL ingress wrongLength "DIDL\x00\x00"
//CALL ingress noCells "DIDL\x00\x00"
//CALL ingress tooLarge "DIDL\x00\x00"
//CALL ingress surrogate "DIDL\x00\x00"
