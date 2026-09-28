let m = object {
  public module X {
    public type T = Int;
    public func bar() { m.X.bar(); };
    let _ = m.X.bar();
  }
};
