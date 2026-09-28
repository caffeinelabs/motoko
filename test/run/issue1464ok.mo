import Prim "mo:⛔";
type Mean = ?Mean;
class Bar() = Self {
  public func mean(n:Nat) : Mean = if n == 0 { null } else { ?Self.mean(n - 1) };
};
Prim.debugPrint(debug_show Bar().mean(5));
