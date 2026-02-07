let a = actor {
  public type T = Int;
  public func foo() : () { a.foo();  };
  let _ = a.foo();
};
