import Prim "mo:prim";
import Buffer "buffer";

module {
  public class Stack<T>() {
    let buffer = Buffer.Buffer<T>(0);

    public func isEmpty() : Bool {
      buffer.size() == 0;
    };

    public func push(value : T) {
      buffer.add(value);
    };

    public func pop() : T {
      switch buffer.removeLast() {
        case ?value { value }
        case _ { Prim.trap("empty") }
      };
    };
  };
};
