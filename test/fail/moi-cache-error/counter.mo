module {
  public type Counter = { var count : Nat };
  public func make() : Counter = { var count = 0 };
  public func inc(c : Counter) { c.count += 1 };
  public func get(c : Counter) : Nat = c.count;
}
