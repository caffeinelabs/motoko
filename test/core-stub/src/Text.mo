/// Stub for Text.

import Types "Types";
import Prim "mo:⛔";

module {
  public type Text = Prim.Types.Text;

  public func compare(self : Text, t2 : Text) : Types.Order {
    let c = Prim.textCompare(self, t2);
    if c < 0 { #less } else if c == 0 { #equal } else { #greater }
  };
}
