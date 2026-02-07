actor foo {
  public func foo() : () {};

  flexible func go() : async () {
    let bar = actor bar { public func bar() : () {} }
  };

}
