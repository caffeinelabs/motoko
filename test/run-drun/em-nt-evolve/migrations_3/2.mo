import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Changes the row type of orders and few: drops note, adds coupon and tip. Keeps the value of
// orders from before the change as stale
module {
  public func migration(old : { orders : T.Orders<T.OrderV1>; few : T.Orders<T.OrderV1> }) : {
    orders : T.Orders<T.OrderV2>;
    few : T.Orders<T.OrderV2>;
    stale : T.Orders<T.OrderV1>
  } {
    { orders = old.orders.evolve(); few = old.few.evolve(); stale = old.orders }
  }
}
