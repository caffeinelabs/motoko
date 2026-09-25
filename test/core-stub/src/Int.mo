/// Stub for Int.

import Types "Types";

module {
  public func compare(self : Int, y : Int) : Types.Order {
    if self < y { #less } else if self == y { #equal } else { #greater }
  };
}
