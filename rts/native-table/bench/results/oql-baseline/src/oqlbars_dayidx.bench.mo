/// one day of a ledger-like table through an index on `day`, 10^5 rows
/// (day = k / 3,000: 3,000 rows per day; status = k mod 2; amount = 7k mod 1000).
///   range_day_ord  : count where day >= 17 and day <= 17, IndexedMap with `day #ordered`
///                    (a range count is not served from the index's statistics: the range is streamed)
///   eq_day_hash    : count where day = 17, IndexedMap with `day #hash` (served: the posting's size)
///   group_day_hash : day = 17, group by status, count and sum(amount), IndexedMap with `day #hash`
///   group_day_ord  : the same with `day #ordered`
///   range_day_none / group_day_none : the same two queries with no index (scan)
/// Query context: `mops bench oqlbars_dayidx --query`.
import Bench      "mo:bench";
import Map        "mo:core/Map";
import Nat        "mo:core/Nat";
import IndexedMap "../src/IndexedMap";
import _NatValue "../src/NatValue";
import _RecordValue "../src/RecordValue";
import Entity     "../src/Entity";
import Executor   "../src/Executor";
import Query      "../src/Query";
import Registry   "../src/Registry";
import OQL        "../src";

module {
  type D = { id : Nat; day : Nat; status : Nat; amount : Nat };
  func mk(k : Nat) : D = { id = k; day = k / 3_000; status = k % 2; amount = (k * 7) % 1_000 };
  let unrestricted : Executor.Access = func (_ : OQL.Decl) : OQL.Access = #unrestricted;
  func q0() : Query.Query = { start = "d"; where_ = null; groupBy = []; aggregate = []; orderBy = []; offset = null; limit = null; select = null };
  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: one day through an index on day");
    bench.description("Range count over one day (#ordered), eq count (#hash), one day grouped by status with count and sum; 10^5 rows, 3,000 per day.");
    let n = 100_000;
    let mo = IndexedMap.new<Nat, D>([("day", #ordered)]);
    let mh = IndexedMap.new<Nat, D>([("day", #hash)]);
    let plain = Map.empty<Nat, D>();
    var k = 0;
    while (k < n) { let d = mk(k); mo.put(k, d); mh.put(k, d); plain.add(Nat.compare, k, d); k += 1 };
    let ro = Registry.build([mo.entity("d", "D", "id").build()]);
    let rh = Registry.build([mh.entity("d", "D", "id").build()]);
    let rn = Registry.build([Entity.new<D>("d", func () = plain.values(), "D", "id").build()]);
    let cnt : Query.Agg = { fn = #count; field = null; as_ = null };
    let sm : Query.Agg = { fn = #sum; field = ?["amount"]; as_ = null };
    let qRange = { q0() with where_ = ?(#and_([#ge(["day"], #nat(17)), #le(["day"], #nat(17))])); aggregate = [cnt] };
    let qEq = { q0() with where_ = ?(#eq(["day"], #nat(17))); aggregate = [cnt] };
    let qGroup = { q0() with where_ = ?(#eq(["day"], #nat(17))); groupBy = [["status"]]; aggregate = [cnt, sm] };
    var sink = 0;
    bench.rows(["range_day_ord", "eq_day_hash", "group_day_hash", "group_day_ord", "range_day_none", "group_day_none"]);
    bench.cols(["100000"]);
    bench.runner(func (row, _c) {
      let (reg, q) = switch row {
        case "range_day_ord" (ro, qRange); case "eq_day_hash" (rh, qEq); case "group_day_hash" (rh, qGroup);
        case "group_day_ord" (ro, qGroup); case "range_day_none" (rn, qRange); case _ (rn, qGroup);
      };
      sink += Executor.runWith(reg, q, unrestricted).rows.size();
    });
    bench;
  };
};
