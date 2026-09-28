actor {
  public type List<T> = ?(T, List<T>);
  public func f(i: List<Int>) : async List<Int> { i };
}
