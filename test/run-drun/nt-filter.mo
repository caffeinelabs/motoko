import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Filtered reads match a model: the rows whose field meets a filter, their number, and the sum
// and count of an integer field over them. The filters are equalities, ranges closed, half-open,
// open and empty, with included and excluded ends, and null asked of option fields, on fields of
// type Nat, Int (also past 64 and 128 bits), Nat8, Int16, ?Int, Text, ?Text, Bool, Float and
// Char, none of them indexed. The table goes through inserts in key order, upkeep that seals its
// row groups, replacements and removals, inserts again and the removal of every row. A scan
// resumed after writes hands back each matching row that stayed exactly once. A name the row
// does not have, a cell of the wrong kind or out of its field's range, null asked of a field that
// is not an option, and a null end of a range trap
actor {
  type Cell = NativeTable.Cell;
  type Filter = NativeTable.Filter;
  type Bound = NativeTable.Bound;
  type Row = {
    a : Nat;
    b : ?Int;
    c : Int;
    s : Text;
    f : Bool;
    n8 : Nat8;
    i16 : Int16;
    ot : ?Text;
    fl : Float;
    ch : Char
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

  // The row of key `k` at step `s`: on the first step, field a is the key, so each row group
  // holds a run of its values
  func row(k : Nat, s : Nat) : Row {
    let r = mix(k, s);
    let w = words.size();
    {
      a = if s == 1 { k } else { r % 20_000 };
      b = if r % 4 == 0 { null } else { ?(r / 4 % 2_001 - 1_000) };
      c = if r % 53 == 0 { (r - 2 ** 63) * 2 ** 70 } else { r % 100_001 - 50_000 };
      s = words[r / 7 % w];
      f = r % 3 == 0;
      n8 = Prim.natToNat8(r % 256);
      i16 = Prim.intToInt16(r % 65_536 - 32_768);
      ot = if r / 11 % 3 == 0 { null } else { ?words[r / 13 % w] };
      fl = Prim.intToFloat(r / 17 % 2_001 - 1_000) / 8.0;
      ch = Prim.nat32ToChar(Prim.natToNat32(0x61 + r / 19 % 26))
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

  // The fields whose sums are checked, each with each filter in turn
  transient let sums = ["a", "b", "c", "n8", "i16"];

  // The value of the field `field` of a row, as a cell, `#null_` for null
  func getter(field : Text) : Row -> Cell {
    func text(t : ?Text) : Cell = switch t {
      case null { #null_ }
      case ?t { #text(t) }
    };
    switch field {
      case "a" { func r = #nat(r.a) }
      case "b" {
        func r = switch r.b {
          case null { #null_ }
          case ?b { #int(b) }
        }
      }
      case "c" { func r = #int(r.c) }
      case "s" { func r = #text(r.s) }
      case "f" { func r = #bool(r.f) }
      case "n8" { func r = #nat(Prim.nat8ToNat(r.n8)) }
      case "i16" { func r = #int(Prim.int16ToInt(r.i16)) }
      case "ot" { func r = text(r.ot) }
      case "fl" { func r = #float(r.fl) }
      case "ch" { func r = #nat(Prim.nat32ToNat(Prim.charToNat32(r.ch))) }
      case _ { Prim.trap("no such field") }
    }
  };

  // The value of the integer field `field` of a row, null for null; null for any other field
  func intGetter(field : Text) : ?(Row -> ?Int) {
    switch field {
      case "a" { ?(func r = ?r.a) }
      case "b" { ?(func r = r.b) }
      case "c" { ?(func r = ?r.c) }
      case "n8" { ?(func r = ?Prim.nat8ToNat(r.n8)) }
      case "i16" { ?(func r = ?Prim.int16ToInt(r.i16)) }
      case _ { null }
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
      case (#float(a), #float(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#text(a), #text(b)) { if a < b { -1 } else if a > b { 1 } else { 0 } }
      case (#bool(a), #bool(b)) { if a == b { 0 } else if b { -1 } else { 1 } }
      case _ { Prim.trap("cells of two kinds") }
    }
  };

  // Whether the value `v` meets the filter, as the model has it
  func meets(v : Cell, filter : Filter) : Bool {
    switch filter {
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

  // The sum and the number of the non-null values `get` gives of the model's rows that `keep`
  // keeps
  func modelSum(get : Row -> ?Int, keep : Row -> Bool) : (Int, Nat) {
    var sum : Int = 0;
    var n = 0;
    for r in model.values() {
      switch r {
        case ?r {
          if keep(r) {
            switch get(r) {
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
    (sum, n)
  };

  // Prints, after `step`, the number of rows, the number of answers that are not the model's,
  // and the number of rows that match each filter, over every filter
  func check(step : Text) {
    var wrong = 0;
    var matches = 0;
    let seen = Prim.Array_init<Bool>(m, false);
    var i = 0;
    for (field, filter) in filters.values() {
      let get = getter(field);
      let keep = func(r : Row) : Bool = meets(get(r), filter);
      // The model's answers
      var count = 0;
      for k in model.keys() {
        seen[k] := false;
        switch model[k] {
          case ?r { if keep(r) { count += 1 } }
          case null {}
        }
      };
      // The rows the table hands back: each once, as the model has it, and matching
      var got = 0;
      for (k, r) in rows.entriesWhere(field, filter) {
        got += 1;
        if k >= m or seen[k] or model[k] != ?r or not keep(r) { wrong += 1 } else {
          seen[k] := true
        }
      };
      if got != count { wrong += 1 };
      if rows.countWhere(field, filter) != count { wrong += 1 };
      let sumField = sums[i % sums.size()];
      switch intGetter(sumField) {
        case ?g { if rows.sumWhere(sumField, field, filter) != ?modelSum(g, keep) { wrong += 1 } }
        case null { wrong += 1 }
      };
      // The sum of the filter's own field
      switch intGetter(field) {
        case ?g { if rows.sumWhere(field, field, filter) != ?modelSum(g, keep) { wrong += 1 } }
        case null { if rows.sumWhere(field, field, filter) != null { wrong += 1 } }
      };
      matches += count;
      i += 1
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; matches })
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

  // A scan stopped after 100 rows, writes that remove, replace and insert rows, and the scan
  // resumed: each row that the writes did not touch comes exactly once when it matches, else never
  public func resume() : async () {
    let filter : Filter = #range { low = incl(#nat(0)); high = excl(#nat(10_000)) };
    let keep = func(r : Row) : Bool = meets(#nat(r.a), filter);
    let touched = Prim.Array_init<Bool>(m, false);
    let before = Prim.Array_init<Bool>(m, false);
    for k in model.keys() {
      switch model[k] {
        case ?r { before[k] := keep(r) }
        case null {}
      }
    };
    let seen = Prim.Array_init<Nat>(m, 0);
    let it = rows.entriesWhere("a", filter);
    var taken = 0;
    while taken < 100 {
      switch it.next() {
        case ?(k, _) { seen[k] += 1 }
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
    var wrong = 0;
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

  public func noField() : async () { ignore rows.countWhere("nope", #eq(#nat(1))) };
  public func noFieldScan() : async () {
    ignore rows.entriesWhere("nope", #eq(#nat(1)))
  };
  public func noSumField() : async () {
    ignore rows.sumWhere("nope", "a", #eq(#nat(1)))
  };
  public func wrongKind() : async () { ignore rows.countWhere("a", #eq(#text("1"))) };
  public func wrongKindEnd() : async () {
    ignore rows.countWhere("s", #range { low = incl(#nat(1)); high = null })
  };
  public func outOfRange() : async () { ignore rows.countWhere("n8", #eq(#nat(256))) };
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
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress refill "DIDL\x00\x00"
//CALL ingress resume "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress noFieldScan "DIDL\x00\x00"
//CALL ingress noSumField "DIDL\x00\x00"
//CALL ingress wrongKind "DIDL\x00\x00"
//CALL ingress wrongKindEnd "DIDL\x00\x00"
//CALL ingress outOfRange "DIDL\x00\x00"
//CALL ingress negativeNat "DIDL\x00\x00"
//CALL ingress nullNotOption "DIDL\x00\x00"
//CALL ingress nullEnd "DIDL\x00\x00"
