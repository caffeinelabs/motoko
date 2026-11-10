//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_3 --package native-table ../../lib/native-table

import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import Cells "mo:native-table/NativeTableCells";
import T "Types";

// Version 3: orders and few under OrderV3
actor {
  let orders : T.Orders<T.OrderV3>;
  let few : T.Orders<T.OrderV3>;
  let names : T.Orders<Text>;

  // The same 10 new rows with gift into both tables
  public func write() : async () {
    for t in [orders, few].vals() {
      var j = 0;
      while j < 10 {
        assert t.insert(2_000_000 + j, T.v3(j));
        j += 1
      }
    }
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show { version = 3; orders = orders.size(); few = few.size(); names = names.size() });
    // Stored under OrderV1, under OrderV2 (with the long coupon, now dropped), under OrderV1 and
    // replaced under OrderV2, and under OrderV3
    Prim.debugPrint(debug_show (orders.get(25), orders.get(1_000_000), orders.get(0), orders.get(2_000_001)));
    Prim.debugPrint(debug_show (orders.sum("amount"), orders.sumCount("tip"), orders.min("tip"), orders.max("tip")));
    Prim.debugPrint(debug_show orders.countWhere("gift", #eq(#null_)));
    Prim.debugPrint(debug_show T.take(orders.entriesWhere("city", #eq(#text "Bern")), 2));
    Prim.debugPrint(debug_show T.take(Cells.cells(orders, true, ["city", "gift", "tip"], null), 3));
    Prim.debugPrint(debug_show (orders.count(["amount"], [#nat 7]), T.take(orders.indexEntries(["amount"], ?[#nat 998], null), 2)));
    Prim.debugPrint(debug_show T.take(orders.indexEntries(["city"], null, null), 2));
    Prim.debugPrint(debug_show T.take(orders.groupCounts(["city"]), 8));
    Prim.debugPrint(debug_show (orders.indexes, T.names(orders.fieldTypes())))
  }
}
