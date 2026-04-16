/// A module with a doc comment before imports.
import Nat "mo:core/Nat";
module {
  /// Triple a number.
  public func triple(x : Nat) : Nat = x * 3;
}
