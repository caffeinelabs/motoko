persistent actor Counter {

  var value = 0; // implicitly stable!

  public func inc() : async Nat {
    value += 1;
    value;
  };
}
