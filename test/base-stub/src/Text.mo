/// Stub for Text.

import Types "Types";
import Prim "mo:⛔";

module {
  public func compare(t1 : Text, t2 : Text) : Types.Order {
    let c = Prim.textCompare(t1, t2);
    if c < 0 { #less } else if c == 0 { #equal } else { #greater };
  };
};
