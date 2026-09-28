module {
  public class Random(seed : Nat) {
    var number = seed;

    public func next() : Nat {
      number := (123138118391 * number + 133489131) % 9_999_999;
      number;
    };
  };
};
