/// update-context harness (target canister). One canister holds one
/// configuration (`setup`), is pre-filled with batched messages (`prefill`), and
/// then runs lone operations, one per message (`op`), each read with
/// performanceCounter(0) right before and right after the OQL / core call.
/// The row is built before the first counter read (it is the message's input,
/// not OQL's work). The drivers read per message: the in-method count (reply),
/// the whole message's mutator and collector instructions (`last`, a query
/// after the message), and the cycles the message cost. `wseq.mjs` sends each
/// `op` to this canister directly, as an ingress message, and subtracts the
/// ingress fee, which it calibrates with a no-op `op` sent both directly and
/// through the relay canister (`relay.mo`); `wbatch.mjs` and `paidorders.mjs`
/// send their measured message through the relay, so no ingress fee lands on
/// this canister. `wseq.mjs` flags a message structural when `sh(k)`, a hash of
/// the height and the internal node counts along every B-tree path the key
/// touches (for a Table, also its segment count), differs before and after it.
/// `sig(k)` returns the same path data as a list; no driver calls it.
///
/// Reference row: Int timestamp, Nat amount, a
/// 3-case tag (OQL has no variant Value: a 3-value Text), a Principal, a 10-60 B
/// Text, a Bool; Nat key. OQL's own implicit `_toRow` derivation is used
/// (RecordValue + the per-type Value modules), as OQL's benches do.
import Prim "mo:prim";
import Array "mo:core/Array";
import VarArray "mo:core/VarArray";
import List "mo:core/List";
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Nat64 "mo:core/Nat64";
import Nat8 "mo:core/Nat8";
import Nat32 "mo:core/Nat32";
import Int "mo:core/Int";
import Text "mo:core/Text";
import Order "mo:core/Order";
import Runtime "mo:core/Runtime";
import CT "mo:core/Types";
import IndexedMap "mo:oql/IndexedMap";
import Backfill "mo:oql/Backfill";
import Table "mo:oql/Table";
import Predicate "mo:oql/Predicate";
import Types "mo:oql/Types";
import _NatValue "mo:oql/NatValue";
import _IntValue "mo:oql/IntValue";
import _TextValue "mo:oql/TextValue";
import _BoolValue "mo:oql/BoolValue";
import _PrincipalValue "mo:oql/PrincipalValue";
import _RecordValue "mo:oql/RecordValue";

persistent actor {

  type R = { ts : Int; amount : Nat; kind : Text; owner : Principal; note : Text; flag : Bool };
  type L = { id : Nat; ts : Nat; kind : Text; amount : Nat };
  type T = { id : Nat; pair : Text; price : Nat; ts : Nat };
  type M = { instr : Nat64; pmut : Nat64; pgc : Nat64; hb : Nat; ha : Nat };
  type Value = Types.Value;

  let TS0 : Int = 1_760_000_000_000_000_000;
  let TS_MOVE : Int = 1_000_000_000_000; // a replace moves ts by this much

  // 1,000 owners (29-byte self-authenticating-sized principals) and 51 note
  // texts of 10..60 bytes, built once.
  transient let owners : [Principal] = Array.tabulate<Principal>(1_000, func i {
    Prim.principalOfBlob(Prim.arrayToBlob(Array.tabulate<Nat8>(29, func j = Nat8.fromNat((i * 31 + j * 7 + 3) % 256))))
  });
  transient let notes : [Text] = Array.tabulate<Text>(51, func i {
    var t = "";
    var j = 0;
    while (j < 10 + i) { t #= Prim.charToText(Prim.nat32ToChar(Nat32.fromNat(97 + (i + j) % 26))); j += 1 };
    t
  });

  func kindOf(k : Nat) : Text = if (k % 10 == 0) "burn" else if (k % 3 == 0) "mint" else "xfer";
  func mkR(k : Nat) : R = {
    ts = TS0 + k * 1_000;
    amount = (k * 7) % 1_000;
    kind = kindOf(k);
    owner = owners[k % 1_000];
    note = notes[k % 51];
    flag = k % 2 == 0;
  };
  func mkL(k : Nat) : L = { id = k; ts = k; kind = kindOf(k); amount = (k * 7) % 1000 };
  let mids = ["BTC/USD", "ETH/USD", "SOL/USD", "DOGE/USD", "ADA/USD"];
  func mkT(k : Nat) : T {
    let r = k % 20;
    let pair = if (r < 8) "ICP/USD" else if (r < 18) mids[(r - 8) % 5] else if (r == 18) "XRP/USD" else "GLDT/USD";
    { id = k; pair; price = (k * 7919) % 10_000; ts = k }
  };
  // 27-byte Text key: "account-" + 19 zero-padded digits.
  func textKey(k : Nat) : Text {
    let d = k.toText();
    var pad = "";
    var i = d.size();
    while (i < 19) { pad #= "0"; i += 1 };
    "account-" # pad # d
  };

  let TB_COLS : [Table.Column] = [("ts", #int), ("amount", #nat), ("kind", #text), ("owner", #text), ("note", #text), ("flag", #bool)];

  var cfg : Text = "";
  var cm = Map.empty<Nat, R>();
  var cmt = Map.empty<Text, R>();
  var im : IndexedMap.IndexedMap<Nat, R> = IndexedMap.new<Nat, R>([]);
  var tb : Table.Table = Table.new(TB_COLS, []);
  // batched IndexedMap writes and index backfill (rows L and T)
  var iml : IndexedMap.IndexedMap<Nat, L> = IndexedMap.new<Nat, L>([]);
  var imt : IndexedMap.IndexedMap<Nat, T> = IndexedMap.new<Nat, T>([]);
  var calibBuf : [var Nat] = [var];

  func perf() : Nat64 = Prim.performanceCounter(0);

  func idxFor(c : Text) : [(Text, IndexedMap.Kind)] = switch c {
    case "im0" []; case "im1" [("ts", #ordered)];
    case "im2" [("ts", #ordered), ("kind", #hash)];
    case "im3" [("ts", #ordered), ("kind", #hash), ("amount", #ordered)];
    case "im1a" [("amount", #ordered)];
    case "tba" [("amount", #ordered)];
    case "tbt" [("ts", #ordered)];
    case _ [];
  };

  public func setup(c : Text) : async () {
    cfg := c;
    cm := Map.empty<Nat, R>();
    cmt := Map.empty<Text, R>();
    im := IndexedMap.new<Nat, R>(idxFor(c));
    tb := Table.new(TB_COLS, if (c == "tb" or c == "tba" or c == "tbt") idxFor(c) else []);
    // only the calibration configuration holds the 8 MiB buffer (it would add to every collection)
    calibBuf := if (c == "calib") VarArray.repeat<Nat>(0, 512 * 2_048) else [var];
  };

  /// Insert keys [from, from+count) (batched, not measured).
  public func prefill(from : Nat64, count : Nat64) : async Nat {
    var k = from.toNat();
    let e = k + count.toNat();
    switch cfg {
      case "cm" { while (k < e) { cm.add(Nat.compare, k, mkR(k)); k += 1 }; cm.size() };
      case "cmt" { while (k < e) { cmt.add(Text.compare, textKey(k), mkR(k)); k += 1 }; cmt.size() };
      case ("tb" or "tba" or "tbt") { while (k < e) { ignore tb.append(mkR(k)); k += 1 }; tb.size() };
      case _ { while (k < e) { im.put(k, mkR(k)); k += 1 }; im.size() };
    };
  };

  /// One lone operation. kind 0 insert of a new key k (Table: append), 1 replace
  /// of key k moving ts (core Map: add over an existing key), 2 remove/delete k.
  public func op(kind : Nat64, k64 : Nat64) : async M {
    let pmut = Prim.rts_mutator_instructions().toNat64();
    let pgc = Prim.rts_collector_instructions().toNat64();
    let instr = opInner(kind, k64.toNat());
    { instr; pmut; pgc; hb = 0; ha = 0 };
  };

  func opInner(kind : Nat64, k : Nat) : Nat64 {
    if (kind == 99) return 0;
    switch cfg {
      case "cm" {
        if (kind == 0) { let r = mkR(k); let t0 = perf(); cm.add(Nat.compare, k, r); let t1 = perf(); t1 - t0 }
        else if (kind == 1) { let r = { mkR(k) with ts = TS0 + k * 1_000 + TS_MOVE }; let t0 = perf(); cm.add(Nat.compare, k, r); let t1 = perf(); t1 - t0 }
        else { let t0 = perf(); cm.remove(Nat.compare, k); let t1 = perf(); t1 - t0 };
      };
      case "cmt" {
        let key = textKey(k);
        if (kind == 0) { let r = mkR(k); let t0 = perf(); cmt.add(Text.compare, key, r); let t1 = perf(); t1 - t0 }
        else if (kind == 1) { let r = { mkR(k) with ts = TS0 + k * 1_000 + TS_MOVE }; let t0 = perf(); cmt.add(Text.compare, key, r); let t1 = perf(); t1 - t0 }
        else { let t0 = perf(); cmt.remove(Text.compare, key); let t1 = perf(); t1 - t0 };
      };
      case ("tb" or "tba" or "tbt") {
        if (kind == 0) { let r = mkR(k); let t0 = perf(); ignore tb.append(r); let t1 = perf(); t1 - t0 }
        else if (kind == 2) { let t0 = perf(); Table.delete(tb, k); let t1 = perf(); t1 - t0 }
        else Runtime.trap("Table has no update in place");
      };
      case _ {
        if (kind == 0) { let r = mkR(k); let t0 = perf(); im.put(k, r); let t1 = perf(); t1 - t0 }
        else if (kind == 1) { let r = { mkR(k) with ts = TS0 + k * 1_000 + TS_MOVE }; let t0 = perf(); im.put(k, r); let t1 = perf(); t1 - t0 }
        else { let t0 = perf(); im.delete(k); let t1 = perf(); t1 - t0 };
      };
    };
  };

  // Structural hash of the touched B-tree paths, computed without allocating:
  // the height and each internal node's count along the path toward the key's
  // leaf (an equal key descends to its left, so the path does not depend on
  // whether the key is present). A node split or merge on the path changes it.
  func hmix(h : Nat, x : Nat) : Nat = (h * 31 + x) % 1_000_000_007;
  func pathHash<K, V>(h0 : Nat, m : Map.Map<K, V>, cmp : (K, K) -> Order.Order, k : K) : Nat {
    var h = hmix(h0, height(m));
    var node : CT.Map.Node<K, V> = m.root;
    label walk loop {
      switch node {
        case (#leaf _) break walk;
        case (#internal int) {
          h := hmix(h, int.data.count);
          var j = 0;
          label s while (j < int.data.count) {
            switch (int.data.kvs[j]) {
              case (?(kk, _)) { switch (cmp(k, kk)) { case (#greater) j += 1; case _ break s } };
              case null break s;
            };
          };
          switch (int.children[j]) { case (?c) node := c; case null break walk };
        };
      };
    };
    h
  };
  func shash(k : Nat) : Nat {
    switch cfg {
      case "cm" pathHash(7, cm, Nat.compare, k);
      case "cmt" pathHash(7, cmt, Text.compare, textKey(k));
      case ("tb" or "tba" or "tbt") {
        var h = hmix(7, tb.store().segments.size());
        let ts = TS0 + k * 1_000;
        let am = (k * 7) % 1_000;
        for ((col, _) in idxFor(cfg).values()) {
          switch (tb.ix.byCol.get(Text.compare, col)) {
            case (?ci) { if (col == "ts") { h := pathHash(h, ci, Predicate.compare, #int(ts)); h := pathHash(h, ci, Predicate.compare, #int(ts + TS_MOVE)) } else h := pathHash(h, ci, Predicate.compare, #nat(am)) };
            case null {};
          };
        };
        h
      };
      case _ {
        var h = pathHash(7, im.inner, Nat.compare, k);
        let ts = TS0 + k * 1_000;
        let am = (k * 7) % 1_000;
        for ((col, _) in idxFor(cfg).values()) {
          switch (im.ix.byCol.get(Text.compare, col)) {
            case (?ci) {
              if (col == "ts") { h := pathHash(h, ci, Predicate.compare, #int(ts)); h := pathHash(h, ci, Predicate.compare, #int(ts + TS_MOVE)) }
              else if (col == "kind") h := pathHash(h, ci, Predicate.compare, #text(kindOf(k)))
              else h := pathHash(h, ci, Predicate.compare, #nat(am));
            };
            case null {};
          };
        };
        h
      };
    };
  };

  /// Calibration: `n` loop iterations of scalar work, then one write into each
  /// of `pages` distinct 4 KiB pages of a pre-allocated buffer.
  public func calib(n : Nat64, pages : Nat64) : async M {
    let t0 = perf();
    var i : Nat = 0;
    var acc : Nat = 0;
    let nn = n.toNat();
    while (i < nn) { acc := (acc + i) % 1_000_003; i += 1 };
    var p = 0;
    let pp = pages.toNat();
    while (p < pp) { calibBuf[p * 512] += 1 + acc % 2; p += 1 };
    let t1 = perf();
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };

  public func tick() : async () {};

  /// The last update message's mutator instructions (everything before its
  /// end-of-message collection, decoding and reply included) and collector
  /// instructions, as Motoko's runtime records them.
  public query func last() : async { mut : Nat64; gc : Nat64 } {
    { mut = Prim.rts_mutator_instructions().toNat64(); gc = Prim.rts_collector_instructions().toNat64() };
  };

  public query func stats() : async { size : Nat; heap : Nat; alloc : Nat; reclaimed : Nat; segs : Nat } {
    let size = switch cfg { case "cm" cm.size(); case "cmt" cmt.size(); case ("tb" or "tba" or "tbt") tb.size(); case _ im.size() };
    { size; heap = Prim.rts_heap_size(); alloc = Prim.rts_total_allocation(); reclaimed = Prim.rts_reclaimed(); segs = tb.store().segments.size() };
  };

  // ── structural signature ────────────────────────────────────────────────────
  // Per map touched: [height, count of each internal node on the search path of
  // the key ..., SEP]. A node split or merge on the path changes an internal
  // node's count or the height; an insert or remove in a leaf changes neither.
  let SEP = 999_999_999;
  func height<K, V>(m : Map.Map<K, V>) : Nat {
    var node : CT.Map.Node<K, V> = m.root;
    var h = 1;
    label w loop {
      switch node {
        case (#leaf _) break w;
        case (#internal int) { h += 1; switch (int.children[0]) { case (?c) node := c; case null break w } };
      };
    };
    h
  };
  func sigInto<K, V>(out : List.List<Nat>, m : Map.Map<K, V>, cmp : (K, K) -> Order.Order, k : K) {
    out.add(height(m));
    var node : CT.Map.Node<K, V> = m.root;
    label walk loop {
      switch node {
        case (#leaf _) break walk;
        case (#internal int) {
          out.add(int.data.count);
          var j = 0;
          var found = false;
          label s while (j < int.data.count) {
            switch (int.data.kvs[j]) {
              case (?(kk, _)) { switch (cmp(k, kk)) { case (#less) break s; case (#equal) { found := true; break s }; case (#greater) j += 1 } };
              case null break s;
            };
          };
          if (found) break walk;
          switch (int.children[j]) { case (?c) node := c; case null break walk };
        };
      };
    };
    out.add(SEP);
  };
  public query func sh(k64 : Nat64) : async Nat { shash(k64.toNat()) };

  public query func sig(k64 : Nat64) : async [Nat] {
    let k = k64.toNat();
    let out = List.empty<Nat>();
    switch cfg {
      case "cm" sigInto(out, cm, Nat.compare, k);
      case "cmt" sigInto(out, cmt, Text.compare, textKey(k));
      case ("tb" or "tba" or "tbt") {
        out.add(tb.store().segments.size()); out.add(SEP);
        let r = mkR(k);
        for ((col, _) in idxFor(cfg).values()) {
          switch (tb.ix.byCol.get(Text.compare, col)) {
            case (?ci) {
              let (v1, v2) : (Value, Value) = if (col == "ts") (#int(r.ts), #int(r.ts + TS_MOVE)) else (#nat(r.amount), #nat(r.amount));
              sigInto(out, ci, Predicate.compare, v1);
              sigInto(out, ci, Predicate.compare, v2);
            };
            case null {};
          };
        };
      };
      case _ {
        sigInto(out, im.inner, Nat.compare, k);
        let r = mkR(k);
        for ((col, _) in idxFor(cfg).values()) {
          switch (im.ix.byCol.get(Text.compare, col)) {
            case (?ci) {
              let (v1, v2) : (Value, Value) =
                if (col == "ts") (#int(r.ts), #int(r.ts + TS_MOVE))
                else if (col == "kind") (#text(r.kind), #text(r.kind))
                else (#nat(r.amount), #nat(r.amount));
              sigInto(out, ci, Predicate.compare, v1);
              sigInto(out, ci, Predicate.compare, v2);
            };
            case null {};
          };
        };
      };
    };
    out.toArray();
  };

  // ── batched writes and index backfill: one message, the whole pass, in the method ──
  func lIdx(n : Nat) : [(Text, IndexedMap.Kind)] = switch n {
    case 0 []; case 1 [("ts", #ordered)]; case 2 [("ts", #ordered), ("kind", #hash)];
    case _ [("ts", #ordered), ("kind", #hash), ("amount", #ordered)];
  };
  /// Batched inserts (wbatch.mjs cells build_0idx .. build_3idx): a fresh
  /// IndexedMap of L with `nidx` indexes, then n inserts; the map
  /// stays live (held by the actor).
  public func wBuild(nidx : Nat64, n : Nat64) : async M {
    let t0 = perf();
    let m = IndexedMap.new<Nat, L>(lIdx(nidx.toNat()));
    var k = 0;
    let nn = n.toNat();
    while (k < nn) { m.put(k, mkL(k)); k += 1 };
    let t1 = perf();
    iml := m;
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };
  /// Batched re-indexing updates (cell reindex_2): n updates moving ts on the
  /// held map (built by wBuild with 2 indexes).
  public func wReindex(n : Nat64) : async M {
    let nn = n.toNat();
    let t0 = perf();
    var k = 0;
    while (k < nn) { iml.put(k, { mkL(k) with ts = k + nn }); k += 1 };
    let t1 = perf();
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };
  /// Batched deletes (cell delete_2): n deletes on the held map.
  public func wDelete(n : Nat64) : async M {
    let nn = n.toNat();
    let t0 = perf();
    var k = 0;
    while (k < nn) { iml.delete(k); k += 1 };
    let t1 = perf();
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };
  /// Batched inserts of T (cells buildT_0idx, buildT_price, buildT_comp): a
  /// fresh map of T with `which` = 0 no index, 1 price #ordered, 2 the
  /// composite [pair, price]; n inserts.
  public func wBuildT(which : Nat64, n : Nat64) : async M {
    let t0 = perf();
    let m = if (which == 0) IndexedMap.newWith<Nat, T>([], [])
      else if (which == 1) IndexedMap.newWith<Nat, T>([("price", #ordered)], [])
      else IndexedMap.newWith<Nat, T>([], [(["pair", "price"], #ordered)]);
    var k = 0;
    let nn = n.toNat();
    while (k < nn) { m.put(k, mkT(k)); k += 1 };
    let t1 = perf();
    imt := m;
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };
  /// Index backfill (cells backfill_price, backfill_comp): on the held T map
  /// (built with no index), addIndex / addComposite and
  /// the Backfill walk in 10^4-row steps, all in this message.
  public func wBackfill(which : Nat64) : async M {
    let t0 = perf();
    let st = if (which == 1) imt.addIndex("price", #ordered) else imt.addComposite(["pair", "price"], #ordered);
    while (not Backfill.step(imt, st, 10_000)) {};
    let t1 = perf();
    { instr = t1 - t0; pmut = 0; pgc = 0; hb = 0; ha = 0 };
  };
};
