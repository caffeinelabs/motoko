import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// Rows read as cells match a model: every row, or the rows whose field meets a filter, each as
// the cells of its key and of the fields named, in the order named, also a field named twice and
// none at all, and the same rows read without the key's cell. The fields are of every scalar type, with values that do not lie in one piece of
// table memory (Text, Blob, Nat and Int of more than 64 bytes) and option fields with nulls. The
// table goes through inserts in key order, upkeep that seals its row groups, replacements and
// removals, inserts again and the removal of every row. A read resumed after writes hands back
// each matching row that stayed exactly once. A table of Text keys longer than 64 bytes and a
// scalar row is read with no field. A name the row does not have, and a filter of the wrong kind,
// trap
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
  let names = NativeTable.empty<Text, Nat>().build();

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

  // A Text of `n` times "x"
  func xs(n : Nat) : Text {
    var t = "";
    var i = 0;
    while i < n {
      t #= "x";
      i += 1
    };
    t
  };

  // Short words, one longer than 16 bytes, one of 300 bytes, the empty one and one with a
  // character of two bytes
  transient let words : [Text] = [
    "apple",
    "banana",
    "cherry",
    "pear",
    "plum",
    "a fruit with a long name",
    "",
    "\u{e9}clair",
    xs(300),
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

  // The row of key `k` at step `s`: on the first step, field a is the key
  func row(k : Nat, s : Nat) : Row {
    let r = mix(k, s);
    {
      a = if s == 1 { k } else { r % 20_000 };
      b = if r % 4 == 0 { null } else { ?(r / 4 % 2_001 - 1_000) };
      c = if r % 53 == 0 { (r - 2 ** 63) * 2 ** 500 } else if r % 59 == 0 {
        (r - 2 ** 63) * 2 ** 70
      } else { r % 100_001 - 50_000 };
      nb = if r % 61 == 0 { r * 2 ** 600 } else { r };
      s = words[r / 7 % words.size()];
      f = r % 3 == 0;
      n8 = Prim.natToNat8(r % 256);
      i16 = Prim.intToInt16(r % 65_536 - 32_768);
      n64 = Prim.natToNat64(r);
      i64 = Prim.intToInt64(r % 2 ** 64 - 2 ** 63);
      ot = if r / 11 % 3 == 0 { null } else { ?words[r / 13 % words.size()] };
      fl = Prim.intToFloat(r / 17 % 2_001 - 1_000) / 8.0;
      ch = Prim.nat32ToChar(Prim.natToNat32(0x61 + r / 19 % 26));
      bl = blobs[r / 23 % blobs.size()];
      p = principals[r / 29 % principals.size()]
    }
  };

  func incl(c : Cell) : ?Bound = ?#incl(c);
  func excl(c : Cell) : ?Bound = ?#excl(c);

  transient let filters : [?(Text, Filter)] = [
    null,
    ?("a", #range { low = incl(#nat(2_000)); high = excl(#nat(5_000)) }),
    ?("b", #eq(#null_)),
    ?("s", #eq(#text("pear"))),
    ?("c", #range { low = incl(#int(2 ** 64)); high = null }),
  ];

  transient let every = ["a", "b", "c", "nb", "s", "f", "n8", "i16", "n64", "i64", "ot", "fl", "ch", "bl", "p"];

  // The field lists read with every filter, and those read with the first two
  transient let narrow : [[Text]] = [[], ["a"], ["s", "a"], ["c", "c", "a"], ["bl", "nb", "ot"]];
  transient let wide : [[Text]] = [every, Prim.Array_tabulate<Text>(every.size(), func i = every[every.size() - 1 - i])];

  func text(t : ?Text) : Cell = switch t {
    case null { #null_ }
    case ?t { #text(t) }
  };

  // The cell of the field `field` of a row
  func cell(field : Text, r : Row) : Cell {
    switch field {
      case "a" { #nat(r.a) }
      case "b" {
        switch r.b {
          case null { #null_ }
          case ?b { #int(b) }
        }
      }
      case "c" { #int(r.c) }
      case "nb" { #nat(r.nb) }
      case "s" { #text(r.s) }
      case "f" { #bool(r.f) }
      case "n8" { #nat(Prim.nat8ToNat(r.n8)) }
      case "i16" { #int(Prim.int16ToInt(r.i16)) }
      case "n64" { #nat(Prim.nat64ToNat(r.n64)) }
      case "i64" { #int(Prim.int64ToInt(r.i64)) }
      case "ot" { text(r.ot) }
      case "fl" { #float(r.fl) }
      case "ch" { #nat(Prim.nat32ToNat(Prim.charToNat32(r.ch))) }
      case "bl" { #blob(r.bl) }
      case "p" { #principal(r.p) }
      case _ { Prim.trap("no such field") }
    }
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
      case (#text(a), #text(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case _ { Prim.trap("cells the model does not order") }
    }
  };

  // Whether the row meets the filter, as the model has it
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

  // The number of answers that are not the model's in reading `fields` under `filter`, with the
  // key and without, and the number of rows read
  func read(fields : [Text], filter : ?(Text, Filter)) : (Nat, Nat) {
    var wrong = 0;
    var count = 0;
    for r in model.values() {
      switch r {
        case ?r { if meets(r, filter) { count += 1 } }
        case null {}
      }
    };
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
    if bare.next() != null { wrong += 1 };
    if got != count { wrong += 1 };
    (wrong, got)
  };

  // Prints, after `step`, the number of rows, the number of answers that are not the model's,
  // and the number of rows read over every read
  func check(step : Text) {
    var wrong = 0;
    var cells = 0;
    for fields in narrow.values() {
      for filter in filters.values() {
        let (w, n) = read(fields, filter);
        wrong += w;
        cells += n * (fields.size() + 1)
      }
    };
    for fields in wide.values() {
      for filter in filters.values() {
        switch filter {
          case null or ?("a", _) {
            let (w, n) = read(fields, filter);
            wrong += w;
            cells += n * (fields.size() + 1)
          }
          case _ {}
        }
      }
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; cells })
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

  // A read stopped after 100 rows, writes that remove, replace and insert rows, and the read
  // resumed: each row that the writes did not touch comes exactly once, with its cells, when it
  // matches, else never
  public func resume() : async () {
    let filter = ?("a", #range { low = incl(#nat(0)); high = excl(#nat(10_000)) } : Filter);
    let fields = ["s", "a"];
    let touched = Prim.Array_init<Bool>(m, false);
    let before = Prim.Array_init<Bool>(m, false);
    for k in model.keys() {
      switch model[k] {
        case ?r { before[k] := meets(r, filter) }
        case null {}
      }
    };
    let seen = Prim.Array_init<Nat>(m, 0);
    var wrong = 0;
    func take(cells : [Cell]) {
      switch cells[0] {
        case #nat(k) {
          seen[k] += 1;
          // A row the writes touched may come with the cells it had before them, which the
          // last batch read
          switch (model[k], touched[k]) {
            case (?r, false) { if not same(cells, k, fields, r) { wrong += 1 } }
            case (null, false) { wrong += 1 }
            case (_, true) {}
          }
        }
        case _ { wrong += 1 }
      }
    };
    let it = rows.cells(true, fields, filter);
    var taken = 0;
    while taken < 100 {
      switch it.next() {
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
    for cells in it { take(cells) };
    for k in model.keys() {
      if not touched[k] and seen[k] != (if before[k] { 1 } else { 0 }) { wrong += 1 }
    };
    Prim.debugPrint("resume " # debug_show { wrong });
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

  // Keys of 1 to 200 bytes, read with no field: each key once; without the key, each row as no
  // cell
  public func keys() : async () {
    var n = 0;
    while n < 200 {
      names.add(xs(n + 1), n);
      n += 1
    };
    let seen = Prim.Array_init<Bool>(200, false);
    var wrong = 0;
    var got = 0;
    for cells in names.cells(true, [], null) {
      got += 1;
      switch (cells.size(), cells[0]) {
        case (1, #text(t)) {
          let n = t.size();
          if n == 0 or n > 200 or t != xs(n) or seen[n - 1] { wrong += 1 } else {
            seen[n - 1] := true
          }
        }
        case _ { wrong += 1 }
      }
    };
    if got != 200 { wrong += 1 };
    var bare = 0;
    for cells in names.cells(false, [], null) {
      bare += 1;
      if cells.size() != 0 { wrong += 1 }
    };
    if bare != 200 { wrong += 1 };
    Prim.debugPrint("keys " # debug_show { got; wrong })
  };

  public func noField() : async () { ignore rows.cells(false, ["a", "nope"], null) };
  public func noRowField() : async () { ignore names.cells(true, ["a"], null) };
  public func noFilterField() : async () { ignore rows.cells(true, ["a"], ?("nope", #eq(#nat(1)))) };
  public func wrongKind() : async () { ignore rows.cells(false, ["a"], ?("a", #eq(#text("1")))) }
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
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress keys "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress noRowField "DIDL\x00\x00"
//CALL ingress noFilterField "DIDL\x00\x00"
//CALL ingress wrongKind "DIDL\x00\x00"
