// this example triggers non-termination in subtyping due to non-regular type recursion in map<R>
module {

  public class Iter<T>({ next = n : () -> ?T }) = this {

    public let next = n;

    public func map<R>(f : T -> R) : Iter<R> {
      loop {}
    };
  }

}
