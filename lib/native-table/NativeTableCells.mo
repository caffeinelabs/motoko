/// Reads of a native table's rows as cells, for a query engine over native tables: each row
/// comes as the cells of the fields the reader names, and of its key when asked for, and the table
/// reads no other field and builds no row. Not part of `NativeTable`.

import Prim "mo:⛔";
import NativeTable "NativeTable";

module {

  /// The rows a call to the table hands back.
  let batch : Nat64 = 64;

  /// The rows of the table in the table's own order: every row, or with `filter` =
  /// ?(field, f) the rows whose field `field` meets `f`. Each row is an array of cells: its
  /// key's when `key`, then those of the fields `fields` names, in the order named, a field
  /// named twice twice. A Nat or Int stands for a value of any integer type or Char, `#null_`
  /// for an option that holds none. Every row that stays in the table while the iterator runs,
  /// and meets the filter, comes exactly once, also when other rows are written in between. The
  /// iterator fetches 64 rows per call to the table, the first of them at this call. Traps on a
  /// name the row does not have, and on a filter as `NativeTable.entriesWhere` does.
  public func cells<K, R>(self : NativeTable.NativeTable<K, R>, key : Bool, fields : [Text], filter : ?(Text, NativeTable.Filter)) : {
    next : () -> ?[NativeTable.Cell]
  } {
    let (first, after) = Prim.ntScanCells(self.id, self.codec, key, fields, filter, 0 : Nat64, batch);
    var rows = first;
    var rest = after;
    var at = 0;
    object {
      public func next() : ?[NativeTable.Cell] {
        loop {
          if at < rows.size() {
            at += 1;
            return ?rows[at - 1]
          };
          switch rest {
            case null { return null }
            case ?from {
              let (more, after) = Prim.ntScanCells(self.id, self.codec, key, fields, filter, from, batch);
              rows := more;
              at := 0;
              rest := after
            }
          }
        }
      }
    }
  }

}
