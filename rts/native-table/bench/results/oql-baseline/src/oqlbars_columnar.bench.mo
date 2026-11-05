/// shape O {id, customerId, amount = k mod 1000, paid = k even} without
/// any index, on the columnar OQL.Table ("col", flushed, so footers serve) and on
/// a plain core-Map entity ("heap"); plus footer min/max with and without a
/// deleted row, and a sum over a Table whose rows are all still buffered.
/// Query context: `mops bench oqlbars_columnar --query`.
import Bench    "mo:bench";
import Map      "mo:core/Map";
import Nat      "mo:core/Nat";
import OQL      "../src";
import Entity   "../src/Entity";
import Executor "../src/Executor";
import Query    "../src/Query";
import Registry "../src/Registry";
import Table    "../src/Table";
import _RecordValue "../src/RecordValue";
import _NatValue "../src/NatValue";
import _BoolValue "../src/BoolValue";

module {
  type Order = { id : Nat; customerId : Nat; amount : Nat; paid : Bool };
  func orderRow(o : Order) : OQL.Entity.Row = [("id", #nat(o.id)), ("customerId", #nat(o.customerId)), ("amount", #nat(o.amount)), ("paid", #bool(o.paid))];
  func mk(k : Nat) : Order = { id = k; customerId = k % 100; amount = k % 1_000; paid = (k % 2 == 0) };
  let COLS : [Table.Column] = [("customerId", #nat), ("amount", #nat), ("paid", #bool)];

  type Regs = { col : Registry.Registry; colDel : Registry.Registry; colBuf : Registry.Registry; heap : Registry.Registry };
  func build(n : Nat) : Regs {
    let t = Table.new(COLS, []);
    let td = Table.new(COLS, []);
    let tb = Table.newWith(COLS, [], [], 0);
    let m = Map.empty<Nat, Order>();
    var k = 0;
    while (k < n) { let o = mk(k); ignore Table.append(t, o, orderRow); ignore Table.append(td, o, orderRow); ignore Table.append(tb, o, orderRow); m.add(Nat.compare, k, o); k += 1 };
    Table.flush(t); Table.flush(td);
    Table.delete(td, 1);   // one deleted row in the first segment (not an extreme)
    {
      col = Registry.build([Table.entityWith(t, "order", "Order", "id").build()]);
      colDel = Registry.build([Table.entityWith(td, "order", "Order", "id").build()]);
      colBuf = Registry.build([Table.entityWith(tb, "order", "Order", "id").build()]);
      heap = Registry.build([Entity.new<Order>("order", func () = m.values(), "Order", "id", orderRow).build()]);
    }
  };

  let unrestricted : Executor.Access = func (_ : OQL.Decl) : OQL.Access = #unrestricted;
  func base() : Query.Query = { start = "order"; where_ = null; groupBy = []; aggregate = []; orderBy = []; offset = null; limit = null; select = null };

  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: shape O without an index: columnar Table and heap Map");
    bench.description("Footer min/max with and without a deleted row; no-index filter, count, group, top-10, range, multi; sum with every row buffered.");
    let qMin = { base() with aggregate = [{ fn = #min; field = ?["amount"]; as_ = null }] };
    let qMax = { base() with aggregate = [{ fn = #max; field = ?["amount"]; as_ = null }] };
    let qSum = { base() with aggregate = [{ fn = #sum; field = ?["amount"]; as_ = null }] };
    let qFilter = { base() with where_ = ?(#eq(["paid"], #bool(true))) };
    let qCount = { base() with where_ = ?(#eq(["paid"], #bool(true))); aggregate = [{ fn = #count; field = null; as_ = null }] };
    let qRange = { base() with where_ = ?(#ge(["amount"], #nat(500))) };
    let qTopN = { base() with orderBy = [{ field = ["amount"]; dir = #desc }]; limit = ?10 };
    let qGroup = { base() with groupBy = [["paid"]]; aggregate = [{ fn = #count; field = null; as_ = null }] };
    let qMulti = { base() with where_ = ?(#and_([#eq(["paid"], #bool(true)), #ge(["amount"], #nat(500))])) };
    let r1k = build(1_000); let r10k = build(10_000); let r40k = build(40_000); let r100k = build(100_000);
    func rFor(c : Text) : Regs = switch c { case "1000" r1k; case "10000" r10k; case "40000" r40k; case _ r100k };
    // answers agree between col and heap
    for (q in [qMin, qMax, qSum, qCount, qGroup].values()) {
      let a = Executor.runWith(r10k.col, q, unrestricted); let b = Executor.runWith(r10k.heap, q, unrestricted);
      assert a.rows.size() == b.rows.size();
    };
    var sink = 0;
    bench.rows([
      "min_col", "max_col", "min_col_del", "max_col_del",
      "sum_col_buffered", "sum_col",
      "filter_col", "filter_heap", "count_col", "count_heap",
      "range_col", "range_heap", "topn_col", "topn_heap",
      "group_col", "group_heap", "multi_col", "multi_heap",
    ]);
    bench.cols(["1000", "10000", "40000", "100000"]);
    bench.runner(func (row, c) {
      let r = rFor(c);
      let (reg, q) = switch row {
        case "min_col" (r.col, qMin); case "max_col" (r.col, qMax);
        case "min_col_del" (r.colDel, qMin); case "max_col_del" (r.colDel, qMax);
        case "sum_col_buffered" (r.colBuf, qSum); case "sum_col" (r.col, qSum);
        case "filter_col" (r.col, qFilter); case "filter_heap" (r.heap, qFilter);
        case "count_col" (r.col, qCount); case "count_heap" (r.heap, qCount);
        case "range_col" (r.col, qRange); case "range_heap" (r.heap, qRange);
        case "topn_col" (r.col, qTopN); case "topn_heap" (r.heap, qTopN);
        case "group_col" (r.col, qGroup); case "group_heap" (r.heap, qGroup);
        case "multi_col" (r.col, qMulti); case _ (r.heap, qMulti);
      };
      sink += Executor.runWith(reg, q, unrestricted).rows.size();
    });
    bench;
  };
};
