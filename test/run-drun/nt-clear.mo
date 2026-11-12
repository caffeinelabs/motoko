import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Emptying a table: after `clear` the table has no row, its index and its sums, counts and
// extremes start again from no rows, and rows written after it read back, while and after the
// memory the old rows used goes back through writes and `maintain`. A second clear before that
// memory is back, and a clear through a view, empty the table the same way. Rows removed one at a
// time give back the pages they left free in the same steps, and rows added after read back
actor {
  type Item = { name : Text; qty : Nat; note : Text };

  let items = NativeTable.empty<Nat, Item>().createIndex(["qty"]).build();
  let names : NativeTable.NativeTable<Nat, { name : Text }> = items.view();

  // A Text of 3,000 bytes: past what a row holds in one piece
  func long(c : Text) : Text {
    var t = "";
    var i = 0;
    while i < 3_000 { t #= c; i += 1 };
    t
  };

  func item(i : Nat) : Item = {
    name = "item " # debug_show i;
    qty = i % 7;
    note = if i % 500 == 0 { long("n") } else { "" };
  };

  func fill(n : Nat) {
    var i = 0;
    while i < n { items.add(i, item(i)); i += 1 };
    i := 0;
    while i < n { ignore items.update(i, { qty = 8 }); i += 3 };
    i := 0;
    while i < n { items.remove(i); i += 4 }
  };

  func show() {
    let first = do ? { items.entries().next()!.0 };
    let walked = do ? { items.indexEntries(["qty"], null, null).next()!.0 };
    Prim.debugPrint(debug_show (items.size(), items.isEmpty(), first, walked, items.count(["qty"], [#nat(8)])));
    Prim.debugPrint(debug_show (items.sum("qty"), items.min("qty"), items.max("qty"), items.get(5), names.get(6)))
  };

  public func put() : async () {
    fill(4_300);
    show()
  };

  public func clear() : async () {
    items.clear();
    show();
    var i = 0;
    while i < 10 { items.add(i, item(i)); i += 1 };
    show()
  };

  // Clears again before the first clear's memory is back, fills, then clears through a view
  public func again() : async () {
    items.clear();
    fill(5_000);
    show();
    names.clear();
    Prim.debugPrint(debug_show (items.maintain(0)));
    items.add(1, item(1));
    show()
  };

  // Maintenance gives the memory back in steps until no work is left; a budget of 0 runs no step
  // and tells whether work is pending
  public func upkeep() : async () {
    while items.maintain(100_000) {};
    Prim.debugPrint(debug_show (items.maintain(0)));
    show()
  };

  public func empty() : async () {
    fill(9_000);
    var i = 0;
    while i < 9_000 { items.remove(i); i += 1 };
    while items.maintain(100_000) {};
    show();
    fill(300);
    show()
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress clear "DIDL\x00\x00"
//CALL ingress again "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress empty "DIDL\x00\x00"
