/// Top-of-file module doc (old convention).
import Nat "mo:core/Nat";
/// Module doc after imports (new convention).
module {
  /// Square a number.
  public func square(x : Nat) : Nat = x * x;
}
