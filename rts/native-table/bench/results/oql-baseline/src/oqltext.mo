/// OQL's count of one TEXT key through a heap #hash index written on chain, and a
/// core Map<Text, Nat> get over the same keys, for reference.
///
/// One canister per key shape (`setup`): the same rows go, by on-chain appends
/// (`gen`), into
///   - `ttext`: an `OQL.Table` with one text column `k` and a heap `k #hash` index
///     (the columnar table; rows flushed into Region segments, the index on the heap);
///   - `mtext`: an `IndexedMap<Nat, R>` with a heap `k #hash` index (the heap table);
///   - `mp`: a mo:core `Map<Text, Nat>`, key -> rows holding it.
/// The three hold the same Text objects.
///
/// Key shapes (the key of value u; text = the bytes below, no terminator):
///   0  17 characters: 'b', then u's 16 hexadecimal digits as the letters a..p;
///   1  paths: 59 to 199 characters (59 + splitmix64fin(u) mod 141): 40 shared
///      characters ('a' + i mod 23), u's 16 digits as letters, then filler
///      ('q' + i mod 7);
///   2  1,999 characters: 1,500 shared characters, u's 16 digits, filler.
/// Row j's value: splitmix64fin(j + 0x5EED) mod 100,000 with its low bit cleared,
/// except the planted rows (odd values, which no other row holds):
///   39,595 in 1 row (j = n/2 + 1), 34,623 in 6 rows (j = 2 + 2i(n/12), i < 6),
///   74,037 in 1,000 rows (j mod (n/1,000) = n/1,000 - 1).
import Prim "mo:⛔";
import Map "mo:core/Map";
import _List "mo:core/List";
import Text "mo:core/Text";
import Nat "mo:core/Nat";
import Nat8 "mo:core/Nat8";
import Nat64 "mo:core/Nat64";
import Blob "mo:core/Blob";
import VarArray "mo:core/VarArray";
import Runtime "mo:core/Runtime";
import OQL "mo:oql";
import Table "mo:oql/Table";
import IndexedMap "mo:oql/IndexedMap";
import Executor "mo:oql/Executor";
import Registry "mo:oql/Registry";
import Json "mo:oql/Json";
import Query "mo:oql/Query";
import _Entity "mo:oql/Entity";
import _NatValue "mo:oql/NatValue";
import _TextValue "mo:oql/TextValue";
import _RecordValue "mo:oql/RecordValue";

actor self {
  type R = { id : Nat; k : Text };
  type Row = [(Text, OQL.Value)];

  var shape : Nat = 99;
  var rowsN : Nat = 0;
  var appended : Nat = 0;

  let tt = Table.new([("k", #text)], [("k", #hash)]);
  let im = IndexedMap.new<Nat, R>([("k", #hash)]);
  let mp = Map.empty<Text, Nat>();

  func ttRow(t : Text) : Row = [("k", #text t)];
  func rRow(r : R) : Row = [("id", #nat(r.id)), ("k", #text(r.k))];

  func fin(z0 : Nat64) : Nat64 {
    var z = z0;
    z := z ^ (z >> 30);
    z := z *% 0xBF58_476D_1CE4_E5B9;
    z := z ^ (z >> 27);
    z := z *% 0x94D0_49BB_1331_11EB;
    z ^ (z >> 31);
  };

  func fromBytes(a : [var Nat8]) : Text {
    let ?t = Text.decodeUtf8(Blob.fromVarArray(a)) else Runtime.trap("utf8");
    t;
  };

  // Shared prefix and the fillers, built once per canister by setup.
  transient var prefixT : Text = "";
  transient var fillers : [var Text] = [var];

  func prefixLen() : Nat = switch shape { case 0 1; case 1 40; case _ 1_500 };
  // Characters of the key of value u (the crate's key length less its terminator).
  func keyLen(u : Nat64) : Nat = switch shape {
    case 0 17;
    case 1 59 + Nat64.toNat(fin(u) % 141);
    case _ 1_999;
  };

  func prepare() {
    let p = prefixLen();
    prefixT := fromBytes(VarArray.tabulate<Nat8>(p, func i = if (shape == 0) 0x62 else Nat8.fromNat(0x61 + i % 23)));
    let maxLen = switch shape { case 0 17; case 1 199; case _ 1_999 };
    let minLen = switch shape { case 0 17; case 1 59; case _ 1_999 };
    let start = p + 16;
    fillers := VarArray.repeat<Text>("", maxLen + 1);
    var len = minLen;
    while (len <= maxLen) {
      fillers[len] := fromBytes(VarArray.tabulate<Nat8>(len - start, func i = Nat8.fromNat(0x71 + (start + i) % 7)));
      len += 1;
    };
  };

  func hex16(u : Nat64) : Text =
    fromBytes(VarArray.tabulate<Nat8>(16, func i = Nat8.fromNat(0x61 + Nat64.toNat((u >> Nat64.fromNat(4 * (15 - i))) & 15))));

  // A flat (blob-backed) text, as a key decoded from Candid is.
  func keyText(u : Nat64) : Text {
    if (prefixT == "") prepare();
    let t = prefixT # hex16(u) # fillers[keyLen(u)];
    let ?f = Text.decodeUtf8(Text.encodeUtf8(t)) else Runtime.trap("utf8");
    f;
  };

  func valueOf(j : Nat) : Nat64 {
    let n = rowsN;
    let m = n / 1_000;
    if (j == n / 2 + 1) return 39_595;
    let step = 2 * (n / 12);
    if (j >= 2 and (j - 2) % step == 0 and (j - 2) / step < 6) return 34_623;
    if (j % m == m - 1) return 74_037;
    (fin(Nat64.fromNat(j) +% 0x5EED) % 100_000) & 0xFFFF_FFFF_FFFF_FFFE;
  };

  public func setup(s : Nat, n : Nat) : async () {
    if (shape != 99) Runtime.trap("setup: already set up");
    shape := s;
    rowsN := n;
    prepare();
  };

  /// Append rows j in [from, from + count) on chain into the three structures.
  public func gen(from : Nat, count : Nat) : async Nat {
    var j = from;
    while (j < from + count) {
      let t = keyText(valueOf(j));
      ignore tt.append<Text>(t, ttRow);
      im.put(j, { id = j; k = t }, Nat.compare, rRow);
      switch (mp.get(Text.compare, t)) {
        case (?c) mp.add(Text.compare, t, c + 1);
        case null mp.add(Text.compare, t, 1);
      };
      j += 1;
    };
    appended += count;
    appended;
  };
  public func flush() : async () { tt.flush() };

  public query func keyOf(u : Nat64) : async Text { keyText(u) };
  public query func info() : async { shape : Nat; rows : Nat; appended : Nat; tableSize : Nat; mapSize : Nat; distinct : Nat; segments : Nat; heap : Nat; reclaimed : Nat; totalAllocation : Nat } {
    { shape; rows = rowsN; appended; tableSize = tt.size(); mapSize = im.size(); distinct = mp.size(); segments = tt.store().segments.size();
      heap = Prim.rts_heap_size(); reclaimed = Prim.rts_reclaimed(); totalAllocation = Prim.rts_total_allocation() };
  };
  public func tick() : async () {};
  public query func last() : async { mut : Nat64; gc : Nat64 } {
    { mut = Prim.rts_mutator_instructions().toNat64(); gc = Prim.rts_collector_instructions().toNat64() };
  };

  func unrestricted(_ : OQL.Decl) : OQL.Access = #unrestricted;
  transient let registry : Registry.Registry = Registry.build([
    tt.entityWith("ttext", "TText", "id").public_().build(),
    im.entity("mtext", "MText", "id", Nat.compare, rRow).public_().build(),
  ]);

  type JMeasure = { instrs : Nat; rows : Nat; value : Nat };
  func run(q : Query.Query) : JMeasure {
    let b = Prim.performanceCounter(0);
    let res = Executor.runWith(registry, q, unrestricted);
    let a = Prim.performanceCounter(0);
    let n = res.rows.size();
    let value = if (n > 0 and res.rows[0].size() > 0) {
      switch (res.rows[0][0].value) { case (#nat v) v; case _ 0 };
    } else 0;
    { instrs = Nat64.toNat(a - b); rows = n; value };
  };

  /// As `oqlbars.mo`'s `measure`: the query parsed from JSON outside the
  /// counter, the counter around `Executor.runWith`.
  public query func measure(qJson : Text) : async JMeasure {
    switch (Json.parseQuery(qJson)) {
      case (#err e) { Runtime.trap("measure: invalid query — " # e) };
      case (#ok q) run(q);
    };
  };

  /// The same count with the query built in Motoko around a key decoded from
  /// Candid (a flat text), the counter around `Executor.runWith` as above.
  public query func measureFlat(start : Text, key : Text) : async JMeasure {
    run({ start; where_ = ?#eq(["k"], #text key); groupBy = []; aggregate = [{ fn = #count; field = null; as_ = null }];
          orderBy = []; offset = null; limit = null; select = null });
  };

  /// mo:core Map<Text, Nat> get, the counter around `get`; key decoded from Candid.
  public query func mapGet(key : Text) : async { instrs : Nat; value : Nat } {
    let b = Prim.performanceCounter(0);
    let r = mp.get(Text.compare, key);
    let a = Prim.performanceCounter(0);
    { instrs = Nat64.toNat(a - b); value = switch r { case (?c) c; case null 0 } };
  };

  /// The same get with the key taken from a query parsed from JSON (the key
  /// the `measure` path compares with).
  public query func mapGetJson(qJson : Text) : async { instrs : Nat; value : Nat } {
    switch (Json.parseQuery(qJson)) {
      case (#ok q) {
        switch (q.where_) {
          case (?#eq(_, #text key)) {
            let b = Prim.performanceCounter(0);
            let r = mp.get(Text.compare, key);
            let a = Prim.performanceCounter(0);
            { instrs = Nat64.toNat(a - b); value = switch r { case (?c) c; case null 0 } };
          };
          case _ Runtime.trap("mapGetJson: not an eq on a text");
        };
      };
      case (#err e) Runtime.trap("mapGetJson: " # e);
    };
  };
};
