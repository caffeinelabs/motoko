/// Stub with a nested module, for nested implicit and contextual dot search tests.
/// The field names are unique so they cannot interfere with other tests.

module {
  public module Inner {
    public func nestedDouble(x : Nat) : Nat = x * 2;
    public func nestedTriple(self : Nat) : Nat = self * 3;
  };
}
