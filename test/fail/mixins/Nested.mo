import Counter "Counter";

mixin() {
  include Counter(0);
  public func decrement() : async () {
    await increment();
    counter -= 2;
  }
};
