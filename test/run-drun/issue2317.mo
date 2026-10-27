//MOC-FLAG -W=M0215
actor {
  public func baz(b : Bool) : () {
      ignore({b = b} : {})
  };
}
