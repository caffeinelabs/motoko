/// top-50 after deletes, queue reads after deletes or updates, and closures;
/// with oqlbars_extra.bench.mo, the shapes not covered by OQL's own suites.
///   top50_amt      : candidate API, top-50 by amount desc (L, amount #ordered), typed
///   top50_amt_del  : the same after deleting the 1% rows with the largest amounts (amount >= 990)
///   q_pend_idx / q_pend_scan         : `status = pending` limit 10 (pending one row in ten), status #hash / no index,
///                                      after the oldest 1% of rows were deleted (q_*) or updated to done (u_*)
///   own_scan / view_scan / scoped_scan / unr_scan : all rows of M read by a scoped subject through
///                                      `.ownedByWith` (canSee = true), `.viewWith` (identity), `newScoped`,
///                                      and unrestricted (no closure), count(*) is not used (it is served);
///                                      *_none: the same scans with `z = 1`, which matches no row (no output)
/// Query context: `mops bench oqlbars_extra2 --query`.
import Bench      "mo:bench";
import Array      "mo:core/Array";
import Float      "mo:core/Float";
import Iter       "mo:core/Iter";
import Map        "mo:core/Map";
import Nat        "mo:core/Nat";
import Principal  "mo:core/Principal";
import IndexedMap "../src/IndexedMap";
import _NatValue "../src/NatValue";
import _IntValue "../src/IntValue";
import _FloatValue "../src/FloatValue";
import _TextValue "../src/TextValue";
import _BoolValue "../src/BoolValue";
import _RecordValue "../src/RecordValue";
import Entity     "../src/Entity";
import Executor   "../src/Executor";
import Query      "../src/Query";
import Registry   "../src/Registry";
import OQL        "../src";

module {
  type E = { id : Nat; status : Text; amount : Nat };
  type P = { id : Nat; price : Float };
  type M = { id : Nat; n : Nat; i : Int; f : Float; z : Nat; t : Text; b : Bool; ref : Nat };
  type L = { id : Nat; ts : Nat; kind : Text; amount : Nat };
  type Q = { id : Nat; status : Text; amount : Nat; ts : Nat };

  let statuses = ["a", "b", "c"];
  func mkE(k : Nat) : E = { id = k; status = statuses[k % 3]; amount = (k * 7) % 1000 };
  func mkP(k : Nat) : P = { id = k; price = ((k * 7) % 1000).toFloat() / 100.0 };
  func mkM(k : Nat) : M = { id = k; n = k; i = k; f = Float.fromInt(k); z = 0; t = "benchmark"; b = (k % 2 == 0); ref = k };
  func mRow(m : M) : OQL.Entity.Row = [
    ("id", #nat(m.id)), ("n", #nat(m.n)), ("i", #int(m.i)), ("f", #float(m.f)),
    ("z", #nat(m.z)), ("t", #text(m.t)), ("b", #bool(m.b)), ("ref", #nat(m.ref)),
  ];
  func kindOf(k : Nat) : Text = if (k % 10 == 0) "burn" else if (k % 3 == 0) "mint" else "xfer";
  func mkL(k : Nat) : L = { id = k; ts = k; kind = kindOf(k); amount = (k * 7) % 1000 };
  func mkQ(k : Nat) : Q = { id = k; status = if (k % 10 == 0) "pending" else "other"; amount = k % 1000; ts = k };

  let unrestricted : Executor.Access = func (_ : OQL.Decl) : OQL.Access = #unrestricted;
  func subjectP() : Principal = Principal.fromText("aaaaa-aa");
  let scoped : Executor.Access = func (_ : OQL.Decl) : OQL.Access = #scoped(subjectP());
  func q0(start : Text) : Query.Query = { start; where_ = null; groupBy = []; aggregate = []; orderBy = []; offset = null; limit = null; select = null };

  type S = {
    l : IndexedMap.IndexedMap<Nat, L>; lDel : IndexedMap.IndexedMap<Nat, L>;
    qDelIdx : Registry.Registry; qDelScan : Registry.Registry; qUpdIdx : Registry.Registry; qUpdScan : Registry.Registry;
    own : Registry.Registry; view : Registry.Registry; scopedSrc : Registry.Registry; unr : Registry.Registry;
    n : Nat;
  };

  func build(n : Nat) : S {
    let lm = IndexedMap.new<Nat, L>([("amount", #ordered)]);
    let ld = IndexedMap.new<Nat, L>([("amount", #ordered)]);
    let qdi = IndexedMap.new<Nat, Q>([("status", #hash)]);
    let qds = IndexedMap.new<Nat, Q>([]);
    let qui = IndexedMap.new<Nat, Q>([("status", #hash)]);
    let qus = IndexedMap.new<Nat, Q>([]);
    let ms = Array.tabulate<M>(n, mkM);
    var k = 0;
    while (k < n) {
      lm.put(k, mkL(k)); ld.put(k, mkL(k));
      qdi.put(k, mkQ(k)); qds.put(k, mkQ(k)); qui.put(k, mkQ(k)); qus.put(k, mkQ(k));
      k += 1;
    };
    // the 1% rows with the largest amounts (amount >= 990) deleted
    k := 0;
    while (k < n) { if (mkL(k).amount >= 990) ld.delete(k); k += 1 };
    // the oldest 1% of the queue deleted / updated to done
    k := 0;
    while (k < n / 100) { qdi.delete(k); qds.delete(k); qui.put(k, { mkQ(k) with status = "done" }); qus.put(k, { mkQ(k) with status = "done" }); k += 1 };
    {
      l = lm; lDel = ld;
      qDelIdx = Registry.build([qdi.entity("q", "Q", "id").build()]);
      qDelScan = Registry.build([qds.entity("q", "Q", "id").build()]);
      qUpdIdx = Registry.build([qui.entity("q", "Q", "id").build()]);
      qUpdScan = Registry.build([qus.entity("q", "Q", "id").build()]);
      own = Registry.build([Entity.new<M>("m", func () = ms.values(), "M", "id", mRow).ownedByWith("t", func (_s : Principal, _o : OQL.Value) : Bool = true).scopedPerUser().build()]);
      view = Registry.build([Entity.new<M>("m", func () = ms.values(), "M", "id", mRow).ownedByWith("t", func (_s : Principal, _o : OQL.Value) : Bool = true).viewWith(func (_s : Principal, x : M) : M = x).scopedPerUser().build()]);
      scopedSrc = Registry.build([Entity.newScoped<M>("m", func (_ : ?Principal) : Iter.Iter<M> = ms.values(), "M", "id", mRow).scopedPerUser().build()]);
      unr = Registry.build([Entity.new<M>("m", func () = ms.values(), "M", "id", mRow).build()]);
      n;
    }
  };

  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: top-50 after deletes, queue reads, closures");
    bench.description("Top-50 by amount after deletes; pending-queue reads after deletes or updates, with and without an index; closure scans (owned, view, scoped, unrestricted), each also with a predicate no row matches.");
    let s10k = build(10_000); let s100k = build(100_000);
    func sFor(c : Text) : S = if (c == "10000") s10k else s100k;
    let cnt : Query.Agg = { fn = #count; field = null; as_ = null };
    let qGsa = { q0("e") with groupBy = [["status"], ["amount"]]; aggregate = [cnt] };
    let qGp = { q0("p") with groupBy = [["price"]]; aggregate = [cnt] };
    let qEqAll = { q0("m") with where_ = ?(#eq(["z"], #nat(0))) };
    let qGeAll = { q0("m") with where_ = ?(#ge(["n"], #nat(0))) };
    let qSort = { q0("m") with orderBy = [{ field = ["n"]; dir = #asc }] };
    func qSel(n : Nat) : Query.Query = { q0("m") with where_ = ?(#lt(["n"], #nat(n / 100))) };
    let qNone = { q0("m") with where_ = ?(#eq(["z"], #nat(1))) };   // scans every row, matches none
    let qPend = { q0("q") with where_ = ?(#eq(["status"], #text("pending"))); limit = ?10 };
    func top50(m : IndexedMap.IndexedMap<Nat, L>) : Nat {
      var c = 0; var acc = 0;
      label l for ((_, v) in m.candidatesRange("amount", null, null, #desc)) { acc += v.amount; c += 1; if (c == 50) break l };
      acc
    };
    // checks
    assert Executor.runWith(s10k.qDelIdx, qPend, unrestricted).rows.size() == 10;
    assert Executor.runWith(s10k.own, q0("m"), scoped).rows.size() == 10_000;
    var sink = 0;
    bench.rows(["top50_amt", "top50_amt_del", "q_pend_idx", "q_pend_scan", "u_pend_idx", "u_pend_scan", "own_scan", "view_scan", "scoped_scan", "unr_scan", "own_none", "view_none", "scoped_none", "unr_none"]);
    bench.cols(["10000", "100000"]);
    bench.runner(func (row, c) {
      let s = sFor(c);
      switch row {
        case "top50_amt" { sink += top50(s.l) };
        case "top50_amt_del" { sink += top50(s.lDel) };
        case _ {
          let (reg, q, acc) = switch row {
            case "q_pend_idx" (s.qDelIdx, qPend, unrestricted); case "q_pend_scan" (s.qDelScan, qPend, unrestricted);
            case "u_pend_idx" (s.qUpdIdx, qPend, unrestricted); case "u_pend_scan" (s.qUpdScan, qPend, unrestricted);
            case "own_scan" (s.own, q0("m"), scoped); case "view_scan" (s.view, q0("m"), scoped);
            case "scoped_scan" (s.scopedSrc, q0("m"), scoped); case "unr_scan" (s.unr, q0("m"), unrestricted);
            case "own_none" (s.own, qNone, scoped); case "view_none" (s.view, qNone, scoped);
            case "scoped_none" (s.scopedSrc, qNone, scoped); case _ (s.unr, qNone, unrestricted);
          };
          sink += Executor.runWith(reg, q, acc).rows.size();
        };
      };
    });
    bench;
  };
};
