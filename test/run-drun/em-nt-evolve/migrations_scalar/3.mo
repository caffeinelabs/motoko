import NativeTable "mo:native-table/NativeTable";
import T "../Types";

// Changes the scalar row of names to a record: traps
module {
  public func migration(old : { names : NativeTable.NativeTable<Nat, Text> }) : { names : NativeTable.NativeTable<Nat, T.Name> } {
    { names = old.names.evolve() }
  }
}
