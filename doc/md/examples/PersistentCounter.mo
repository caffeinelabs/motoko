actor Counter {

  var value = 0; // persisted across upgrades (the default)

  public func inc() : async Nat {
    value += 1;
    value;
  };
}
