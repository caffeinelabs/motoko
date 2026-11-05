import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// sum, avg and sumCount of integer fields, of every integer type with and without an option, match
// a model through inserts, replacements and removals, also for sums past 64 and 128 bits. An
// empty table sums to 0 over 0 values and has no average. A field of any other type, or an option
// of one, has none of them, and a name the row does not have traps
actor {
  type Row = {
    n : Nat;
    i : Int;
    on : ?Nat;
    oi : ?Int;
    w : Int64;
    f : Float;
    t : Text;
    n8 : Nat8;
    n16 : Nat16;
    n32 : Nat32;
    n64 : Nat64;
    i8 : Int8;
    i16 : Int16;
    i32 : Int32;
    on8 : ?Nat8;
    on16 : ?Nat16;
    on32 : ?Nat32;
    on64 : ?Nat64;
    oi8 : ?Int8;
    oi16 : ?Int16;
    oi32 : ?Int32;
    ow : ?Int64;
    b : Bool;
    c : Char;
    bl : Blob;
    p : Principal;
    ob : ?Bool;
    oc : ?Char;
    ofl : ?Float;
    ot : ?Text;
    obl : ?Blob;
    op : ?Principal
  };
  type Big = { v : Int; w : ?Nat };

  let rows = NativeTable.empty<Nat, Row>().build();
  let big = NativeTable.empty<Nat, Big>().build();

  let n = 1_000;
  let p53 : Int = 2 ** 53;

  // The rows the tables should hold, by key
  let model = Prim.Array_init<?Row>(n, null);
  let bigModel = Prim.Array_init<?Big>(9, null);

  // The value, or null for every third key
  func some<T>(k : Nat, v : T) : ?T = if k % 3 == 2 { null } else { ?v };

  func row(k : Nat, s : Nat) : Row {
    let i : Int = k - 400 * s;
    let n8 = Prim.natToNat8(k * s % 256);
    let n16 = Prim.natToNat16(k * s * 251 % 65_536);
    let n32 = Prim.natToNat32(k * s * 4_000_037 % 4_294_967_296);
    let n64 = Prim.natToNat64(18_446_744_073_709_551_615 - k * s);
    let i8 = Prim.intToInt8(i % 128);
    let i16 = Prim.intToInt16(i * 32);
    let i32 = Prim.intToInt32(i * 2_000_000);
    let w = Prim.intToInt64(i * p53);
    let f = Prim.intToFloat(k) / 3;
    let t = debug_show k;
    let b = k % 2 == 0;
    let c = Prim.nat32ToChar(Prim.natToNat32(0x41 + k * s % 26));
    let bl = Prim.arrayToBlob([Prim.natToNat8(k % 256), Prim.natToNat8(s)]);
    let p = Prim.principalOfBlob(bl);
    {
      n = k * s;
      i;
      on = if k % 4 == 0 { null } else { ?(k + s) };
      oi = if k % 5 == 0 { null } else { ?(s * 100 - k * 3) };
      w;
      f;
      t;
      n8;
      n16;
      n32;
      n64;
      i8;
      i16;
      i32;
      on8 = some(k, n8);
      on16 = some(k, n16);
      on32 = some(k, n32);
      on64 = some(k, n64);
      oi8 = some(k, i8);
      oi16 = some(k, i16);
      oi32 = some(k, i32);
      ow = some(k, w);
      b;
      c;
      bl;
      p;
      ob = some(k, b);
      oc = some(k, c);
      ofl = some(k, f);
      ot = some(k, t);
      obl = some(k, bl);
      op = some(k, p)
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
    ("on", func r = r.on),
    ("oi", func r = r.oi),
    ("w", func r = ?Prim.int64ToInt(r.w)),
    ("n8", func r = ?Prim.nat8ToNat(r.n8)),
    ("n16", func r = ?Prim.nat16ToNat(r.n16)),
    ("n32", func r = ?Prim.nat32ToNat(r.n32)),
    ("n64", func r = ?Prim.nat64ToNat(r.n64)),
    ("i8", func r = ?Prim.int8ToInt(r.i8)),
    ("i16", func r = ?Prim.int16ToInt(r.i16)),
    ("i32", func r = ?Prim.int32ToInt(r.i32)),
    ("on8", func r = int(r.on8, Prim.nat8ToNat)),
    ("on16", func r = int(r.on16, Prim.nat16ToNat)),
    ("on32", func r = int(r.on32, Prim.nat32ToNat)),
    ("on64", func r = int(r.on64, Prim.nat64ToNat)),
    ("oi8", func r = int(r.oi8, Prim.int8ToInt)),
    ("oi16", func r = int(r.oi16, Prim.int16ToInt)),
    ("oi32", func r = int(r.oi32, Prim.int32ToInt)),
    ("ow", func r = int(r.ow, Prim.int64ToInt)),
  ];

  // The fields of the other types and their options
  let others = ["f", "t", "b", "c", "bl", "p", "ob", "oc", "ofl", "ot", "obl", "op"];

  func average(s : Int, c : Nat) : ?Float = if c == 0 { null } else { ?(Prim.intToFloat(s) / Prim.intToFloat(c)) };

  // Prints the sum and number of values and the average of every integer field of `rows`, the
  // number of fields whose answers differ from the model's, and the number of other fields that
  // have no sum, no number of values and no average
  func check() {
    var wrong = 0;
    for (name, field) in fields.values() {
      var s : Int = 0;
      var c = 0;
      for r in model.values() {
        switch r {
          case null {}
          case ?r { switch field(r) {
            case null {}
            case ?v { s += v; c += 1 }
          } }
        }
      };
      if rows.sumCount(name) != ?(s, c) or rows.sum(name) != ?s or rows.avg(name) != average(s, c) { wrong += 1 };
      Prim.debugPrint(name # " " # debug_show (rows.sumCount(name), rows.avg(name)))
    };
    var none = 0;
    for name in others.values() {
      if rows.sumCount(name) == null and rows.sum(name) == null and rows.avg(name) == null { none += 1 }
    };
    Prim.debugPrint(debug_show { size = rows.size(); wrong; none })
  };

  public func empty() : async () { check() };

  public func fill() : async () {
    var k = 0;
    while k < n {
      let r = row(k, 1);
      rows.add(k, r);
      model[k] := ?r;
      k += 1
    };
    check()
  };

  // Removals, and replacements that change every field, null to a value and back
  public func change() : async () {
    var k = 0;
    while k < n {
      if k % 3 == 0 {
        rows.remove(k);
        model[k] := null
      } else if k % 7 == 1 {
        let r = row(k + 1, 2);
        rows.add(k, r);
        model[k] := ?r
      };
      k += 1
    };
    check()
  };

  public func clear() : async () {
    var k = 0;
    while k < n {
      rows.remove(k);
      model[k] := null;
      k += 1
    };
    check()
  };

  // Prints the sums of `big` and whether they and the averages are the model's
  func bigCheck(step : Text) {
    var v : Int = 0;
    var w : Int = 0;
    var vc = 0;
    var wc = 0;
    for r in bigModel.values() {
      switch r {
        case null {}
        case ?r {
          v += r.v;
          vc += 1;
          switch r.w {
            case null {}
            case ?x { w += x; wc += 1 }
          }
        }
      }
    };
    let right = big.sum("v") == ?v and big.sum("w") == ?w and big.avg("v") == average(v, vc) and big.avg("w") == average(w, wc)
      and big.sumCount("v") == ?(v, vc) and big.sumCount("w") == ?(w, wc);
    Prim.debugPrint(step # " " # debug_show (big.sum("v"), big.sum("w"), right))
  };

  func bigAdd(k : Nat, r : Big) {
    big.add(k, r);
    bigModel[k] := ?r
  };

  func bigRemove(k : Nat) {
    big.remove(k);
    bigModel[k] := null
  };

  // Sums past 64 bits, then past 128 bits, both signs, and back
  public func bigs() : async () {
    bigCheck("empty");
    bigAdd(0, { v = 2 ** 62; w = ?1 });
    bigAdd(1, { v = 2 ** 62; w = null });
    bigCheck("2^63");
    bigAdd(2, { v = -(2 ** 64); w = null });
    bigCheck("-2^63");
    bigAdd(3, { v = -1; w = ?2 });
    bigCheck("-2^63 - 1");
    var k = 4;
    while k < 8 {
      bigAdd(k, { v = 2 ** 126; w = ?(2 ** 200) });
      k += 1
    };
    bigCheck("past 128 bits");
    bigAdd(8, { v = -(2 ** 130); w = null });
    bigCheck("below -2^128");
    k := 4;
    while k < 9 {
      bigRemove(k);
      k += 1
    };
    bigCheck("back");
    k := 0;
    while k < 4 {
      bigRemove(k);
      k += 1
    };
    bigCheck("cleared")
  };

  public func noField() : async () { ignore rows.sum("nope") };
  public func emptyName() : async () { ignore rows.avg("") };
  public func noFieldCount() : async () { ignore rows.sumCount("nope") }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress empty "DIDL\x00\x00"
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress bigs "DIDL\x00\x00"
//CALL ingress noField "DIDL\x00\x00"
//CALL ingress emptyName "DIDL\x00\x00"
//CALL ingress noFieldCount "DIDL\x00\x00"
