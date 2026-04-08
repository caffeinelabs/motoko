import Nat "mo:core/Nat";

/// A module with a doc comment after imports.
module {
  /// Double a number.
  public func double(x : Nat) : Nat = x * 2;
}
