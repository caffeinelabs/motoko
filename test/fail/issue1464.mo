class Foo(bar : { field : () }) {
  public func doFoo() = bar.field;
};

class Bar() = Self {
  public var foo : Foo = Foo(Self);
  public let field = ();
};

Bar().foo.doFoo();
