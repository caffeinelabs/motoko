/// index-present shapes; with oqlbars_extra2.bench.mo, the shapes not covered
/// by OQL's own suites.
///   gsa_idx        : group by (status, amount), count, IndexedMap with status #hash, amount #ordered (rows E)
///   gprice_idx     : group by a Float price (1,000 values), count, IndexedMap with price #ordered
///   eq_all_idx     : `z = 0` (every row matches), IndexedMap of M with z #hash, n #ordered
///   ge_all_idx     : `n >= 0` (every row matches), the same map
///   sort1_idx      : order by n (all rows), the same map (n #ordered)
///   sel1pct_idx    : `n < 1% of rows`, the same map
/// Query context: `mops bench oqlbars_extra --query`.
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
    e : Registry.Registry; p : Registry.Registry; m : Registry.Registry;
    n : Nat;
  };

  func build(n : Nat) : S {
    let em = IndexedMap.new<Nat, E>([("status", #hash), ("amount", #ordered)]);
    let pm = IndexedMap.new<Nat, P>([("price", #ordered)]);
    let mm = IndexedMap.new<Nat, M>([("z", #hash), ("n", #ordered)]);
    var k = 0;
    while (k < n) {
      em.put(k, mkE(k)); pm.put(k, mkP(k)); mm.put(k, mkM(k), Nat.compare, mRow);
      k += 1;
    };
    {
      e = Registry.build([em.entity("e", "E", "id").build()]);
      p = Registry.build([pm.entity("p", "P", "id").build()]);
      m = Registry.build([mm.entity("m", "M", "id", Nat.compare, mRow).build()]);
      n;
    }
  };

  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: index-present shapes");
    bench.description("Group by two columns and by a Float with indexes present; all-rows predicates and a sort with an index; 1% range.");
    let s10k = build(10_000); let s100k = build(100_000);
    func sFor(c : Text) : S = if (c == "10000") s10k else s100k;
    let cnt : Query.Agg = { fn = #count; field = null; as_ = null };
    let qGsa = { q0("e") with groupBy = [["status"], ["amount"]]; aggregate = [cnt] };
    let qGp = { q0("p") with groupBy = [["price"]]; aggregate = [cnt] };
    let qEqAll = { q0("m") with where_ = ?(#eq(["z"], #nat(0))) };
    let qGeAll = { q0("m") with where_ = ?(#ge(["n"], #nat(0))) };
    let qSort = { q0("m") with orderBy = [{ field = ["n"]; dir = #asc }] };
    func qSel(n : Nat) : Query.Query = { q0("m") with where_ = ?(#lt(["n"], #nat(n / 100))) };
    let qPend = { q0("q") with where_ = ?(#eq(["status"], #text("pending"))); limit = ?10 };
    // checks
    var sink = 0;
    bench.rows(["gsa_idx", "gprice_idx", "eq_all_idx", "ge_all_idx", "sort1_idx", "sel1pct_idx"]);
    bench.cols(["10000", "100000"]);
    bench.runner(func (row, c) {
      let s = sFor(c);
      switch row {
        case _ {
          let (reg, q, acc) = switch row {
            case "gsa_idx" (s.e, qGsa, unrestricted); case "gprice_idx" (s.p, qGp, unrestricted);
            case "eq_all_idx" (s.m, qEqAll, unrestricted); case "ge_all_idx" (s.m, qGeAll, unrestricted);
            case "sort1_idx" (s.m, qSort, unrestricted);
            case _ (s.m, qSel(s.n), unrestricted);
          };
          sink += Executor.runWith(reg, q, acc).rows.size();
        };
      };
    });
    bench;
  };
};
