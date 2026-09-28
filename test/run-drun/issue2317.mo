actor {
  public func baz(b : Bool) : () {
    ignore({ b = b } : {})
  };
}
