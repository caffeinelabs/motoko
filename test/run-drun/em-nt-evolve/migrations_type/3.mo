import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Changes the type of amount from Nat to Int: traps
module {
  public func migration(old : { orders : T.Orders<T.OrderV2> }) : { orders : T.Orders<T.OrderV2Int> } {
    { orders = old.orders.evolve() }
  }
}
