import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// Blobs and Texts in every form the runtime makes them are written, read, indexed and filtered,
// as keys and as row fields. Blobs: literals; from `encodeUtf8`, which hands back the Text's own
// heap object, of a literal and of a concatenation; from `arrayToBlob` and `arrayMutToBlob`; two
// halves joined; a part of a longer Blob; a part of a Region. Texts: literals; concatenations,
// character by character and of two halves; from `decodeUtf8`; a part of a longer Text; a part
// of a longer Blob and of a Region, decoded. Each form is looked up, counted, walked and filtered
// by the same bytes in every other form. The values are empty, short, not ASCII, and longer than
// 64 bytes. The forms are made in one call and used in later ones, after garbage collection
actor {
  type Row = { b : Blob; t : Text; ob : ?Blob; ot : ?Text };

  let byBlob = NativeTable.empty<Blob, Row>().createIndex(["b"]).createIndex(["t"]).createIndex(["ob"]).createIndex(["ot"]).build();
  let byText = NativeTable.empty<Text, Row>().createIndex(["b"]).createIndex(["t"]).build();
  let blobRows = NativeTable.empty<Text, Blob>().build();

  let texts : [Text] = [
    "",
    "abc",
    "Zürich, the largest city",
    "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789",
  ];
  let blobs : [Blob] = [
    "",
    "abc",
    "Zürich, the largest city",
    "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789",
  ];

  let region = Prim.regionNew();

  // The forms of each value
  var blobForms : [[Blob]] = [];
  var textForms : [[Text]] = [];

  // The characters of `t` from the `from`th on, `n` of them, concatenated one by one
  func chars(t : Text, from : Nat, n : Nat) : Text {
    var r = "";
    var i = 0;
    for c in t.chars() {
      if i >= from and i < from + n { r #= Prim.charToText(c) };
      i += 1
    };
    r
  };

  // The `n` bytes of `b` from the `from`th on
  func part(b : Blob, from : Nat, n : Nat) : Blob {
    let a = Prim.blobToArray(b);
    Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(n, func(i) = a[from + i]))
  };

  func join(a : Blob, b : Blob) : Blob {
    let x = Prim.blobToArray(a);
    let y = Prim.blobToArray(b);
    Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(x.size() + y.size(), func(i) = if i < x.size() { x[i] } else { y[i - x.size()] }))
  };

  func decode(b : Blob) : Text {
    switch Prim.decodeUtf8(b) {
      case ?t { t }
      case null { Prim.trap("not UTF-8") }
    }
  };

  func row(k : Nat, i : Nat) : Row {
    let bs = blobForms[k];
    let ts = textForms[k];
    {
      b = bs[(i + 1) % bs.size()];
      t = ts[(i + 2) % ts.size()];
      ob = ?bs[(i + 3) % bs.size()];
      ot = ?ts[(i + 4) % ts.size()]
    }
  };

  func expected(k : Nat) : Row = { b = blobs[k]; t = texts[k]; ob = ?blobs[k]; ot = ?texts[k] };

  // Whether `it` hands out exactly one row, the row of value `k`
  func one<K>(it : { next : () -> ?(K, Row) }, k : Nat) : Bool {
    var n = 0;
    var same = true;
    for (_, r) in it {
      n += 1;
      if r != expected(k) { same := false }
    };
    n == 1 and same
  };

  public func forms() : async () {
    ignore Prim.regionGrow(region, 1);
    let bss = Prim.Array_init<[Blob]>(texts.size(), []);
    let tss = Prim.Array_init<[Text]>(texts.size(), []);
    var wrong = 0;
    for k in texts.keys() {
      let t = texts[k];
      let b = blobs[k];
      let n = b.size();
      let wide = Prim.encodeUtf8("<<" # t # ">>");
      let at = Prim.natToNat64(k * 256);
      Prim.regionStoreBlob(region, at, wide);
      bss[k] := [
        b,
        Prim.encodeUtf8(t),
        Prim.encodeUtf8(chars(t, 0, t.size())),
        Prim.arrayToBlob(Prim.blobToArray(b)),
        Prim.arrayMutToBlob(Prim.blobToArrayMut(b)),
        join(part(b, 0, n / 2), part(b, n / 2, n - n / 2)),
        part(wide, 2, n),
        Prim.regionLoadBlob(region, at + 2, n),
      ];
      tss[k] := [
        t,
        chars(t, 0, t.size()),
        chars(t, 0, t.size() / 2) # chars(t, t.size() / 2, t.size() - t.size() / 2),
        decode(b),
        chars("<<" # t # ">>", 2, t.size()),
        decode(part(wide, 2, n)),
        decode(Prim.regionLoadBlob(region, at + 2, n)),
      ];
      for x in bss[k].values() { if x != b { wrong += 1 } };
      for x in tss[k].values() { if x != t { wrong += 1 } }
    };
    blobForms := Prim.Array_tabulate<[Blob]>(bss.size(), func(k) = bss[k]);
    textForms := Prim.Array_tabulate<[Text]>(tss.size(), func(k) = tss[k]);
    Prim.debugPrint("forms " # debug_show { blobs = blobForms[0].size(); texts = textForms[0].size(); wrong })
  };

  // Every value written under each of its forms as a key, its row fields in other forms
  public func write() : async () {
    var fresh = 0;
    for k in texts.keys() {
      let bs = blobForms[k];
      let ts = textForms[k];
      var i = 0;
      while i < bs.size() {
        if byBlob.insert(bs[i], row(k, i)) { fresh += 1 };
        if byText.insert(ts[i % ts.size()], row(k, i)) { fresh += 1 };
        if blobRows.insert(ts[i % ts.size()], bs[(i + 1) % bs.size()]) { fresh += 1 };
        i += 1
      }
    };
    Prim.debugPrint("write " # debug_show { fresh });
    check("write")
  };

  public func upkeep() : async () {
    while byBlob.maintain(100_000_000) {};
    while byText.maintain(100_000_000) {};
    while blobRows.maintain(100_000_000) {};
    check("upkeep")
  };

  // Each value read by key, counted and walked through each index and met by filters, in each of
  // its forms
  func check(step : Text) {
    var wrong = 0;
    for k in texts.keys() {
      let bs = blobForms[k];
      let ts = textForms[k];
      for j in bs.keys() {
        let b = bs[j];
        let b2 = bs[(j + 1) % bs.size()];
        if byBlob.get(b) != ?expected(k) or not byBlob.containsKey(b) { wrong += 1 };
        if byBlob.count(["b"], [#blob(b)]) != 1 or byBlob.count(["ob"], [#blob(b)]) != 1 { wrong += 1 };
        if byText.count(["b"], [#blob(b)]) != 1 { wrong += 1 };
        if byBlob.countWhere("b", #eq(#blob(b))) != 1 or byBlob.countWhere("ob", #eq(#blob(b))) != 1 { wrong += 1 };
        if not one(byBlob.indexEntries(["b"], ?[#blob(b)], ?[#blob(b2)]), k) { wrong += 1 };
        if not one(byBlob.reverseIndexEntries(["ob"], ?[#blob(b2)], ?[#blob(b)]), k) { wrong += 1 };
        if not one(byBlob.entriesWhere("b", #range { low = ?#incl(#blob(b)); high = ?#incl(#blob(b2)) }), k) {
          wrong += 1
        };
        var rows = 0;
        for cells in byBlob.cells(true, ["t", "b"], ?("ob", #eq(#blob(b)))) {
          rows += 1;
          if cells != [#blob(blobs[k]), #text(texts[k]), #blob(blobs[k])] { wrong += 1 }
        };
        if rows != 1 { wrong += 1 }
      };
      for j in ts.keys() {
        let t = ts[j];
        let t2 = ts[(j + 1) % ts.size()];
        if byText.get(t) != ?expected(k) or not byText.containsKey(t) { wrong += 1 };
        if blobRows.get(t) != ?blobs[k] { wrong += 1 };
        if byBlob.count(["t"], [#text(t)]) != 1 or byBlob.count(["ot"], [#text(t)]) != 1 { wrong += 1 };
        if byText.count(["t"], [#text(t)]) != 1 { wrong += 1 };
        if byText.countWhere("t", #eq(#text(t))) != 1 or byBlob.countWhere("ot", #eq(#text(t))) != 1 { wrong += 1 };
        if not one(byText.indexEntries(["t"], ?[#text(t)], ?[#text(t2)]), k) { wrong += 1 };
        if not one(byBlob.reverseIndexEntries(["ot"], ?[#text(t2)], ?[#text(t)]), k) { wrong += 1 };
        if not one(byText.entriesWhere("t", #range { low = ?#incl(#text(t)); high = ?#incl(#text(t2)) }), k) {
          wrong += 1
        };
        var rows = 0;
        for cells in byText.cells(true, ["b"], ?("t", #eq(#text(t)))) {
          rows += 1;
          if cells != [#text(texts[k]), #blob(blobs[k])] { wrong += 1 }
        };
        if rows != 1 { wrong += 1 }
      }
    };
    for (k, r) in byBlob.entries() {
      var found = false;
      for i in blobs.keys() { if k == blobs[i] and r == expected(i) { found := true } };
      if not found { wrong += 1 }
    };
    let size = (byBlob.size(), byText.size(), blobRows.size());
    Prim.debugPrint(step # " " # debug_show { size; wrong })
  };

  // Each value removed by one form; removing it again by another finds nothing
  public func remove() : async () {
    var removed = 0;
    for k in texts.keys() {
      let bs = blobForms[k];
      let ts = textForms[k];
      if byBlob.delete(bs[k % bs.size()]) { removed += 1 };
      if byBlob.delete(bs[(k + 1) % bs.size()]) { removed += 1 };
      if byText.delete(ts[(k + 2) % ts.size()]) { removed += 1 };
      if byText.delete(ts[(k + 3) % ts.size()]) { removed += 1 };
      if blobRows.delete(ts[(k + 4) % ts.size()]) { removed += 1 }
    };
    let size = (byBlob.size(), byText.size(), blobRows.size());
    Prim.debugPrint("remove " # debug_show { removed; size })
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress forms "DIDL\x00\x00"
//CALL ingress write "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress write "DIDL\x00\x00"
//CALL ingress remove "DIDL\x00\x00"
