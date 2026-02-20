//MOC-FLAG -A=M0194
actor foo {
  public func foo() : () {};

  flexible func go() : async () {
    let bar = actor bar { public func bar() : () {} }
  };

}
