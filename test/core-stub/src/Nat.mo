/// Stub for Nat.

import Types "Types";

module {
  public func compare(self : Nat, y : Nat) : Types.Order {
    if self < y { #less } else if self == y { #equal } else { #greater }
  };
}
