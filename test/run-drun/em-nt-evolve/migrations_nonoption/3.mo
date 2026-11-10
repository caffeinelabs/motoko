import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Adds vip, a field that is not an option: traps
module {
  public func migration(old : { orders : T.Orders<T.OrderV2> }) : { orders : T.Orders<T.OrderV2Vip> } {
    { orders = old.orders.evolve() }
  }
}
