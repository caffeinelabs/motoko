module {
  public type List<T> = ?(T, List<T>);

  public func nil<T>() : List<T> = null;
  public func cons<T>(x : T, l : List<T>) : List<T> = ?(x, l)
}
