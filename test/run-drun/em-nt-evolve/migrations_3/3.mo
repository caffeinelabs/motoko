import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Changes the row type of orders and few again: drops coupon, paid and placed, adds gift. Both
// tables got the same writes since the last change, so both changes pack the same rows, and they
// cost the same to within 500 instructions, however many rows orders holds besides
module {
  public func migration(old : { orders : T.Orders<T.OrderV2>; few : T.Orders<T.OrderV2>; stale : T.Orders<T.OrderV1> }) : {
    orders : T.Orders<T.OrderV3>;
    few : T.Orders<T.OrderV3>
  } {
    let start = Prim.performanceCounter(0);
    let orders : T.Orders<T.OrderV3> = old.orders.evolve();
    let middle = Prim.performanceCounter(0);
    let few : T.Orders<T.OrderV3> = old.few.evolve();
    let end = Prim.performanceCounter(0);
    let (a, b) = (middle - start, end - middle);
    let difference = if a > b { a - b } else { b - a };
    Prim.debugPrint("evolve cost flat: " # debug_show (difference < 500));
    { orders; few }
  }
}
