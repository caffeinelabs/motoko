module X {
  public func f() { g() };
  func g() { f() };
  module Y {
    public type U = U -> U;
  };
  type T = Y.U; // ok private field eventhough Y.U private and unavoidable
  func h() : T { h() }; // ditto
};
