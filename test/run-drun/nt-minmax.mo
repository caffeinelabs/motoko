import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// min and max of integer fields, of every integer type with and without an option, match a model
// through inserts, replacements that move a field's smallest or largest value, and removals of
// the rows that hold them, in a table of one row group and in one of several, sealed and still
// buffered, also for values past 64, 128 and 256 bits and at the bounds of every type. A table
// with no rows, and a field whose values are all null, have none. A field of any other type, or
// an option of one, has none, and a name the row does not have traps
actor {
  type Row = {
    n : Nat;
    i : Int;
    n8 : Nat8;
    n16 : Nat16;
    n32 : Nat32;
    n64 : Nat64;
    i8 : Int8;
    i16 : Int16;
    i32 : Int32;
    i64 : Int64;
    on : ?Nat;
    oi : ?Int;
    on8 : ?Nat8;
    on16 : ?Nat16;
    on32 : ?Nat32;
    on64 : ?Nat64;
    oi8 : ?Int8;
    oi16 : ?Int16;
    oi32 : ?Int32;
    oi64 : ?Int64;
    z : ?Nat;
    f : Float;
    t : Text;
    b : Bool;
    c : Char;
    ofl : ?Float;
    ot : ?Text
  };
  type Item = { v : Int; o : ?Nat; w : Nat16 };

  // A table of one row group, and one of several
  let rows = NativeTable.empty<Nat, Row>().build();
  let items = NativeTable.empty<Nat, Item>().build();

  let n = 1_000;
  let m = 12_000;
  var added = 0;

  // The rows the tables should hold, by key
  let model = Prim.Array_init<?Row>(n, null);
  let itemModel = Prim.Array_init<?Item>(m, null);

  // A value from 0 to 2^64 - 1 that the key `k` and the step `s` pick
  func mix(k : Nat, s : Nat) : Nat {
    var x = Prim.natToNat64(k * 1_000_003 + s * 7_919 + 1);
    x := (x ^ (x >> 30)) *% 0xbf58_476d_1ce4_e5b9;
    x := (x ^ (x >> 27)) *% 0x94d0_49bb_1331_11eb;
    Prim.nat64ToNat(x ^ (x >> 31))
  };

  // The value, or null for about a third of the picks `r`
  func some<T>(r : Nat, v : T) : ?T = if r / 7 % 3 == 0 { null } else { ?v };

  func row(k : Nat, s : Nat) : Row {
    let r = mix(k, s);
    let n : Nat = if r % 31 == 0 { r * 2 ** 70 } else if r % 31 == 1 { r * 2 ** 30 } else { r % 1_000_000 };
    let i : Int = if r % 37 == 0 { (r - 2 ** 63) * 2 ** 70 } else if r % 37 == 1 { (r - 2 ** 63) * 2 ** 30 } else { r % 1_000_000 - 500_000 };
    let n8 = Prim.natToNat8(r % 256);
    let n16 = Prim.natToNat16(r % 65_536);
    let n32 = Prim.natToNat32(r % 4_294_967_296);
    let n64 = Prim.natToNat64(r);
    let i8 = Prim.intToInt8(r % 256 - 128);
    let i16 = Prim.intToInt16(r % 65_536 - 32_768);
    let i32 = Prim.intToInt32(r % 4_294_967_296 - 2_147_483_648);
    let i64 = Prim.intToInt64(r - 9_223_372_036_854_775_808);
    {
      n;
      i;
      n8;
      n16;
      n32;
      n64;
      i8;
      i16;
      i32;
      i64;
      on = some(r, n);
      oi = some(r, i);
      on8 = some(r, n8);
      on16 = some(r, n16);
      on32 = some(r, n32);
      on64 = some(r, n64);
      oi8 = some(r, i8);
      oi16 = some(r, i16);
      oi32 = some(r, i32);
      oi64 = some(r, i64);
      z = null;
      f = Prim.intToFloat(i);
      t = debug_show k;
      b = r % 2 == 0;
      c = Prim.nat32ToChar(Prim.natToNat32(0x41 + r % 26));
      ofl = some(r, Prim.intToFloat(i));
      ot = some(r, debug_show k)
    }
  };

  // A row whose integer fields all hold their type's smallest value, or its largest when `high`,
  // 2^200 in size for Nat and Int
  func bound(high : Bool) : Row {
    let r = row(0, 0);
    if high {
      {
        r with n = 2 ** 200;
        i = 2 ** 200;
        n8 = 255 : Nat8;
        n16 = 65_535 : Nat16;
        n32 = 4_294_967_295 : Nat32;
        n64 = 18_446_744_073_709_551_615 : Nat64;
        i8 = 127 : Int8;
        i16 = 32_767 : Int16;
        i32 = 2_147_483_647 : Int32;
        i64 = 9_223_372_036_854_775_807 : Int64;
        on = ?(2 ** 200);
        oi = ?(2 ** 200);
        on8 = ?(255 : Nat8);
        on16 = ?(65_535 : Nat16);
        on32 = ?(4_294_967_295 : Nat32);
        on64 = ?(18_446_744_073_709_551_615 : Nat64);
        oi8 = ?(127 : Int8);
        oi16 = ?(32_767 : Int16);
        oi32 = ?(2_147_483_647 : Int32);
        oi64 = ?(9_223_372_036_854_775_807 : Int64)
      }
    } else {
      {
        r with n = 0;
        i = -(2 ** 200);
        n8 = 0 : Nat8;
        n16 = 0 : Nat16;
        n32 = 0 : Nat32;
        n64 = 0 : Nat64;
        i8 = -128 : Int8;
        i16 = -32_768 : Int16;
        i32 = -2_147_483_648 : Int32;
        i64 = -9_223_372_036_854_775_808 : Int64;
        on = ?0;
        oi = ?(-(2 ** 200));
        on8 = ?(0 : Nat8);
        on16 = ?(0 : Nat16);
        on32 = ?(0 : Nat32);
        on64 = ?(0 : Nat64);
        oi8 = ?(-128 : Int8);
        oi16 = ?(-32_768 : Int16);
        oi32 = ?(-2_147_483_648 : Int32);
        oi64 = ?(-9_223_372_036_854_775_808 : Int64)
      }
    }
  };

  func item(k : Nat, s : Nat) : Item {
    let r = mix(k, s);
    {
      v = if r % 997 == 0 { (r - 2 ** 63) * 2 ** 70 } else { r % 2_000_001 - 1_000_000 };
      o = if r % 5 == 0 { null } else { ?(r / 5 % 100_000) };
      w = Prim.natToNat16(r % 1_000)
    }
  };

  // The value of an option as an Int, null for null
  func int<T>(v : ?T, toInt : T -> Int) : ?Int = switch v {
    case null { null }
    case ?v { ?toInt(v) }
  };

  // The integer fields, each with its value in a row, null for null
  transient let fields : [(Text, Row -> ?Int)] = [
    ("n", func r = ?r.n),
    ("i", func r = ?r.i),
    ("n8", func r = ?Prim.nat8ToNat(r.n8)),
    ("n16", func r = ?Prim.nat16ToNat(r.n16)),
    ("n32", func r = ?Prim.nat32ToNat(r.n32)),
    ("n64", func r = ?Prim.nat64ToNat(r.n64)),
    ("i8", func r = ?Prim.int8ToInt(r.i8)),
    ("i16", func r = ?Prim.int16ToInt(r.i16)),
    ("i32", func r = ?Prim.int32ToInt(r.i32)),
    ("i64", func r = ?Prim.int64ToInt(r.i64)),
    ("on", func r = r.on),
    ("oi", func r = r.oi),
    ("on8", func r = int(r.on8, Prim.nat8ToNat)),
    ("on16", func r = int(r.on16, Prim.nat16ToNat)),
    ("on32", func r = int(r.on32, Prim.nat32ToNat)),
    ("on64", func r = int(r.on64, Prim.nat64ToNat)),
    ("oi8", func r = int(r.oi8, Prim.int8ToInt)),
    ("oi16", func r = int(r.oi16, Prim.int16ToInt)),
    ("oi32", func r = int(r.oi32, Prim.int32ToInt)),
    ("oi64", func r = int(r.oi64, Prim.int64ToInt)),
    ("z", func r = r.z),
  ];

  transient let itemFields : [(Text, Item -> ?Int)] = [
    ("v", func r = ?r.v),
    ("o", func r = r.o),
    ("w", func r = ?Prim.nat16ToNat(r.w)),
  ];

  // The fields of the other types and their options
  let others = ["f", "t", "b", "c", "ofl", "ot"];

  // The smallest and the largest value of `field` over the rows of `model`, null when no row
  // holds one
  func extremes<T>(model : [var ?T], field : T -> ?Int) : (?Int, ?Int) {
    var lo : ?Int = null;
    var hi : ?Int = null;
    for r in model.values() {
      switch r {
        case null {}
        case ?r {
          switch field(r) {
            case null {}
            case ?v {
              switch lo {
                case ?x { if v < x { lo := ?v } }
                case null { lo := ?v }
              };
              switch hi {
                case ?x { if v > x { hi := ?v } }
                case null { hi := ?v }
              }
            }
          }
        }
      }
    };
    (lo, hi)
  };

  // The first key of `model` whose row holds the value `v` of `field`, null when there is none
  func holder<T>(model : [var ?T], field : T -> ?Int, v : ?Int) : ?Nat {
    if v == null { return null };
    var k = 0;
    while k < model.size() {
      switch model[k] {
        case ?r { if field(r) == v { return ?k } }
        case null {}
      };
      k += 1
    };
    null
  };

  // The number of `fields` whose min or max in `table` is not the model's, printing each field's
  // min and max when `show`
  func wrongs<T>(table : NativeTable.NativeTable<Nat, T>, model : [var ?T], fields : [(Text, T -> ?Int)], show : Bool) : Nat {
    var wrong = 0;
    for (name, field) in fields.values() {
      let (lo, hi) = extremes(model, field);
      let (min, max) = (table.min(name), table.max(name));
      if min != lo or max != hi { wrong += 1 };
      if show { Prim.debugPrint(name # " " # debug_show (min, max)) }
    };
    wrong
  };

  // Prints, after `step`, the number of integer fields of `rows` whose min or max is not the
  // model's, and the number of other fields that have neither
  func check(step : Text, show : Bool) {
    let wrong = wrongs(rows, model, fields, show);
    var none = 0;
    for name in others.values() {
      if rows.min(name) == null and rows.max(name) == null { none += 1 }
    };
    Prim.debugPrint(step # " " # debug_show { size = rows.size(); wrong; none })
  };

  // Prints, after `step`, the min and max of every field of `items`, and the number of them that
  // are not the model's
  func checkItems(step : Text) {
    let wrong = wrongs(items, itemModel, itemFields, false);
    Prim.debugPrint(step # " " # debug_show { size = items.size(); wrong });
    Prim.debugPrint(debug_show (items.min("v"), items.max("v"), items.min("o"), items.max("o"), items.min("w"), items.max("w")))
  };

  func put(k : Nat, r : Row) {
    rows.add(k, r);
    model[k] := ?r
  };

  func drop(k : Nat) {
    rows.remove(k);
    model[k] := null
  };

  func putItem(k : Nat, r : Item) {
    items.add(k, r);
    itemModel[k] := ?r
  };

  func dropItem(k : Nat) {
    items.remove(k);
    itemModel[k] := null
  };

  // For every field and both its ends, the row that holds the end, replaced by `replace`, or
  // removed when `replace` is null
  func moveEnds<T>(model : [var ?T], fields : [(Text, T -> ?Int)], replace : ?(Nat -> T), set : (Nat, T) -> (), unset : Nat -> ()) {
    for (_, field) in fields.values() {
      let (lo, hi) = extremes(model, field);
      for v in [lo, hi].values() {
        switch (holder(model, field, v), replace) {
          case (?k, ?by) { set(k, by(k)) }
          case (?k, null) { unset(k) }
          case (null, _) {}
        }
      }
    }
  };

  public func empty() : async () {
    check("empty", true);
    checkItems("items empty")
  };

  public func fill() : async () {
    var k = 0;
    while k < n {
      put(k, row(k, 1));
      k += 1
    };
    check("fill", true)
  };

  // Removals, and replacements that change every field, null to a value and back
  public func change() : async () {
    var k = 0;
    while k < n {
      if k % 3 == 0 { drop(k) } else if k % 7 == 1 { put(k, row(k, 2)) };
      k += 1
    };
    check("change", false)
  };

  // Three times: the rows that hold each field's smallest and largest value replaced by rows of
  // other values, then the rows that hold them removed
  public func ends() : async () {
    var round = 0;
    while round < 3 {
      moveEnds(model, fields, ?(func(k : Nat) : Row = row(k, 3 + round)), put, drop);
      check("replaced " # debug_show round, false);
      moveEnds(model, fields, null, put, drop);
      check("removed " # debug_show round, false);
      round += 1
    }
  };

  // Rows at the bounds of every type, then replaced by rows of other values
  public func bounds() : async () {
    put(1, bound(false));
    put(2, bound(true));
    check("bounds", true);
    put(1, row(1, 9));
    put(2, row(2, 9));
    check("bounds replaced", true)
  };

  public func clear() : async () {
    var k = 0;
    while k < n {
      drop(k);
      k += 1
    };
    check("clear", true)
  };

  // The next 4,000 rows of `items`
  public func more() : async () {
    let end = added + 4_000;
    while added < end {
      putItem(added, item(added, 1));
      added += 1
    };
    checkItems("items " # debug_show added)
  };

  // Runs the upkeep of `items` until no work is pending
  public func upkeep() : async () {
    while items.maintain(100_000_000) {};
    checkItems("items upkeep")
  };

  // Five times: the rows that hold each field's smallest and largest value replaced by rows of
  // other values, then the rows that hold them removed
  public func itemEnds() : async () {
    var round = 0;
    while round < 5 {
      moveEnds(itemModel, itemFields, ?(func(k : Nat) : Item = item(k, 10 + round)), putItem, dropItem);
      checkItems("items replaced " # debug_show round);
      moveEnds(itemModel, itemFields, null, putItem, dropItem);
      checkItems("items removed " # debug_show round);
      round += 1
    }
  };

  // A new smallest value in the first row group and a new largest in the last, then replaced
  public func itemBounds() : async () {
    putItem(5, { v = -(2 ** 300); o = ?0; w = 0 });
    putItem(m - 5, { v = 2 ** 300; o = ?(2 ** 130); w = 65_535 });
    checkItems("items bounds");
    putItem(5, item(5, 20));
    putItem(m - 5, item(m - 5, 20));
    checkItems("items bounds replaced")
  };

  // Every row of the first row group and more removed, before and after the upkeep
  public func itemGroup() : async () {
    var k = 0;
    while k < 5_000 {
      dropItem(k);
      k += 1
    };
    checkItems("items first group removed");
    while items.maintain(100_000_000) {};
    checkItems("items first group upkeep")
  };

  public func itemClear() : async () {
    var k = 0;
    while k < m {
      dropItem(k);
      k += 1
    };
    checkItems("items clear")
  };

  public func noField() : async () { ignore rows.min("nope") };
  public func emptyName() : async () { ignore rows.max("") }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress empty "DIDL\x00\x00"
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress ends "DIDL\x00\x00"
//CALL ingress bounds "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress more "DIDL\x00\x00"
//CALL ingress more "DIDL\x00\x00"
//CALL ingress more "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress itemEnds "DIDL\x00\x00"
//CALL ingress itemBounds "DIDL\x00\x00"
//CALL ingress itemGroup "DIDL\x00\x00"
//CALL ingress itemClear "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress emptyName "DIDL\x00\x00"
