//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_1 --package native-table ../../lib/native-table

import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import Cells "mo:native-table/NativeTableCells";
import T "Types";

// Version 1: orders under OrderV1, filled with 40,000 rows in four calls
actor {
  let orders : T.Orders<T.OrderV1>;
  let few : T.Orders<T.OrderV1>;
  let names : T.Orders<Text>;

  public func fill() : async () {
    let start = orders.size();
    var i = start;
    while i < start + 10_000 {
      orders.add(i, T.v1(i));
      i += 1
    };
    if few.isEmpty() {
      var k = 0;
      while k < 1_000 {
        few.add(k, T.v1(k));
        k += 1
      };
      names.add(1, "Ada");
      names.add(2, "Grace")
    }
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show { version = 1; orders = orders.size(); few = few.size(); names = names.size() });
    Prim.debugPrint(debug_show (orders.get(25), orders.get(39_999)));
    Prim.debugPrint(debug_show (orders.sum("amount"), orders.min("amount"), orders.max("amount")));
    Prim.debugPrint(debug_show orders.countWhere("note", #eq(#null_)));
    Prim.debugPrint(debug_show T.take(orders.entriesWhere("city", #eq(#text "Bern")), 2));
    Prim.debugPrint(debug_show T.take(Cells.cells(orders, true, ["city", "note"], null), 3));
    Prim.debugPrint(debug_show (orders.count(["amount"], [#nat 7]), T.take(orders.indexEntries(["amount"], ?[#nat 998], null), 2)));
    Prim.debugPrint(debug_show T.take(orders.indexEntries(["city"], null, null), 2));
    Prim.debugPrint(debug_show T.take(orders.groupCounts(["city"]), 8));
    Prim.debugPrint(debug_show (orders.indexes, T.names(orders.fieldTypes())))
  }
}
