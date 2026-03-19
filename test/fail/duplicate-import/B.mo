module {
  public type T = Nat;
  public func f() : T = 1;

  public module Inner {
    public type T = Nat;
    public func f() : T = 1;
  }
}
