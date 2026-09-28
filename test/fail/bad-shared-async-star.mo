actor {

  public func bad()  : async* Nat {
    // reject (shared function can't return async*)
    666
  };

}
