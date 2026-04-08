import Foo "./issue-1174/foo";
actor Example {
  private type Bar = Foo.foo;
  public func example() : () {};
}