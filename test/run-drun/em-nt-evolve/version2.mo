//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_2 --package native-table ../../lib/native-table

import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import Cells "mo:native-table/NativeTableCells";
import T "Types";

// Version 2: orders and few under OrderV2, and the value of orders from before the change
actor {
  let orders : T.Orders<T.OrderV2>;
  let few : T.Orders<T.OrderV2>;
  let names : T.Orders<Text>;
  let stale : T.Orders<T.OrderV1>;

  // The same writes into both tables: 50 new rows with coupon and tip, coupon and tip set on rows
  // 0 to 9 stored before, rows 10 to 19 removed
  public func write() : async () {
    for t in [orders, few].vals() {
      var j = 0;
      while j < 50 {
        assert t.insert(1_000_000 + j, T.v2(j));
        j += 1
      };
      var i = 0;
      while i < 10 {
        let ?o = t.get(i) else do { Prim.trap("missing row") };
        assert not t.insert(i, { o with coupon = ?("r" # debug_show i); tip = ?(i * 1_000) });
        assert t.delete(10 + i);
        i += 1
      }
    }
  };

  public func readStale() : async () {
    Prim.debugPrint(debug_show stale.size())
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show { version = 2; orders = orders.size(); few = few.size(); names = names.size() });
    // Stored under OrderV1, under OrderV1 and replaced under OrderV2, and under OrderV2
    Prim.debugPrint(debug_show (orders.get(25), orders.get(0), orders.get(1_000_001)));
    Prim.debugPrint(debug_show (do ? { orders.get(1_000_000)!.coupon!.size() }));
    Prim.debugPrint(debug_show (orders.sum("amount"), orders.sumCount("tip"), orders.min("tip"), orders.max("tip")));
    Prim.debugPrint(debug_show orders.countWhere("coupon", #eq(#null_)));
    Prim.debugPrint(debug_show T.take(orders.entriesWhere("city", #eq(#text "Bern")), 2));
    Prim.debugPrint(debug_show T.take(Cells.cells(orders, true, ["city", "coupon", "tip"], null), 3));
    Prim.debugPrint(debug_show (orders.count(["amount"], [#nat 7]), T.take(orders.indexEntries(["amount"], ?[#nat 998], null), 2)));
    Prim.debugPrint(debug_show T.take(orders.indexEntries(["city"], null, null), 2));
    Prim.debugPrint(debug_show T.take(orders.groupCounts(["city"]), 8));
    Prim.debugPrint(debug_show (orders.indexes, T.names(orders.fieldTypes())))
  }
}
