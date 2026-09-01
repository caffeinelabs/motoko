/// Stub with a nested module, for nested implicit search tests.
/// The field name is unique so it cannot interfere with other tests.

module {
  public module Inner {
    public func nestedDouble(x : Nat) : Nat = x * 2;
  };
}
