module L {
  public type List<T> = ?(T, List<T>);

  public func nil<T>() : List<T> = null;
  public func cons<T>(x : T, l : List<T>) : List<T> = ?(x, l);
  public func map<T, U>(f : T -> U, l : L.List<T>) : L.List<U> {
    switch l {
      case null { L.nil() }
      case ?(h, t) { L.cons(f h, map(f, t)) }
    }
  }
}
