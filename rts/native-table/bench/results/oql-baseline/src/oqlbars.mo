/// Benchmark canister, and a worked example of exposing columnar `Table`s.
///
/// Four tables: a ledger with two secondary indexes, the same ledger shape with
/// none (which is what lets it reach 100M rows), and a customer/order pair joined
/// by an FK edge. All of them are served through `include Expose({...})`, so this
/// canister answers `schema()` and `execute(qJson)` like any OQL canister — the
/// registry the mixin builds is the SAME one every measurement below runs
/// against, so the numbers describe the path an application actually takes.
///
/// `execute` also means a long run can be inspected while it is still going:
/// a `count(*)` over the ledger is a query away, with no endpoint to add and no
/// redeploy. `measure(qJson)` is its instrumented twin — same arbitrary query,
/// plus the instructions it cost.
///
/// Rows are pushed in from the driver over many update calls, or generated
/// in-canister from a row index (see `bigRow`) when shipping them through Candid
/// would dominate. Queries run through real `query` endpoints, so the system's
/// query instruction limit applies. Instruction cost is measured in-canister with
/// the performance counter. account_spender is nullable, exercising the null path.
///
/// Deploy under PocketIC via bench/scale/bench.mjs.
import Prim "mo:⛔";
import Nat "mo:core/Nat";
import Principal "mo:core/Principal";
import Set "mo:core/Set";
import VarArray "mo:core/VarArray";
import Nat64 "mo:core/Nat64";
import List "mo:core/List";
import Region "mo:core/Region";
import OQL "../../src";
import Table "../../src/Table";
import Entity "../../src/Entity";
import Executor "../../src/Executor";
import Expose "../../src/Expose";
import ImportData "../../src/ImportData";
import Json "../../src/Json";
import Runtime "mo:core/Runtime";

actor self {
  // account_from, account_to, account_spender?, amount_e8s, day
  type InRow = (Nat, Nat, ?Nat, Nat, Nat);

  // Custom lineariser (the store's row-id is the key, so no id field here); the
  // nullable spender renders as #null_ — the columnar null path.
  func toRow((af, at, sp, amt, day) : InRow) : [(Text, OQL.Value)] = [
    ("account_from", #nat af),
    ("account_to", #nat at),
    ("account_spender", switch sp { case (?n) #nat n; case null #null_ }),
    ("amount_e8s", #nat amt),
    ("day", #nat day),
  ];

  let m = Table.new(
    [("account_from", #nat), ("account_to", #nat), ("account_spender", #nat), ("amount_e8s", #nat), ("day", #nat)],
    [("account_from", #hash), ("day", #hash)]);
  var loaded = 0;

  // Ingestion message: just append. The table buffers and auto-flushes into a
  // segment once the buffer fills, moving those rows off the heap on its own.
  public func loadRows(rows : [InRow]) : async Nat {
    for (r in rows.values()) ignore m.append(r, toRow);
    loaded += rows.size();
    loaded;
  };

  // Flush the trailing partial buffer after loading (auto-flush handles the rest).
  public func flush() : async () { m.flush() };

  // Every entity here is `.public_()`, which resolves to `#unrestricted` for any
  // caller. Measurements name that level directly rather than going through
  // `Auth.resolve`, so a benchmark never depends on who called it.
  func unrestricted(_ : OQL.Decl) : OQL.Access = #unrestricted;
  // `maxLive` is the collector's historical MAXIMUM live size, so during a churny
  // ingest it captures floating garbage that was still counted live at some
  // cycle — it is not the retained set (it can even exceed `residentHeap`). The
  // retained set is `totalAllocation - reclaimed`, which equals live data once a
  // GC cycle has completed with nothing new being allocated; the driver settles
  // the GC first and reports that.
  public query func stats() : async { loaded : Nat; residentHeap : Nat; maxLive : Nat; reclaimed : Nat; totalAllocation : Nat; regionBytes : Nat } {
    {
      loaded;
      residentHeap = Prim.rts_heap_size();
      maxLive = Prim.rts_max_live_size();
      reclaimed = Prim.rts_reclaimed();
      totalAllocation = Prim.rts_total_allocation();
      regionBytes = m.store().region.size().toNat() * 65_536;
    };
  };

  // ── Scale join: two columnar tables joined by an FK edge ────────────────────
  // A `customer` dimension (JCUSTOMERS rows) and an `order` fact table (up to 1M
  // rows) generated in-canister so every answer is a closed form. `order` edges
  // to `customer` by `customerId` (which equals the customer's dense store
  // position). Rows are generated over batched update calls; queries arrive as
  // real `query` calls, so the system's query instruction limit applies.
  //
  // The driver holds the query shapes, as OQL JSON sent to `measure`. Only shapes
  // that stay within the query instruction budget at 1M rows are worth running: a
  // full-row scan join is deliberately not among them, since materializing every
  // columnar row exceeds the ceiling. Cost is instead bounded by the index (point),
  // the footers (sum), an ordered prefix (top-N), or the size of the dimension a
  // semi-join materializes to traverse.
  let JCUSTOMERS = 100_000;
  let JREGIONS = 100;                 // ⟹ JCUSTOMERS/JREGIONS customers per region

  let customers = Table.new([("region", #nat), ("tier", #nat)], [("region", #hash)]);
  let orders = Table.new(
    [("customerId", #nat), ("amount", #nat), ("status", #nat)],
    [("customerId", #hash), ("amount", #ordered)]);
  var genCustomers_ = 0;
  var genOrders_ = 0;

  func custRow((region, tier) : (Nat, Nat)) : [(Text, OQL.Value)] = [("region", #nat region), ("tier", #nat tier)];
  func ordRow((cid, amount, status) : (Nat, Nat, Nat)) : [(Text, OQL.Value)] = [("customerId", #nat cid), ("amount", #nat amount), ("status", #nat status)];

  // Customer k (its store position): region = k mod JREGIONS, tier = k mod 3.
  public func genCustomers(fromK : Nat, count : Nat) : async Nat {
    var k = fromK; let e = fromK + count;
    while (k < e) { ignore customers.append<(Nat, Nat)>((k % JREGIONS, k % 3), custRow); k += 1 };
    genCustomers_ += count; genCustomers_;
  };
  // Order k: customerId = k mod JCUSTOMERS (each customer gets orders/JCUSTOMERS
  // of them), amount = k mod 1_000. status is the parity of the per-customer
  // order index (k / JCUSTOMERS), so it splits each customer's orders 50/50 and
  // stays independent of the customer's region (which is customerId mod 100) —
  // that independence is what makes the semi-join + status residual filter to
  // exactly half.
  public func genOrders(fromK : Nat, count : Nat) : async Nat {
    var k = fromK; let e = fromK + count;
    while (k < e) { ignore orders.append<(Nat, Nat, Nat)>((k % JCUSTOMERS, k % 1_000, (k / JCUSTOMERS) % 2), ordRow); k += 1 };
    genOrders_ += count; genOrders_;
  };
  public func joinFlush() : async () { customers.flush(); orders.flush() };

  type JMeasure = { instrs : Nat; rows : Nat; value : Nat };
  public query func joinStats() : async { customers : Nat; orders : Nat } { { customers = genCustomers_; orders = genOrders_ } };

  // ── Very large ingest: ledger rows generated in-canister ────────────────────
  // A 100M-row run cannot come from the fixture file — the driver would have to
  // hold and parse a compressed file far larger than memory allows — so the rows
  // are generated here instead, keeping every ingress message two numbers wide and
  // the rows deterministic in `k`.
  //
  // This table carries NO secondary index, which is what lets it reach that
  // size. An on-heap index replicates every row-id, and that heap cost grows with
  // the row count where the store's does not. Index-free, the row data lives
  // entirely off-heap in the Region, so the ceiling becomes the Region cap (raised by
  // `--max-stable-pages`) — and the query shapes that need no index still
  // answer: `count` off the live row count, `sum`/`avg` off the segment footers,
  // and a range on an unindexed column pruned by the per-segment zone maps.
  let big = Table.new(
    [("account_from", #nat), ("account_to", #nat), ("account_spender", #nat), ("amount_e8s", #nat), ("day", #nat)],
    []);
  var bigLoaded = 0;

  let BIG_ACCOUNTS : Nat64 = 100_000;
  let BIG_ROWS_PER_DAY = 3_000;

  // splitmix64's finaliser: a cheap deterministic mix, so row `k`'s values are
  // reproducible and evenly spread without storing anything.
  func mix(k : Nat, salt : Nat64) : Nat64 {
    var x = k.toNat64() +% salt +% 0x9E37_79B9_7F4A_7C15;
    x := (x ^ (x >> 30)) *% 0xBF58_476D_1CE4_E5B9;
    x := (x ^ (x >> 27)) *% 0x94D0_49BB_1331_11EB;
    x ^ (x >> 31);
  };

  // Row k, mirroring the fixture's shape: accounts spread over BIG_ACCOUNTS and
  // never a self-transfer, spender null on odd rows, a wide amount, and a fixed
  // run of rows per day (so `day` is ascending — which is what gives the zone
  // maps something to prune on).
  func bigRow(k : Nat) : InRow {
    let from = mix(k, 0) % BIG_ACCOUNTS;
    let t = mix(k, 11) % (BIG_ACCOUNTS - 1);
    let to = if (t >= from) t + 1 else t;
    let sp = if (k % 2 == 1) null else ?Nat64.toNat(mix(k, 22) % BIG_ACCOUNTS);
    (from.toNat(), to.toNat(), sp, (mix(k, 33) % 1_000_000_000_000).toNat(), k / BIG_ROWS_PER_DAY);
  };

  /// Append rows `fromK ..< fromK + count`. Called repeatedly by the driver, so
  /// one update message never approaches its instruction limit.
  /// `chunk` rows between collections, or 0 to append the whole `count` in one
  /// message. Use 0 when timing ingest throughput (collecting would dominate it)
  /// and a real chunk when measuring heap, where an uncollected ingest makes the
  /// retained figure meaningless. See `collect`.
  public func genLedger(fromK : Nat, count : Nat, chunk : Nat) : async Nat {
    var k = fromK;
    let e = fromK + count;
    while (k < e) {
      let stop = if (chunk == 0 or k + chunk >= e) e else k + chunk;
      var i = k;
      while (i < stop) { ignore big.append<InRow>(bigRow(i), toRow); i += 1 };
      k := stop;
      if (chunk != 0) await collect();
    };
    bigLoaded += count;
    bigLoaded;
  };
  public func bigFlush() : async () { big.flush() };

  /// Append the same generated rows into the INDEXED ledger table, so a heap
  /// measurement ingests at in-canister speed instead of shipping every row
  /// through Candid.
  ///
  /// `chunk` rows are appended between collections (see `collect`), so garbage
  /// from the append loop never outruns the collector. That is what makes a heap
  /// figure meaningful: with a whole ingest inside one message the collector never
  /// runs, so the retained figure afterwards reports live data plus everything
  /// that was never collected — several times the live set, not a per-row cost.
  public func genIndexed(fromK : Nat, count : Nat, chunk : Nat) : async Nat {
    var k = fromK;
    let e = fromK + count;
    while (k < e) {
      let stop = if (chunk == 0 or k + chunk >= e) e else k + chunk;
      var i = k;
      while (i < stop) { ignore m.append<InRow>(bigRow(i), toRow); i += 1 };
      k := stop;
      if (chunk != 0) await collect();
    };
    loaded += count;
    loaded;
  };

  // Collect, by calling this canister's own runtime-exported GC entry point. A
  // self-call rather than a bare `await async {}`: the latter only collects
  // because the build sets --force-gc, which makes every heap figure here depend
  // on a build flag with nothing in the code to say so. This asks for a
  // collection outright. No Prim exposes a synchronous collect, so a self-call is
  // the only way to do it from inside.
  func collect() : async () {
    let me : actor { __motoko_gc_trigger : () -> async () } =
      actor (Prim.principalOfActor(self).toText());
    await me.__motoko_gc_trigger();
  };

  /// Collect `n` times for ONE ingress call, instead of the driver making `n` of
  /// them. Used to settle the heap before a measurement.
  public func settle(n : Nat) : async () {
    var i = 0;
    while (i < n) { await collect(); i += 1 };
  };

  /// One ORDINARY append into `jcust` — the write a live canister makes
  /// between ingests. Exists so the harness can prove that a load AFTER one is refused:
  /// the producer's file cursor and the table's position cursor have drifted, and
  /// loading on would put the file's rows at shifted positions and index them wrongly.
  public func appendJord(key : Nat) : async Nat {
    jord.append<(Nat, Nat, Nat)>((key, 0, 0), ordRow);
  };

  public func appendJcust(region : Nat, tier : Nat) : async Nat {
    jcust.append<(Nat, Nat)>((region, tier), custRow);
  };

  public query func bigStats() : async { loaded : Nat; maxLive : Nat; reclaimed : Nat; totalAllocation : Nat; regionBytes : Nat; segments : Nat } {
    {
      loaded = big.size();   // the table's own count — rows arrive via genLedger AND the ImportData mixin
      maxLive = Prim.rts_max_live_size();
      reclaimed = Prim.rts_reclaimed();
      totalAllocation = Prim.rts_total_allocation();
      regionBytes = big.store().region.size().toNat() * 65_536;
      segments = big.store().segments.size();
    };
  };

  // ── Structure probes: what does one posting actually cost? ──────────────────
  // Measures what a container costs per-POSTING versus per-REF, which is what
  // decides a secondary index's heap footprint: build `nSets` sets of `perSet`
  // elements, hold them live, and let the driver settle the GC and read the
  // retained heap. Used against mo:core's Set, this is what showed a fixed
  // 31-slot element array dominating small postings — the finding behind
  // src/Posting.mo.
  var probeHeld : [var ?Set.Set<Nat>] = [var];

  public func probeBuildSets(nSets : Nat, perSet : Nat) : async Nat {
    let arr = VarArray.repeat<?Set.Set<Nat>>(null, nSets);
    var i = 0;
    while (i < nSets) {
      let s = Set.empty<Nat>();
      var j = 0;
      while (j < perSet) { s.add(Nat.compare, i * perSet + j); j += 1 };
      arr[i] := ?s;
      i += 1;
    };
    probeHeld := arr;
    nSets * perSet;
  };
  public func probeClear() : async () { probeHeld := [var] };
  public query func probeStats() : async { reclaimed : Nat; totalAllocation : Nat } {
    { reclaimed = Prim.rts_reclaimed(); totalAllocation = Prim.rts_total_allocation() };
  };


  // ── baseline-run additions ──────────────────────────────────────────────────────
  // A ledger table with ONE heap #hash index (account_from), written on chain.
  let m1 = Table.new(
    [("account_from", #nat), ("account_to", #nat), ("account_spender", #nat), ("amount_e8s", #nat), ("day", #nat)],
    [("account_from", #hash)]);
  public func genIndexed1(fromK : Nat, count : Nat, chunk : Nat) : async Nat {
    var k = fromK;
    let e = fromK + count;
    while (k < e) {
      let stop = if (chunk == 0 or k + chunk >= e) e else k + chunk;
      var i = k;
      while (i < stop) { ignore m1.append<InRow>(bigRow(i), toRow); i += 1 };
      k := stop;
      if (chunk != 0) await collect();
    };
    m1.size();
  };
  public func m1Flush() : async () { m1.flush() };
  /// Append the generated rows k in [fromK, fromK+count) into the bulk-loaded
  /// ledger (after its region index committed, they go to the heap delta).
  public func appendBig(fromK : Nat, count : Nat) : async Nat {
    var k = fromK;
    while (k < fromK + count) { ignore big.append<InRow>(bigRow(k), toRow); k += 1 };
    big.size();
  };
  type MO = { instr : Nat64; pmut : Nat64; pgc : Nat64; r : Nat64 };
  func perf() : Nat64 = Prim.performanceCounter(0);
  /// One measured operation per message; pmut/pgc are the PREVIOUS message's
  /// mutator and collector instructions (the runtime records them at the end of
  /// each update message).
  ///   1: Table.delete(ledger_unindexed, a)
  ///   2: b deletes in ledger_unindexed at positions (a + 7919 i) mod appended, one message
  ///   3: Table.delete(m1, a);  4: b deletes in m1 as 2
  ///   5: append one row to ledger_unindexed with account_from = a (returns its position)
  ///   9: delete positions a .. a+b-1 of ledger_unindexed in one message;
  ///   6: the same into m1;  7 / 8: b rows with account_from = a + i into ledger_unindexed / m1 (returns the last position);  99: nothing
  public func op(kind : Nat64, a64 : Nat64, b64 : Nat64) : async MO {
    let pmut = Prim.rts_mutator_instructions().toNat64();
    let pgc = Prim.rts_collector_instructions().toNat64();
    let a = a64.toNat(); let b = b64.toNat();
    var r : Nat = 0;
    let instr : Nat64 = switch kind {
      case 1 { let t0 = perf(); Table.delete(big, a); let t1 = perf(); t1 - t0 };
      case 2 { let n = big.appended(); let t0 = perf(); var i = 0; while (i < b) { Table.delete(big, (a + 7919 * i) % n); i += 1 }; let t1 = perf(); t1 - t0 };
      case 3 { let t0 = perf(); Table.delete(m1, a); let t1 = perf(); t1 - t0 };
      case 4 { let n = m1.appended(); let t0 = perf(); var i = 0; while (i < b) { Table.delete(m1, (a + 7919 * i) % n); i += 1 }; let t1 = perf(); t1 - t0 };
      case 5 { let row : InRow = (a, 1, null, 1, 0); let t0 = perf(); r := big.append<InRow>(row, toRow); let t1 = perf(); t1 - t0 };
      case 6 { let row : InRow = (a, 1, null, 1, 0); let t0 = perf(); r := m1.append<InRow>(row, toRow); let t1 = perf(); t1 - t0 };
      case 9 { let t0 = perf(); var i = 0; while (i < b) { Table.delete(big, a + i); i += 1 }; let t1 = perf(); t1 - t0 };
      case 7 { var i = 0; while (i < b) { r := big.append<InRow>((a + i, 1, null, 1, 0), toRow); i += 1 }; 0 };
      case 8 { var i = 0; while (i < b) { r := m1.append<InRow>((a + i, 1, null, 1, 0), toRow); i += 1 }; 0 };
      case _ 0;
    };
    { instr; pmut; pgc; r = r.toNat64() };
  };
  public func tick() : async () {};
  public query func last() : async { mut : Nat64; gc : Nat64 } {
    { mut = Prim.rts_mutator_instructions().toNat64(); gc = Prim.rts_collector_instructions().toNat64() };
  };
  public query func m1Stats() : async { size : Nat; segments : Nat; reclaimed : Nat; totalAllocation : Nat; heap : Nat } {
    { size = m1.size(); segments = m1.store().segments.size(); reclaimed = Prim.rts_reclaimed(); totalAllocation = Prim.rts_total_allocation(); heap = Prim.rts_heap_size() };
  };

  // ── Loading and serving the tables ──────────────────────────────────────────
  // Bulk load for the index-free ledger. `genLedger` above builds rows inside the
  // canister, which measures the append path with no ingress at all — a floor, not
  // a load. This is what an actual load does: a producer lays the rows out
  // off-chain and sends segment images, so no row crosses Candid. Same table
  // `Expose` serves below.
  // `csv_demo` exists for the CSV ingest tool's end-to-end test: text, float,
  // bool and a nullable nat — the cell types the ledger shape doesn't carry.
  let csvDemo = Table.new([("name", #text), ("score", #float), ("active", #bool), ("n", #nat), ("delta", #int)], []);

  // A JOIN pair that is bulk-loadable: index-free, so `putSegment` accepts images,
  // with the FK edge declared below. `customers`/`orders` above cannot serve this —
  // they declare their indexes at construction, which is exactly what makes
  // `loadSegment` refuse them. Both sides get their `#hash` index uploaded from
  // off-chain instead, which is the case a migration actually has: two tables, one
  // large and one small, joined on a key neither side indexed on-chain.
  let jcust = Table.new([("region", #nat), ("tier", #nat)], []);
  // `day` is MONOTONE in row position, so each segment's footer [min,max] is a
  // disjoint range and a `day = d` predicate skips whole segments without reading a
  // cell — the zone-map half of what the footers are for. The other columns cycle,
  // which makes their footer ranges span everything and prune nothing.
  let jord = Table.new([("customerId", #nat), ("amount", #nat), ("status", #nat), ("day", #nat)], []);

  include ImportData([
    big.importTarget("ledger_unindexed"),
    csvDemo.importTarget("csv_demo"),
    jcust.importTarget("jcust"),
    jord.importTarget("jord"),
  ]);

  // Test-only: delete a csv_demo row by store position, so a driver can exercise
  // resuming a bulk load onto a table that has had deletes — the case where the
  // resume cursor must be positions-assigned, not the live count.
  public func csvDeleteAt(pos : Nat) : async () { Table.delete(csvDemo, pos) };

  // The mixin adds `schema()` and `execute(qJson)`, and builds the `registry`
  // every measurement above runs against. It comes last because it reads the four
  // tables when the actor initialises, so they must already be declared.
  //
  // `.public_()` throughout: the rows are generated fixture data and the driver
  // calls in anonymously. A real canister would leave the default
  // (`#controllerOnly`) or scope per user — see `.ownedBy` / `.scopedPerUser`.
  //
  // The two ledgers are one shape under two names, `ledger` indexed and
  // `ledger_unindexed` not, so a query can be aimed at either and the pair is
  // what isolates what the secondary indexes cost.
  include Expose({
    entities = [
      m.entityWith("ledger", "Ledger", "id").public_().build(),
      m1.entityWith("ledger1", "Ledger1", "id").public_().build(),
      big.entityWith("ledger_unindexed", "LedgerUnindexed", "id").public_().build(),
      csvDemo.entityWith("csv_demo", "CsvDemo", "id").public_().build(),
      customers.entity("customer").public_().build(),
      orders.entity("order").edge("customerId", "customer").public_().build(),
      jcust.entityWith("jcust", "JCust", "id").public_().build(),
      jord.entityWith("jord", "JOrd", "id").edge("customerId", "jcust").public_().build(),
    ];
  });

  /// What an arbitrary query costs. `execute` answers the same query and returns
  /// its rows; this reports the instructions instead, and only the row COUNT plus
  /// the first aggregate cell — a shape whose result is 10k rows would otherwise
  /// spend more on encoding them into the reply than on running the query.
  ///
  /// The named endpoints above each pin one query shape in Motoko. This takes the
  /// shape from the caller, so a new question needs no new endpoint and no
  /// redeploy — and the JSON path is the one an application uses.
  ///
  /// Parsing happens before the counter starts, so the figure covers the same
  /// `runWith` window the pinned endpoints measure.
  public query func measure(qJson : Text) : async JMeasure {
    switch (Json.parseQuery(qJson)) {
      case (#err e) { Runtime.trap("measure: invalid query — " # e) };
      case (#ok q) {
        let b = Prim.performanceCounter(0);
        let res = Executor.runWith(registry, q, unrestricted);
        let a = Prim.performanceCounter(0);
        let n = res.rows.size();
        // First cell of the first row, when it is a #nat: for an aggregate query
        // that is the aggregate, whatever it was named.
        let value = if (n > 0 and res.rows[0].size() > 0) {
          switch (res.rows[0][0].value) { case (#nat v) v; case _ 0 };
        } else 0;
        { instrs = Nat64.toNat(a - b); rows = n; value };
      };
    };
  };
};