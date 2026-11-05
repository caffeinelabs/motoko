import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Group counts match a model: each distinct key of an index with its number of rows, in the
// index's order with null first, for indexes on a Text (also keys of hundreds of bytes), an
// option Nat with nulls, a Nat of a thousand values (many batches), Float, Char and option Text
// fields, and on several fields: a Bool and an option Nat, an Int past 64 and 128 bits and a
// Text, a Nat8 and an Int16, a Nat64, an Int64 and an Int8, a Blob and a Principal. The table
// goes through inserts, upkeep, replacements and removals, inserts again and the removal of
// every row. Each key's count is also the index's count of it. A walk of the groups resumed after
// writes goes on in order and hands back the keys the writes added past it. A field list the
// table has no index on traps
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
    p : Principal
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
  ];

  func table() : NativeTable.NativeTable<Nat, Row> {
    var b = NativeTable.empty<Nat, Row>();
    for index in indexes.values() { b := b.createIndex(index) };
    b.build()
  };

  let rows = table();

  let m = 6_000;

  // The rows the table should hold, by key
  let model = Prim.Array_init<?Row>(m, null);

  // A value from 0 to 2^64 - 1 that the key `k` and the step `s` pick
  func mix(k : Nat, s : Nat) : Nat {
    var x = Prim.natToNat64(k * 1_000_003 + s * 7_919 + 1);
    x := (x ^ (x >> 30)) *% 0xbf58_476d_1ce4_e5b9;
    x := (x ^ (x >> 27)) *% 0x94d0_49bb_1331_11eb;
    Prim.nat64ToNat(x ^ (x >> 31))
  };

  // A Text of `n` bytes, all `c`
  func repeat(c : Text, n : Nat) : Text {
    var t = "";
    var i = 0;
    while i < n {
      t #= c;
      i += 1
    };
    t
  };

  // Short words, words longer than 16 bytes, words of hundreds of bytes, the empty one and one
  // with a character of two bytes
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
      p = principals[r / 59 % principals.size()]
    }
  };

  // The key of a row under the index on `index`
  func keyOf(r : Row, index : [Text]) : [Cell] {
    func cell(field : Text) : Cell = switch field {
      case "s" { #text(r.s) }
      case "on" {
        switch r.on {
          case null { #null_ }
          case ?n { #nat(n) }
        }
      }
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
    var i = 0;
    while i < x.size() {
      let o = cmp(x[i], y[i]);
      if o != 0 { return o };
      i += 1
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

  // The number of answers for the index on `index` that are not the model's: keys out of order
  // or of no row, counts other than the model's or the index's, rows whose key is missing
  func wrongs(index : [Text], gs : [([Cell], Nat)]) : Nat {
    var wrong = 0;
    var i = 0;
    while i < gs.size() {
      let (key, count) = gs[i];
      if key.size() != index.size() or count == 0 or rows.count(index, key) != count { wrong += 1 };
      if i > 0 and order(gs[i - 1].0, key) >= 0 { wrong += 1 };
      i += 1
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
    i := 0;
    while i < gs.size() {
      if tally[i] != gs[i].1 { wrong += 1 };
      i += 1
    };
    wrong
  };

  // Prints, after `step`, the number of rows, the number of answers that are not the model's,
  // and the number of keys of each index
  func check(step : Text) {
    var wrong = 0;
    let keys = Prim.Array_init<Nat>(indexes.size(), 0);
    var i = 0;
    for index in indexes.values() {
      let gs = groups(index);
      wrong += wrongs(index, gs);
      keys[i] := gs.size();
      i += 1
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; keys = Prim.Array_tabulate<Nat>(keys.size(), func i = keys[i]) })
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
    check("empty")
  };

  public func fill() : async () {
    var k = 0;
    while k < m {
      put(k, row(k, 1));
      k += 1
    };
    check("fill");
    // The keys of the index on a Blob and a Principal, and those of a Bool and an option Nat
    // whose Nat is below 3, with their counts
    Prim.debugPrint(debug_show groups(["bl", "p"]));
    for (key, count) in rows.groupCounts(["f", "on"]) {
      switch key[1] {
        case #nat(n) { if n < 3 { Prim.debugPrint(debug_show (key, count)) } }
        case _ { Prim.debugPrint(debug_show (key, count)) }
      }
    }
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

  public func noIndex() : async () { ignore rows.groupCounts(["s", "a"]) };
  public func noField() : async () { ignore rows.groupCounts(["nope"]) };
  public func plain() : async () {
    ignore NativeTable.empty<Nat, Row>().build().groupCounts(["a"])
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress empty "DIDL\x00\x00"
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress refill "DIDL\x00\x00"
//CALL ingress resume "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress noIndex "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress plain "DIDL\x00\x00"
