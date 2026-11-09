import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// Filtered reads and reads as cells match a model. The filters are equalities, ranges closed,
// half-open, open and empty, with included and excluded ends, and null asked of option fields, on
// fields of type Nat, Int (also past 64, 128 and 512 bits), Nat8, Int16, ?Int, Text, ?Text, Bool,
// Float and Char, none of them indexed. Each filter gives its rows, their number, the sum and
// count of an integer field over them, and the same rows as cells: the key's cell or not, then
// the cells of fields of every scalar type named in any order, a field named twice, or none.
// Every row is also read as cells with no filter. The table goes through inserts in key order,
// upkeep that seals its row groups, replacements and removals, inserts again and the removal of
// every row. A scan and a read as cells resumed after writes hand back each matching row that
// stayed exactly once. A name the row does not have, a cell of the wrong kind or out of its
// field's range, null asked of a field that is not an option, and a null end of a range trap
actor {
  type Cell = NativeTable.Cell;
  type Filter = NativeTable.Filter;
  type Bound = NativeTable.Bound;
  type Row = {
    a : Nat;
    b : ?Int;
    c : Int;
    nb : Nat;
    s : Text;
    f : Bool;
    n8 : Nat8;
    i16 : Int16;
    n64 : Nat64;
    i64 : Int64;
    ot : ?Text;
    fl : Float;
    ch : Char;
    bl : Blob;
    p : Principal
  };

  let rows = NativeTable.empty<Nat, Row>().build();

  let m = 9_000;

  // The rows the table should hold, by key
  let model = Prim.Array_init<?Row>(m, null);

  // A value from 0 to 2^64 - 1 that the key `k` and the step `s` pick
  func mix(k : Nat, s : Nat) : Nat {
    var x = Prim.natToNat64(k * 1_000_003 + s * 7_919 + 1);
    x := (x ^ (x >> 30)) *% 0xbf58_476d_1ce4_e5b9;
    x := (x ^ (x >> 27)) *% 0x94d0_49bb_1331_11eb;
    Prim.nat64ToNat(x ^ (x >> 31))
  };

  // A Text of 300 bytes
  func long() : Text {
    var t = "";
    var i = 0;
    while i < 300 {
      t #= "x";
      i += 1
    };
    t
  };

  // Short words, words longer than 16 bytes, one of 300 bytes, the empty one and one with a
  // character of two bytes
  transient let words : [Text] = [
    "apple",
    "banana",
    "cherry",
    "kiwi",
    "lemon",
    "mango",
    "pear",
    "plum",
    "a fruit with a long name",
    "a fruit with a longer name",
    "zucchini",
    "",
    "\u{e9}clair",
    long(),
  ];

  transient let blobs : [Blob] = [
    "",
    "\01\02\03",
    Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(100, func i = Prim.natToNat8(i))),
  ];

  transient let principals : [Principal] = [
    Prim.principalOfBlob("\00"),
    Prim.principalOfBlob("\01\02\03\04\05\06\07\08\09\0a"),
    Prim.principalOfBlob("\de\ad\be\ef\de\ad\be\ef\de\ad\be\ef\de\ad\be\ef\de\ad\be\ef\de\ad\be\ef\de\ad\be\ef\01"),
  ];

  // The row of key `k` at step `s`: on the first step, field a is the key, so each row group
  // holds a run of its values
  func row(k : Nat, s : Nat) : Row {
    let r = mix(k, s);
    let w = words.size();
    {
      a = if s == 1 { k } else { r % 20_000 };
      b = if r % 4 == 0 { null } else { ?(r / 4 % 2_001 - 1_000) };
      c = if r % 53 == 0 { (r - 2 ** 63) * 2 ** 500 } else if r % 59 == 0 {
        (r - 2 ** 63) * 2 ** 70
      } else { r % 100_001 - 50_000 };
      nb = if r % 61 == 0 { r * 2 ** 600 } else { r };
      s = words[r / 7 % w];
      f = r % 3 == 0;
      n8 = Prim.natToNat8(r % 256);
      i16 = Prim.intToInt16(r % 65_536 - 32_768);
      n64 = Prim.natToNat64(r);
      i64 = Prim.intToInt64(r % 2 ** 64 - 2 ** 63);
      ot = if r / 11 % 3 == 0 { null } else { ?words[r / 13 % w] };
      fl = Prim.intToFloat(r / 17 % 2_001 - 1_000) / 8.0;
      ch = Prim.nat32ToChar(Prim.natToNat32(0x61 + r / 19 % 26));
      bl = blobs[r / 23 % blobs.size()];
      p = principals[r / 29 % principals.size()]
    }
  };

  func incl(c : Cell) : ?Bound = ?#incl(c);
  func excl(c : Cell) : ?Bound = ?#excl(c);

  transient let filters : [(Text, Filter)] = [
    ("a", #eq(#nat(1_234))),
    ("a", #range { low = incl(#nat(2_000)); high = excl(#nat(5_000)) }),
    ("a", #range { low = excl(#nat(8_500)); high = null }),
    ("a", #range { low = null; high = incl(#nat(100)) }),
    ("a", #range { low = null; high = null }),
    ("a", #range { low = incl(#nat(5_000)); high = incl(#nat(4_000)) }),
    ("b", #eq(#null_)),
    ("b", #eq(#int(-7))),
    ("b", #range { low = incl(#int(-100)); high = incl(#int(100)) }),
    ("b", #range { low = excl(#int(0)); high = null }),
    ("c", #range { low = incl(#int(2 ** 64)); high = null }),
    ("c", #range { low = excl(#int(-(2 ** 100))); high = excl(#int(0)) }),
    ("c", #range { low = incl(#int(-(2 ** 140))); high = incl(#int(2 ** 140)) }),
    ("c", #eq(#int(0))),
    ("s", #eq(#text("pear"))),
    ("s", #range { low = incl(#text("b")); high = excl(#text("m")) }),
    ("s", #range { low = excl(#text("a fruit with a long name")); high = null }),
    ("s", #eq(#text(""))),
    ("f", #eq(#bool(true))),
    ("f", #range { low = excl(#bool(false)); high = null }),
    ("n8", #range { low = incl(#nat(200)); high = incl(#nat(255)) }),
    ("i16", #range { low = incl(#int(-32_768)); high = excl(#int(-1_000)) }),
    ("ot", #eq(#null_)),
    ("ot", #range { low = incl(#text("apple")); high = incl(#text("kiwi")) }),
    ("fl", #range { low = excl(#float(-10.5)); high = incl(#float(20.0)) }),
    ("fl", #eq(#float(0.0))),
    ("ch", #range { low = incl(#nat(0x61)); high = excl(#nat(0x66)) }),
  ];

  // The fields whose sums are checked, each with each filter in turn, and the integer fields
  transient let sums = ["a", "b", "c", "n8", "i16"];
  transient let integers = ["a", "b", "c", "nb", "n8", "i16", "n64", "i64"];

  // The field lists read as cells, each with each filter in turn
  transient let every = ["a", "b", "c", "nb", "s", "f", "n8", "i16", "n64", "i64", "ot", "fl", "ch", "bl", "p"];
  transient let lists : [[Text]] = [
    [],
    ["a"],
    ["s", "a"],
    ["c", "c", "a"],
    ["bl", "nb", "ot"],
    every,
    Prim.Array_tabulate<Text>(every.size(), func i = every[every.size() - 1 - i]),
  ];

  // The cell of the field `field` of a row, `#null_` for null
  func cell(field : Text, r : Row) : Cell {
    func opt<T>(v : ?T, c : T -> Cell) : Cell = switch v {
      case null { #null_ }
      case ?v { c(v) }
    };
    switch field {
      case "a" { #nat(r.a) }
      case "b" { opt<Int>(r.b, func b = #int(b)) }
      case "c" { #int(r.c) }
      case "nb" { #nat(r.nb) }
      case "s" { #text(r.s) }
      case "f" { #bool(r.f) }
      case "n8" { #nat(Prim.nat8ToNat(r.n8)) }
      case "i16" { #int(Prim.int16ToInt(r.i16)) }
      case "n64" { #nat(Prim.nat64ToNat(r.n64)) }
      case "i64" { #int(Prim.int64ToInt(r.i64)) }
      case "ot" { opt<Text>(r.ot, func t = #text(t)) }
      case "fl" { #float(r.fl) }
      case "ch" { #nat(Prim.nat32ToNat(Prim.charToNat32(r.ch))) }
      case "bl" { #blob(r.bl) }
      case "p" { #principal(r.p) }
      case _ { Prim.trap("no such field") }
    }
  };

  // The value of the integer field `field` of a row, null for null
  func int(field : Text, r : Row) : ?Int = switch cell(field, r) {
    case #nat(n) { ?n }
    case #int(i) { ?i }
    case _ { null }
  };

  func isInteger(field : Text) : Bool {
    for f in integers.values() { if f == field { return true } };
    false
  };

  func isNull(c : Cell) : Bool = switch c {
    case #null_ { true }
    case _ { false }
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
      case (#bool(a), #bool(b)) { if a == b { 0 } else if b { -1 } else { 1 } }
      case _ { Prim.trap("cells of two kinds") }
    }
  };

  // Whether the row meets the filter, as the model has it; every row meets no filter
  func meets(r : Row, filter : ?(Text, Filter)) : Bool {
    let ?(field, f) = filter else return true;
    let v = cell(field, r);
    switch f {
      case #eq(c) {
        if isNull(c) or isNull(v) { isNull(c) and isNull(v) } else { cmp(v, c) == 0 }
      }
      case #range(r) {
        let low = switch r.low {
          case null { true }
          case ?#incl(c) { cmp(v, c) >= 0 }
          case ?#excl(c) { cmp(v, c) > 0 }
        };
        let high = switch r.high {
          case null { true }
          case ?#incl(c) { cmp(v, c) <= 0 }
          case ?#excl(c) { cmp(v, c) < 0 }
        };
        not isNull(v) and low and high
      }
    }
  };

  // The number of the model's rows that meet the filter, and the sum and the number of the
  // non-null values of the field `sumField` over them
  func modelOf(sumField : Text, filter : ?(Text, Filter)) : (Nat, Int, Nat) {
    var count = 0;
    var sum : Int = 0;
    var n = 0;
    for r in model.values() {
      switch r {
        case ?r {
          if meets(r, filter) {
            count += 1;
            switch int(sumField, r) {
              case ?x {
                sum += x;
                n += 1
              }
              case null {}
            }
          }
        }
        case null {}
      }
    };
    (count, sum, n)
  };

  // Prints, after `step`, the number of rows, the number of answers that are not the model's,
  // and the number of rows that match each filter, over every filter
  func check(step : Text) {
    var wrong = 0;
    var matches = 0;
    let seen = Prim.Array_init<Bool>(m, false);
    var i = 0;
    for (field, filter) in filters.values() {
      let sumField = sums[i % sums.size()];
      let (count, sum, n) = modelOf(sumField, ?(field, filter));
      // The rows the table hands back: each once, as the model has it, and matching
      for k in seen.keys() { seen[k] := false };
      var got = 0;
      for (k, r) in rows.entriesWhere(field, filter) {
        got += 1;
        if k >= m or seen[k] or model[k] != ?r or not meets(r, ?(field, filter)) { wrong += 1 } else {
          seen[k] := true
        }
      };
      if got != count or rows.countWhere(field, filter) != count { wrong += 1 };
      if rows.sumWhere(sumField, field, filter) != ?(sum, n) { wrong += 1 };
      // The sum of the filter's own field
      let (_, own, ownN) = modelOf(field, ?(field, filter));
      let ownSum = if isInteger(field) { ?(own, ownN) } else { null };
      if rows.sumWhere(field, field, filter) != ownSum { wrong += 1 };
      matches += count;
      i += 1
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; matches })
  };

  // Whether `cells` is the key `k`'s cell and the cells of the fields `fields` of the row `r`
  func same(cells : [Cell], k : Nat, fields : [Text], r : Row) : Bool {
    if cells.size() != fields.size() + 1 or cells[0] != #nat(k) { return false };
    var i = 0;
    while i < fields.size() {
      if cells[i + 1] != cell(fields[i], r) { return false };
      i += 1
    };
    true
  };

  // The number of answers that are not the model's in reading `fields` under `filter` as cells,
  // with the key and without, and the number of rows read
  func read(fields : [Text], filter : ?(Text, Filter)) : (Nat, Nat) {
    var wrong = 0;
    let (count, _, _) = modelOf("a", filter);
    let seen = Prim.Array_init<Bool>(m, false);
    var got = 0;
    let bare = rows.cells(false, fields, filter);
    for cells in rows.cells(true, fields, filter) {
      got += 1;
      // The same row read without its key's cell
      switch bare.next() {
        case ?b {
          if b != Prim.Array_tabulate<Cell>(fields.size(), func i = cells[i + 1]) { wrong += 1 }
        }
        case null { wrong += 1 }
      };
      switch cells[0] {
        case #nat(k) {
          if k >= m or seen[k] { wrong += 1 } else {
            seen[k] := true;
            switch model[k] {
              case ?r { if not meets(r, filter) or not same(cells, k, fields, r) { wrong += 1 } }
              case null { wrong += 1 }
            }
          }
        }
        case _ { wrong += 1 }
      }
    };
    if bare.next() != null or got != count { wrong += 1 };
    (wrong, got)
  };

  // Prints the number of answers that are not the model's in reading rows as cells, each field
  // list with each filter in turn and with none, and the number of cells read
  public func readCells() : async () {
    var wrong = 0;
    var total = 0;
    var i = 0;
    while i <= filters.size() {
      let filter = if i < filters.size() { ?filters[i] } else { null };
      let fields = lists[i % lists.size()];
      let (w, n) = read(fields, filter);
      wrong += w;
      total += n * (fields.size() + 1);
      i += 1
    };
    Prim.debugPrint("cells " # debug_show { wrong; cells = total })
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

  // Every key, in key order
  public func fill() : async () {
    var k = 0;
    while k < m {
      put(k, row(k, 1));
      k += 1
    };
    check("fill")
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

  // A scan and a read as cells, each stopped after 100 rows; writes that remove, replace and
  // insert rows; both resumed: each row that the writes did not touch comes exactly once, with its
  // cells, when it matches, else never. The reads and the upkeep after it check the rows then
  public func resume() : async () {
    let filter : Filter = #range { low = incl(#nat(0)); high = excl(#nat(10_000)) };
    let fields = ["s", "a"];
    let touched = Prim.Array_init<Bool>(m, false);
    let before = Prim.Array_init<Bool>(m, false);
    for k in model.keys() {
      switch model[k] {
        case ?r { before[k] := meets(r, ?("a", filter)) }
        case null {}
      }
    };
    let seen = Prim.Array_init<Nat>(m, 0);
    let seenCells = Prim.Array_init<Nat>(m, 0);
    var wrong = 0;
    // A row the writes touched may come as it was before them, which the last batch read
    func take(cells : [Cell]) {
      switch cells[0] {
        case #nat(k) {
          seenCells[k] += 1;
          switch (model[k], touched[k]) {
            case (?r, false) { if not same(cells, k, fields, r) { wrong += 1 } }
            case (null, false) { wrong += 1 }
            case (_, true) {}
          }
        }
        case _ { wrong += 1 }
      }
    };
    let it = rows.entriesWhere("a", filter);
    let itCells = rows.cells(true, fields, ?("a", filter));
    var taken = 0;
    while taken < 100 {
      switch it.next() {
        case ?(k, _) { seen[k] += 1 }
        case null {}
      };
      switch itCells.next() {
        case ?cells { take(cells) }
        case null {}
      };
      taken += 1
    };
    var k = 0;
    while k < m {
      if k % 7 == 2 {
        drop(k);
        touched[k] := true
      } else if k % 11 == 3 {
        put(k, row(k, 4));
        touched[k] := true
      };
      k += 1
    };
    k := 0;
    while k < m {
      if k % 7 == 2 and k % 2 == 0 { put(k, row(k, 5)) };
      k += 1
    };
    for (k, _) in it { seen[k] += 1 };
    for cells in itCells { take(cells) };
    for k in model.keys() {
      let want = if before[k] { 1 } else { 0 };
      if not touched[k] and (seen[k] != want or seenCells[k] != want) { wrong += 1 }
    };
    Prim.debugPrint("resume " # debug_show { wrong })
  };

  public func clear() : async () {
    var k = 0;
    while k < m {
      drop(k);
      k += 1
    };
    check("clear")
  };

  public func noField() : async () { ignore rows.countWhere("nope", #eq(#nat(1))) };
  public func noSumField() : async () { ignore rows.sumWhere("nope", "a", #eq(#nat(1))) };
  public func noCellsField() : async () { ignore rows.cells(false, ["a", "nope"], null) };
  public func wrongKind() : async () {
    ignore rows.countWhere("s", #range { low = incl(#nat(1)); high = null })
  };
  public func negativeNat() : async () {
    ignore rows.sumWhere("a", "a", #range { low = null; high = excl(#int(-1)) })
  };
  public func nullNotOption() : async () { ignore rows.countWhere("a", #eq(#null_)) };
  public func nullEnd() : async () {
    ignore rows.countWhere("b", #range { low = incl(#null_); high = null })
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress empty "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress refill "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress resume "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress readCells "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress noSumField "DIDL\x00\x00"
//CALL ingress noCellsField "DIDL\x00\x00"
//CALL ingress wrongKind "DIDL\x00\x00"
//CALL ingress negativeNat "DIDL\x00\x00"
//CALL ingress nullNotOption "DIDL\x00\x00"
//CALL ingress nullEnd "DIDL\x00\x00"
