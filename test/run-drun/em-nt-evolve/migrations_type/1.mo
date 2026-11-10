import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Makes the tables: orders and few with indexes on city, amount and note, and names with a
// scalar row
module {
  public func migration(_ : {}) : {
    orders : T.Orders<T.OrderV1>;
    few : T.Orders<T.OrderV1>;
    names : NativeTable.NativeTable<Nat, Text>
  } {
    {
      orders = NativeTable.empty<Nat, T.OrderV1>().createIndex(["city"]).createIndex(["amount"]).createIndex(["note"]).build();
      few = NativeTable.empty<Nat, T.OrderV1>().createIndex(["city"]).createIndex(["amount"]).createIndex(["note"]).build();
      names = NativeTable.empty<Nat, Text>().build()
    }
  }
}
