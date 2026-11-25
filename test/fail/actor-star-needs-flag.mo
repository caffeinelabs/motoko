// Without --experimental-local-actors, `actor*` is a syntax error.
actor* class Counter() {
  public func get() : Nat { 0 };
};
