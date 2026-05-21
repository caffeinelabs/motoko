persistent actor Counter {

  var value = 0; // implicitly stable

  transient var invocations = 0; // reset on upgrade

  public func inc() : async Nat {
    value += 1;
    invocations += 1;
    value;
  };

  public func reset() : async () {
    value := 0;
  };

  public func getInvocations() : async Nat {
    invocations
  }

}
