/// core Map and IndexedMap reads on the reference row R (Int ts, Nat
/// amount, 3-value Text kind, Principal owner, 10-60 B Text note, Bool flag).
///   get_nat_1      : one core Map `get`, Nat key n/2
///   get_nat_x1000  : 1,000 core Map `get`s, keys spread over the map (÷ 1,000 per get)
///   get_text_1     : one core Map `get`, 27-byte Text key "account-" + 19 zero-padded digits (a 20-byte shared prefix)
///   get_text2_*    : the same with 27-byte keys that differ in their first bytes
///   empty          : the runner with no work (the cell overhead)
///   get_text_x1000 : 1,000 gets, 27-byte Text keys
///   im_get_1       : one `IndexedMap.get` (Nat key, 3 secondary indexes)
///   im_get_x1000   : 1,000 `IndexedMap.get`s
///   values_all     : core Map `values()` over every row (÷ n per row)
///   entries_all    : core Map `entries()` over every row (÷ n per row)
///   size_map       : core Map `size()`; size_im : `IndexedMap.size()`
/// Query context: `mops bench oqlbars_coremap --query`.
import Bench      "mo:bench";
import Array      "mo:core/Array";
import Map        "mo:core/Map";
import Nat        "mo:core/Nat";
import Nat8       "mo:core/Nat8";
import Nat32      "mo:core/Nat32";
import Text       "mo:core/Text";
import Prim       "mo:prim";
import IndexedMap "../src/IndexedMap";
import _NatValue "../src/NatValue";
import _IntValue "../src/IntValue";
import _TextValue "../src/TextValue";
import _BoolValue "../src/BoolValue";
import _PrincipalValue "../src/PrincipalValue";
import _RecordValue "../src/RecordValue";
import OQL        "../src";

module {
  type R = { ts : Int; amount : Nat; kind : Text; owner : Principal; note : Text; flag : Bool };
  func kindOf(k : Nat) : Text = if (k % 10 == 0) "burn" else if (k % 3 == 0) "mint" else "xfer";
  func mkOwners() : [Principal] = Array.tabulate<Principal>(1_000, func i {
    Prim.principalOfBlob(Prim.arrayToBlob(Array.tabulate<Nat8>(29, func j = Nat8.fromNat((i * 31 + j * 7 + 3) % 256))))
  });
  func mkNotes() : [Text] = Array.tabulate<Text>(51, func i {
    var t = ""; var j = 0;
    while (j < 10 + i) { t #= Prim.charToText(Prim.nat32ToChar(Nat32.fromNat(97 + (i + j) % 26))); j += 1 };
    t
  });
  func mkR(owners : [Principal], notes : [Text], k : Nat) : R = { ts = 1_760_000_000_000_000_000 + k * 1_000; amount = (k * 7) % 1_000; kind = kindOf(k); owner = owners[k % 1_000]; note = notes[k % 51]; flag = k % 2 == 0 };
  func textKey(k : Nat) : Text {
    let d = k.toText(); var pad = ""; var i = d.size();
    while (i < 19) { pad #= "0"; i += 1 };
    "account-" # pad # d
  };

  // 27-byte Text keys that differ early: 19 digits of a multiplicative hash of k, then "-account".
  func textKey2(k : Nat) : Text {
    let d = ((k * 2_654_435_761) % 10_000_000_000_000_000_000).toText(); var pad = ""; var i = d.size();
    while (i < 19) { pad #= "0"; i += 1 };
    d # pad # "-account"
  };
  type S = { cm : Map.Map<Nat, R>; ct : Map.Map<Text, R>; ct2 : Map.Map<Text, R>; im : IndexedMap.IndexedMap<Nat, R>; n : Nat; tkeys : [Text]; t2keys : [Text]; nkeys : [Nat] };
  func build(n : Nat) : S {
    let owners = mkOwners(); let notes = mkNotes();
    let cm = Map.empty<Nat, R>(); let ct = Map.empty<Text, R>(); let ct2 = Map.empty<Text, R>();
    let im = IndexedMap.new<Nat, R>([("ts", #ordered), ("kind", #hash), ("amount", #ordered)]);
    var k = 0;
    while (k < n) { let r = mkR(owners, notes, k); cm.add(Nat.compare, k, r); ct.add(Text.compare, textKey(k), r); ct2.add(Text.compare, textKey2(k), r); im.put(k, r); k += 1 };
    let nkeys = Array.tabulate<Nat>(1_000, func i = (i * 7919 + 13) % n);
    { cm; ct; ct2; im; n; nkeys; tkeys = nkeys.map(textKey); t2keys = nkeys.map(textKey2) }
  };

  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: core Map / IndexedMap reads, reference row");
    bench.description("get by Nat and 27-byte Text key, values()/entries() per row, size().");
    let s10k = build(10_000); let s100k = build(100_000);
    func sFor(col : Text) : S = if (col == "10000") s10k else s100k;
    var sink = 0;
    bench.rows(["empty", "get_text2_1", "get_text2_x1000", "get_nat_1", "get_nat_x1000", "get_text_1", "get_text_x1000", "im_get_1", "im_get_x1000", "values_all", "entries_all", "size_map", "size_im"]);
    bench.cols(["10000", "100000"]);
    bench.runner(func (row, col) {
      let s = sFor(col);
      switch row {
        case "empty" {};
        case "get_text2_1" { switch (s.ct2.get(Text.compare, s.t2keys[0])) { case (?r) sink += r.amount; case null {} } };
        case "get_text2_x1000" { for (k in s.t2keys.values()) { switch (s.ct2.get(Text.compare, k)) { case (?r) sink += r.amount; case null {} } } };
        case "get_nat_1" { switch (s.cm.get(Nat.compare, s.n / 2)) { case (?r) sink += r.amount; case null {} } };
        case "get_nat_x1000" { for (k in s.nkeys.values()) { switch (s.cm.get(Nat.compare, k)) { case (?r) sink += r.amount; case null {} } } };
        case "get_text_1" { switch (s.ct.get(Text.compare, s.tkeys[0])) { case (?r) sink += r.amount; case null {} } };
        case "get_text_x1000" { for (k in s.tkeys.values()) { switch (s.ct.get(Text.compare, k)) { case (?r) sink += r.amount; case null {} } } };
        case "im_get_1" { switch (s.im.get(s.n / 2)) { case (?r) sink += r.amount; case null {} } };
        case "im_get_x1000" { for (k in s.nkeys.values()) { switch (s.im.get(k)) { case (?r) sink += r.amount; case null {} } } };
        case "values_all" { for (r in s.cm.values()) { sink += r.amount } };
        case "entries_all" { for ((k, r) in s.cm.entries()) { sink += r.amount } };
        case "size_map" { sink += s.cm.size() };
        case _ { sink += s.im.size() };
      };
    });
    bench;
  };
};
