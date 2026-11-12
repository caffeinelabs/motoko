import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// The row types of the orders table in each version of the canister, and the rows the versions
// write.
module {
  public type OrderV1 = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; note : ?Text };

  // Drops note; adds coupon and tip
  public type OrderV2 = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; coupon : ?Text; tip : ?Nat };

  // Drops coupon, paid and placed; adds gift
  public type OrderV3 = { customer : Text; city : Text; amount : Nat; tip : ?Nat; gift : ?Text };

  // Changes the type of amount: refused
  public type OrderV2Int = { customer : Text; city : Text; amount : Int; paid : Bool; placed : Int; coupon : ?Text; tip : ?Nat };

  // Adds a field that is not an option: refused
  public type OrderV2Vip = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; coupon : ?Text; tip : ?Nat; vip : Bool };

  // A record in place of the scalar row of names: refused
  public type Name = { name : ?Text };

  public type Orders<R> = NativeTable.NativeTable<Nat, R>;

  public func city(i : Nat) : Text {
    switch i % 4 {
      case 0 { "Basel" }
      case 1 { "Bern" }
      case 2 { "Chur" }
      case _ { "Genf" }
    }
  };

  // The row of key i, stored under OrderV1
  public func v1(i : Nat) : OrderV1 {
    {
      customer = "c" # debug_show (i % 100);
      city = city(i);
      amount = i % 1_000;
      paid = i % 2 == 0;
      placed = i - 20_000;
      note = if i % 5 == 0 { null } else { ?("n" # debug_show (i % 7)) }
    }
  };

  // The row of key 1_000_000 + j, stored under OrderV2; the first has a coupon of 70,000 bytes
  public func v2(j : Nat) : OrderV2 {
    {
      customer = "new " # debug_show j;
      city = "Zug";
      amount = j;
      paid = true;
      placed = j;
      coupon = ?(if j == 0 { long() } else { "k" # debug_show j });
      tip = ?(100 + j)
    }
  };

  // The row of key 2_000_000 + j, stored under OrderV3
  public func v3(j : Nat) : OrderV3 {
    { customer = "gift " # debug_show j; city = "Zug"; amount = j; tip = null; gift = ?("g" # debug_show j) }
  };

  // A Text of 70,000 bytes, longer than a page of table memory
  public func long() : Text {
    var t = "";
    var i = 0;
    while i < 7_000 {
      t #= "0123456789";
      i += 1
    };
    t
  };

  // The first n values of an iterator
  public func take<T>(it : { next : () -> ?T }, n : Nat) : [T] {
    let out = Prim.Array_init<?T>(n, null);
    var k = 0;
    label l while k < n {
      switch it.next() {
        case ?v {
          out[k] := ?v;
          k += 1
        }
        case null { break l }
      }
    };
    Prim.Array_tabulate<T>(k, func i { switch out[i] { case ?v { v } case null { Prim.trap("take") } } })
  };

  // The names of a table's fields
  public func names(fields : [(Text, NativeTable.FieldType)]) : [Text] {
    Prim.Array_tabulate<Text>(fields.size(), func i { fields[i].0 })
  }
}
