/// The `ImportData` mixin. `include ImportData([...])` adds bulk-load endpoints for
/// the named columnar tables.
///
/// `tools/oql-ingest` IS THE ONLY SUPPORTED PRODUCER. These endpoints take bytes whose
/// structure is checked and whose contents are trusted, and they carry invariants — row
/// ranges tiling without overlap, index segments covering their bytes exactly, footers
/// believed as answers — that the tool upholds. The guards here exist so a BUG IN THAT TOOL
/// fails loudly and leaves a recoverable table, not to make this surface safe for an
/// arbitrary caller; a third-party uploader can wedge a load or produce wrong answers
/// without that being a defect here. Hence also: all CONTROLLER-ONLY — a bulk load writes raw bytes
/// into a Region and takes the footers on trust, so it belongs to the principal that
/// could install code anyway. Revoke it by removing the mixin.
///
///   layout(name)                                 — the column order a producer follows
///   rows(name)                                   — rows loaded: the resume cursor
///   putSegment(name, expectFirstRow, img)        — load one data segment
///   importFlush(name)                            — seal a partly filled buffer
///   putIndexChunk(name, col, kind, …)            — upload one index segment, in chunks
///   commitIndex(name, col)                       — commit it onto the column's base
///   indexCoverage(name, col)                     — rows the column's index covers: the resume cursor
///   reopenIndex(name, col)                       — reopen a committed index for a delta load
///   abortIndexUpload(name)                       — discard a half-uploaded segment
///   dropIndexBase(name, col, expectCoveredEnd)   — discard a COMMITTED base that cannot be finished
///   buildIndex(name, cols, kind)                 — build an index ON-CHAIN instead
///   indexStatus(name)                            — on-chain build progress
///
/// ```motoko
/// actor {
///   let ledger = Table.new([("amount_e8s", #nat), ("day", #nat)], []);
///   include Expose({ entities = [ ledger.entity("ledger").public_().build() ] });
///   include ImportData([ ledger.importTarget("ledger") ]);
/// };
/// ```
///
/// The tables stay declared in the actor body: they are the application's own
/// persistent state, and the same table `Expose` reads is the one written here.
/// Records share their mutable parts by reference, so the mixin holds nothing that
/// must survive an upgrade.
///
/// Idempotency without stored progress: every upload states the position it expects
/// the table to be at and traps if the table disagrees, so a retried ingress message
/// fails loudly instead of loading rows twice, and a producer that lost track
/// resynchronises with `rows`. One target's segments must therefore arrive in order;
/// separate targets are independent.
///
/// Two ways to get an index. A `#hash` index built off-chain arrives with the data
/// as index segments (`putIndexChunk`/`commitIndex`) and serves from the first query
/// after the segments tile the loaded rows. Anything else — a column the producer did
/// not index, a composite, `#ordered` — is built on-chain by `buildIndex`: the decl
/// registers pending so queries scan (a correct superset), a self-rescheduling timer
/// walks the store in bounded chunks, and the index serves once the walk completes.
/// An upgrade drops the timer, not the decl; call `buildIndex` again, the walk is
/// idempotent. `Table.loadSegment` refuses a table that already declares an index,
/// rather than let a ready index silently miss every loaded row.
///
/// LOADING MORE DATA LATER is a DELTA upload — new data segments plus index segments
/// covering only the new rows, never the whole index again. The order is:
///
///   1. `reopenIndex(name, col)` for EVERY region-indexed column. Each goes back to
///      pending and drops its heap decl, which is what re-satisfies the load gate
///      above; a column left ready keeps the gate shut, so a partial reopen fails
///      loudly rather than leaving a base that misses the new rows.
///   2. `putSegment` the new data, as on a first load, resuming from `rows(name)`.
///   3. Per column, `putIndexChunk`/`commitIndex` segments starting at
///      `indexCoverage(name, col)` — which still answers while reopened, and is
///      already the row the extension must continue from — until they reach
///      `rows(name)`, at which point the column serves again.
///
/// A reopened column keeps answering throughout: its base still covers `[0,
/// indexCoverage)` and serves that prefix, and only the rows past it are scanned, so
/// the cost is the UNCOVERED TAIL rather than the table. The two ranges are disjoint,
/// so nothing is counted twice.
///
/// What it stops taking is ordinary WRITES. `Table.append` traps between step 1 and
/// the commit that ends step 3, so a segment is the only way a position can enter.
/// Symmetrically, `reopenIndex` refuses a table whose rows arrived by `append` after
/// the index went ready, and `putSegment` refuses a table holding any appended row at
/// all — the producer's file cursor and the table's positions have drifted, and
/// loading on would index rows at positions they do not occupy.

import Prim      "mo:⛔";
import Auth      "Auth";
import Iter       "mo:core/Iter";
import List       "mo:core/List";
import Map        "mo:core/Map";
import Nat        "mo:core/Nat";
import RegionIndex "RegionIndex";
import Runtime   "mo:core/Runtime";
import Set       "mo:core/Set";
import Table     "Table";
import Text      "mo:core/Text";
import Timer     "mo:core/Timer";

mixin (targets : [Table.ImportTarget]) {

  /// Rebuilt on every upgrade: the tables this points at are the actor's own
  /// persistent fields, so nothing here needs to survive one.
  transient let importTargets : Map.Map<Text, Table.ImportTarget> = do {
    let m = Map.empty<Text, Table.ImportTarget>();
    for (t in targets.values()) {
      switch (m.get(Text.compare, t.name)) {
        case (?_) Runtime.trap("ImportData: duplicate target \"" # t.name # "\"");
        case null m.add(Text.compare, t.name, t);
      };
    };
    m;
  };

  // Baseline instrumentation (scratch copy only): `oqlbarsLastImport` holds the
  // in-method instructions of the last putSegment / putIndexChunk / commitIndex,
  // and the query `oqlbarsLastImportInstr` returns it.
  transient var oqlbarsLastImport : Nat64 = 0;
  public shared query func oqlbarsLastImportInstr() : async Nat64 { oqlbarsLastImport };

  func importAllowed(caller : Principal) : Bool =
    switch (Auth.resolve(#controllerOnly, caller)) { case (#unrestricted) true; case _ false };

  func importTargetOf(caller : Principal, name : Text) : Table.ImportTarget {
    if (not importAllowed(caller)) Runtime.trap("ImportData: bulk load is controller-only");
    switch (importTargets.get(Text.compare, name)) {
      case (?t) t;
      case null Runtime.trap("ImportData: no import target named \"" # name # "\"");
    };
  };

  /// The target's columns in storage order, and how many rows it holds. `null` for
  /// an unknown target, or a caller that may not load.
  public shared query ({ caller }) func layout(name : Text) : async ?Table.Layout {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) ?Table.layoutOf(t);
      case null null;
    };
  };

  /// The resume cursor: rows ever loaded, tombstones INCLUDED — the position the
  /// next segment will start at. A producer skips this many source rows on
  /// resume and passes it as the next `expectFirstRow`. It is `Table.appended`,
  /// NOT the live `size`: on a table that has had deletes the two differ, and
  /// resuming from the live count would re-send the deleted rows' source records
  /// as duplicates. Equal to the live count when nothing has been deleted, which
  /// is the ordinary bulk-load case.
  public shared query ({ caller }) func rows(name : Text) : async ?Nat {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) ?t.table.appended();
      case null null;
    };
  };

  /// The producer's FILE cursor: rows of `name` that arrived as a segment image, as
  /// opposed to by ordinary `append`. `rows` is the POSITION cursor — where the next
  /// segment begins — and the two are the same number only while every row came from
  /// the file, in order.
  ///
  /// A producer resuming a load needs both. It skips THIS many source records, and
  /// tells `putSegment` to expect `rows`. Using `rows` for both is what makes a table
  /// that has taken ordinary writes skip one source record per write and put every
  /// later one at a position it does not occupy — which no downstream check can catch,
  /// because the counts still balance.
  ///
  /// `null` for an unknown target or a caller that may not load.
  public shared query ({ caller }) func loadedRows(name : Text) : async ?Nat {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) ?t.table.loadedRows();
      case null null;
    };
  };

  /// Whether `col`'s region index is COMPLETE — its segments covered every position when
  /// it was committed. A producer needs this rather than comparing `indexCoverage` to
  /// `rows`: those stop being equal the moment the table takes an ordinary write, and a
  /// producer reading that as "unfinished" would skip the reopen the column in fact needs,
  /// and then be refused the load. `null` for an unknown target/column or a caller that
  /// may not load.
  public shared query ({ caller }) func indexReady(name : Text, col : Text) : async ?Bool {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) t.table.indexReady(col);
      case null null;
    };
  };

  /// `col`'s region index as a producer needs to see it: `#ready` complete, `#building` a
  /// first upload that never finished, `#extending` one reopened for a delta, `#none`
  /// absent. A producer must name EVERY column that is not `#none` — one left half-built
  /// stays that way while the run reports success. `null` for an unknown target/column or
  /// a caller that may not load.
  public shared query ({ caller }) func indexState(name : Text, col : Text) : async ?RegionIndex.ProducerState {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) t.table.indexState(col);
      case null null;
    };
  };

  /// Discard `col`'s committed region index, leaving the column unindexed (queries scan).
  /// The recovery path when an off-chain index can never be finished — a first upload
  /// overtaken by ordinary writes — since `buildIndex` needs the column clear and
  /// `abortIndexUpload` only drops the segment in flight. Returns false when there was no
  /// base. `expectCoveredEnd` is the `indexCoverage` the caller read: the drop applies only
  /// to that base, so a delayed retry cannot discard a replacement that has since committed
  /// over the same column. Controller-only.
  public shared ({ caller }) func dropIndexBase(name : Text, col : Text, expectCoveredEnd : Nat) : async Bool {
    let t = importTargetOf(caller, name);
    t.table.dropIndexBase(col, expectCoveredEnd);
  };

  /// Validate and place one segment image; returns the rows it added.
  ///
  /// `expectFirstRow` must equal the target's current row count. That is what
  /// makes a retried message safe: the second attempt sees a count that has moved
  /// on and traps, instead of appending the same rows again.
  public shared ({ caller }) func putSegment(name : Text, expectFirstRow : Nat, image : Blob) : async Nat {
    let oqlbarsT0 = Prim.performanceCounter(0);
    let t = importTargetOf(caller, name);
    // Against `appended` (positions assigned), not `size` (live rows): a segment
    // lands at the next position, which counts tombstones, so a table that has
    // had deletes must still be resumed from its position cursor.
    // `expectFirstRow` is where the segment's rows BELONG, not an assertion that the
    // table is already there. Equal to the position count it lands now; ahead of it
    // the segment is held until the gap fills, which is what lets a producer keep
    // several images in flight — messages from one caller have no execution order, so
    // without this the second of two in flight traps half the time. Behind it still
    // traps: those rows exist, and re-sending different ones for a filled range is a
    // producer bug worth failing loudly. A producer that lost track reads `rows()`.
    let before = t.table.appended();
    if (expectFirstRow < before) {
      Runtime.trap(
        "ImportData: segment starts at row " # expectFirstRow.toText()
        # " but \"" # name # "\" already holds " # before.toText()
        # " — resynchronise with rows()");
    };
    let oqlbarsR = t.table.loadSegmentAt(image, expectFirstRow);
    oqlbarsLastImport := Prim.performanceCounter(0) - oqlbarsT0;
    oqlbarsR;
  };

  /// Seal a partially filled write buffer into a segment. Only needed when rows
  /// also reached the table through `append`; a load made purely of images leaves
  /// no buffer behind.
  public shared ({ caller }) func importFlush(name : Text) : async () {
    let t = importTargetOf(caller, name);
    t.table.flush();
  };

  // A build message walks sub-chunks of rows until it has spent this many
  // instructions, then reschedules — adaptive, because per-row cost varies by an
  // order of magnitude with index kind and key type, so any fixed row count is
  // either wasteful or unsafe. 5B per message keeps a wide margin under the
  // update budget while a 100M-row build still finishes in tens of rounds.
  transient let BUILD_BUDGET : Nat64 = 5_000_000_000;
  transient let BUILD_SUBCHUNK = 50_000;

  /// In-flight builds, for progress reporting: target name → (columns-key →
  /// build). Keyed by columns so re-declaring the same index replaces its entry
  /// rather than piling up duplicates. Transient: the walk cursor is rebuilt by
  /// calling `buildIndex` again after an upgrade, and the decl's pending/ready
  /// state — the part correctness rests on — lives in the table.
  transient let building : Map.Map<Text, Map.Map<Text, Table.IndexBuild>> = Map.empty<Text, Map.Map<Text, Table.IndexBuild>>();

  func scheduleBuild<system>(t : Table.Table, st : Table.IndexBuild) {
    ignore Timer.setTimer<system>(#seconds 0, func () : async () {
      let start = Prim.performanceCounter(0);
      var done = t.buildStep(st, BUILD_SUBCHUNK);
      while (not done and Prim.performanceCounter(0) - start < BUILD_BUDGET) {
        done := t.buildStep(st, BUILD_SUBCHUNK);
      };
      if (not done) scheduleBuild<system>(t, st);
    });
  };

  /// Declare an index on a loaded table and build it in the background: one
  /// column is a single-column index, several are a composite in KEY order.
  /// Returns immediately; the planner keeps scanning until the build completes,
  /// so answers are correct throughout. Traps if the index is already declared
  /// AND ready, or if a column is not stored. Calling again while pending
  /// restarts the walk from the start — idempotent, and the restart path after
  /// an upgrade.
  public shared ({ caller }) func buildIndex(name : Text, cols : [Text], kind : Table.Kind) : async () {
    let t = importTargetOf(caller, name);
    if (cols.size() == 0) Runtime.trap("ImportData.buildIndex: no columns given");
    let pending = switch (cols.size()) {
      case 1 { t.table.ix.pending.contains(Text.compare, cols[0]) };
      case _ { var found = false; for (cs in t.table.ix.pending2.values()) { if (cs == cols) found := true }; found };
    };
    let st = if (pending) {
      // Already declared but never finished (an upgrade dropped the walker, or a
      // second call raced the first): a fresh walk over the same decl.
      { target = if (cols.size() == 1) #col(cols[0]) else #cols cols; var next = 0; var done = false } : Table.IndexBuild;
    } else if (cols.size() == 1) {
      t.table.addIndex(cols[0], kind);
    } else {
      t.table.addComposite(cols, kind);
    };
    let byCols = switch (building.get(Text.compare, name)) {
      case (?m) m;
      case null { let m = Map.empty<Text, Table.IndexBuild>(); building.add(Text.compare, name, m); m };
    };
    byCols.add(Text.compare, cols.values().join(","), st);   // replaces any prior build of the same columns
    scheduleBuild<system>(t.table, st);
  };

  /// Upload one ordered chunk of a pre-built index SEGMENT for `col`, covering the
  /// store positions `[firstRow, …)` (the off-chain builder emits one segment at a
  /// time after the data). Ordered by an expect-offset guard, the same idempotency
  /// as `putSegment`: a retried chunk traps rather than duplicating bytes. Not
  /// servable until the column's segments tile the loaded rows (see `commitIndex`).
  /// Returns the bytes filled so far. Controller-only.
  public shared ({ caller }) func putIndexChunk(name : Text, col : Text, kind : Table.Kind, firstRow : Nat, segLen : Nat, expectOffset : Nat, chunk : Blob) : async Nat {
    let oqlbarsT0 = Prim.performanceCounter(0);
    let t = importTargetOf(caller, name);
    let oqlbarsR = t.table.putIndexChunk(col, kind, firstRow, segLen, expectOffset, chunk);
    oqlbarsLastImport := Prim.performanceCounter(0) - oqlbarsT0;
    oqlbarsR;
  };

  /// Commit the staged segment onto `col`'s base. Once the committed segments tile
  /// `[0, loaded)` exactly the column serves point / IN / count / group queries
  /// from the region base merged with the heap delta; until then queries scan.
  /// Traps on a gap/overlap or a segment reaching past the loaded rows (the
  /// coverage guard). Returns the segment's row count. Controller-only. Order:
  /// load the data, commit each column's segments in ascending row order, then
  /// open the table for writes.
  public shared ({ caller }) func commitIndex(name : Text, col : Text) : async Nat {
    let oqlbarsT0 = Prim.performanceCounter(0);
    let t = importTargetOf(caller, name);
    let oqlbarsR = t.table.commitIndex(col);
    oqlbarsLastImport := Prim.performanceCounter(0) - oqlbarsT0;
    oqlbarsR;
  };

  /// Step 1 of the delta upload in the module header: reopen `col`'s committed index
  /// so more data can be loaded and covered by FURTHER segments. Returns false,
  /// changing nothing, when the column has no committed index or is already reopened.
  /// Rows APPENDED since it went ready are absorbed into the base first — the canister
  /// indexes its own rows, since no producer has them — a CHUNK PER CALL. `#absorbing n`
  /// means n rows remain: call again. No timer and no progress state; the cursor is the
  /// column's coverage, which is durable. Controller-only.
  public shared ({ caller }) func reopenIndex(name : Text, col : Text) : async Table.Reopen {
    let t = importTargetOf(caller, name);
    t.table.reopenIndex(col);
  };

  /// Segments received ahead of the gap before them and still waiting, for `name`.
  /// A load is complete when `rows()` is what was sent AND this is 0; a load that
  /// ends with this non-zero is missing a segment. `null` for an unknown target or a
  /// caller that may not load.
  public shared query ({ caller }) func stagedSegments(name : Text) : async ?Nat {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) ?t.table.stagedSegments();
      case null null;
    };
  };

  /// Drop every data segment still waiting on a gap for `name`, returning how many. The
  /// recovery path when a load stops mid-flight with images outstanding: those segments
  /// wait on a predecessor the producer may never send, and until they are gone the table
  /// refuses to flush and an index cannot be completed over it. Their rows were never
  /// counted, so resume from `rows()` afterwards. Controller-only.
  public shared ({ caller }) func dropStagedSegments(name : Text) : async Nat {
    let t = importTargetOf(caller, name);
    t.table.dropStagedSegments();
  };

  /// The index-side resume cursor: `col`'s committed segments tile `[0, n)`, so the
  /// next segment must start at row `n` and the producer skips that many source rows.
  /// 0 before the first commit, and it keeps answering while the column is still
  /// pending — which is the case a resume is for. Without it an interrupted index
  /// upload has nothing to restart from: re-sending from row 0 hits the coverage
  /// guard. A reopened column keeps its coverage, so this is equally the row a DELTA
  /// load's first extension segment must start at. `null` for an unknown target or
  /// column, or a caller that may not load.
  public shared query ({ caller }) func indexCoverage(name : Text, col : Text) : async ?Nat {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name)) {
      case (?t) switch (colIdxOf(t.table, col)) {
        case (?ci) ?RegionIndex.coveredTo(t.table.rix, ci);
        case null null;
      };
      case null null;
    };
  };

  // A column's storage position, by name: the region-index directory is keyed by it.
  func colIdxOf(t : Table.Table, col : Text) : ?Nat {
    var i = 0;
    for (n in t.names.values()) { if (n == col) return ?i; i += 1 };
    null;
  };

  /// Discard an in-flight index-segment upload for `name`, freeing its staged
  /// block, so a load interrupted mid-segment can be retried: while a partial
  /// segment is staged, `putIndexChunk` refuses a different one and `commitIndex`
  /// refuses the partial one. Returns whether there was an upload to discard.
  /// Controller-only.
  public shared ({ caller }) func abortIndexUpload(name : Text) : async Bool {
    let t = importTargetOf(caller, name);
    t.table.abortIndexUpload();
  };

  /// Build progress for a target's in-flight index builds, oldest first. An
  /// entry is done when `ready` is true; `walked`/`total` are store positions.
  /// Empty after an upgrade even if a decl is still pending — restart the build.
  public shared query ({ caller }) func indexStatus(name : Text) : async ?[{ columns : Text; ready : Bool; walked : Nat; total : Nat }] {
    if (not importAllowed(caller)) return null;
    switch (importTargets.get(Text.compare, name), building.get(Text.compare, name)) {
      case (?t, ?m) {
        let total = t.table.store.watermark + t.table.store.buffer.size();
        ?m.entries().map<(Text, Table.IndexBuild), { columns : Text; ready : Bool; walked : Nat; total : Nat }>(
          func ((columns, st)) = { columns; ready = st.done; walked = st.next; total }).toArray();
      };
      case (?_, null) ?[];
      case _ null;
    };
  };

};
