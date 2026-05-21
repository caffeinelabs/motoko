persistent actor Counter {

  var value = 0;

  public func inc() : async Nat {
    value += 1;
    return value;
  };

  public func reset() : async () {
    value := 0;
  }
}
